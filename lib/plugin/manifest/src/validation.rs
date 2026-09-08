use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::{Component as PathComponent, Path, PathBuf},
};

use anyhow::{Context as _, Result, bail, ensure};
use wit_component::{DecodedWasm, decode};
use wit_parser::{Function, FunctionKind, Type, WorldItem, WorldKey};

use crate::{PageBody, PageDefinition, PluginRuntime, RepositoryManifest, RepositoryPackage};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    pub runtime: PluginRuntime,
    pub artifact: Option<PathBuf>,
    pub page_count: usize,
    pub subplugin_count: usize,
}

pub fn read_manifest(root: &Path) -> Result<RepositoryManifest> {
    let path = root.join("aio-plugin.toml");
    let source = fs::read_to_string(&path)
        .with_context(|| format!("读取插件清单失败: {}", path.display()))?;
    parse_manifest(&source)
}

pub fn parse_manifest(source: &str) -> Result<RepositoryManifest> {
    let manifest = toml::from_str::<RepositoryManifest>(source).context("解析插件清单失败")?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest(manifest: &RepositoryManifest) -> Result<()> {
    let plugin = &manifest.plugin;
    ensure!(
        plugin.runtime.is_some() || plugin.client.is_some() || plugin.server.is_some(),
        "插件必须声明 runtime、client 或 server 能力"
    );
    for package in [plugin.client.as_ref(), plugin.server.as_ref()]
        .into_iter()
        .flatten()
    {
        validate_source_package(package)?;
    }
    if let Some(runtime) = &plugin.runtime {
        ensure!(
            runtime.kind != PluginRuntime::RustSource,
            "rust-source 必须通过 plugin.client 或 plugin.server 声明"
        );
        validate_relative_path(&runtime.artifact, "插件 artifact")?;
        if let Some(requirement) = &runtime.host_version {
            validate_name(requirement, "宿主版本约束")?;
            semver::VersionReq::parse(requirement)
                .with_context(|| format!("宿主版本约束无效: {requirement}"))?;
        }
        validate_runtime_options(runtime)?;
    }
    validate_capabilities(&plugin.capabilities.network, "网络能力")?;
    validate_capabilities(&plugin.capabilities.filesystem, "文件系统能力")?;
    validate_subplugins(manifest)
}

pub fn validate_host_compatibility(
    manifest: &RepositoryManifest,
    host_version: &str,
) -> Result<()> {
    let host_version = semver::Version::parse(host_version)
        .with_context(|| format!("宿主版本无效: {host_version}"))?;
    let Some(requirement) = manifest
        .plugin
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.host_version.as_deref())
    else {
        return Ok(());
    };
    let requirement = semver::VersionReq::parse(requirement)
        .with_context(|| format!("宿主版本约束无效: {requirement}"))?;
    ensure!(
        requirement.matches(&host_version),
        "插件需要宿主版本 {requirement}，当前为 {host_version}"
    );
    Ok(())
}

fn validate_runtime_options(runtime: &crate::RuntimeManifest) -> Result<()> {
    if runtime.kind == PluginRuntime::Process {
        let image = runtime
            .container_image
            .as_deref()
            .context("process 插件必须声明 container_image")?;
        validate_container_image(image)?;
        ensure!(
            !runtime.entrypoint.is_empty(),
            "process 插件必须声明 entrypoint"
        );
        for argument in &runtime.entrypoint {
            validate_name(argument, "process 启动参数")?;
            ensure!(!argument.contains('\0'), "process 启动参数不能包含 NUL");
        }
        ensure!(
            runtime
                .entrypoint
                .iter()
                .filter(|argument| argument.as_str() == "{artifact}")
                .count()
                == 1,
            "process entrypoint 必须且只能引用一次 {{artifact}}"
        );
        let health_check = runtime
            .health_check
            .as_deref()
            .context("process 插件必须声明 health_check")?;
        ensure!(
            health_check.starts_with('/')
                && !health_check.contains('?')
                && !health_check.contains('#'),
            "process health_check 必须是不含查询或片段的绝对路径"
        );
        if let Some(timeout) = runtime.shutdown_timeout_seconds {
            ensure!(
                (1..=60).contains(&timeout),
                "process shutdown_timeout_seconds 必须在 1..=60"
            );
        }
        return Ok(());
    }
    ensure!(
        runtime.container_image.is_none()
            && runtime.entrypoint.is_empty()
            && runtime.health_check.is_none()
            && runtime.shutdown_timeout_seconds.is_none(),
        "container_image、entrypoint、health_check 和 shutdown_timeout_seconds 只能用于 process 插件"
    );
    Ok(())
}

fn validate_container_image(image: &str) -> Result<()> {
    validate_name(image, "process 容器镜像")?;
    let Some((repository, digest)) = image.rsplit_once("@sha256:") else {
        bail!("process container_image 必须锁定 sha256 digest");
    };
    ensure!(
        !repository.is_empty()
            && !repository.contains(char::is_whitespace)
            && digest.len() == 64
            && digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "process container_image 必须是有效的镜像@sha256:digest"
    );
    Ok(())
}

pub fn validate_repository(root: &Path) -> Result<ValidationReport> {
    let manifest = read_manifest(root)?;
    for package in [
        manifest.plugin.client.as_ref(),
        manifest.plugin.server.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        source_package_path(root, package)?;
    }
    let Some(runtime) = &manifest.plugin.runtime else {
        return Ok(ValidationReport {
            runtime: PluginRuntime::RustSource,
            artifact: None,
            page_count: 0,
            subplugin_count: manifest.plugin.subplugins.len(),
        });
    };
    let artifact = artifact_path(root, &runtime.artifact)?;
    let page_count = match runtime.kind {
        PluginRuntime::PageDefinition => {
            let pages =
                serde_json::from_slice::<Vec<PageDefinition>>(&fs::read(&artifact).with_context(
                    || format!("读取 PageDefinition 产物失败: {}", artifact.display()),
                )?)
                .context("解析 PageDefinition 产物失败")?;
            validate_page_definitions(&pages)?;
            validate_declared_pages(&manifest, &pages)?;
            pages.len()
        }
        PluginRuntime::WasmComponent => {
            validate_wasm_component(&fs::read(&artifact).with_context(|| {
                format!("读取 Wasm Component 产物失败: {}", artifact.display())
            })?)?;
            declared_pages(&manifest).len()
        }
        PluginRuntime::Process => 0,
        PluginRuntime::RustSource => bail!("rust-source 不使用 plugin.runtime"),
    };
    Ok(ValidationReport {
        runtime: runtime.kind,
        artifact: Some(artifact),
        page_count,
        subplugin_count: manifest.plugin.subplugins.len(),
    })
}

fn source_package_path(root: &Path, package: &RepositoryPackage) -> Result<PathBuf> {
    let root = root
        .canonicalize()
        .with_context(|| format!("解析插件仓库路径失败: {}", root.display()))?;
    let package_root = root.join(&package.path);
    ensure!(
        package_root.is_dir(),
        "Rust 插件包目录不存在: {}",
        package.path
    );
    let package_root = package_root
        .canonicalize()
        .with_context(|| format!("解析 Rust 插件包路径失败: {}", package_root.display()))?;
    ensure!(
        package_root.starts_with(&root),
        "Rust 插件包不能通过符号链接离开仓库: {}",
        package.path
    );
    ensure!(
        package_root.join("Cargo.toml").is_file(),
        "Rust 插件包缺少 Cargo.toml: {}",
        package.path
    );
    Ok(package_root)
}

pub fn artifact_path(root: &Path, relative: &str) -> Result<PathBuf> {
    validate_relative_path(relative, "插件 artifact")?;
    let root = root
        .canonicalize()
        .with_context(|| format!("解析插件仓库路径失败: {}", root.display()))?;
    let artifact = root.join(relative);
    ensure!(artifact.is_file(), "插件 artifact 不存在: {relative}");
    let artifact = artifact
        .canonicalize()
        .with_context(|| format!("解析插件 artifact 路径失败: {}", artifact.display()))?;
    ensure!(
        artifact.starts_with(&root),
        "插件 artifact 不能通过符号链接离开仓库: {relative}"
    );
    Ok(artifact)
}

pub fn validate_page_definitions(pages: &[PageDefinition]) -> Result<()> {
    ensure!(!pages.is_empty(), "插件至少需要贡献一个页面");
    let mut ids = HashSet::new();
    for page in pages {
        validate_name(&page.id, "页面 id")?;
        validate_name(&page.label, "页面标题")?;
        validate_name(&page.scene.id, "页面场景 id")?;
        validate_name(&page.scene.label, "页面场景标题")?;
        ensure!(
            ids.insert(page.id.as_str()),
            "插件页面 id 重复: {}",
            page.id
        );
        if let Some(icon) = &page.icon {
            validate_name(icon, "页面图标")?;
        }
        if let Some(permission) = &page.required_permission {
            validate_name(permission, "页面权限")?;
        }
        match &page.body {
            PageBody::Counter { title, button } => {
                validate_name(title, "计数页标题")?;
                validate_name(button, "计数页按钮")?;
            }
            PageBody::Text { title, content } => {
                validate_name(title, "文本页标题")?;
                validate_name(content, "文本页内容")?;
            }
        }
    }
    Ok(())
}

pub fn validate_wasm_component(bytes: &[u8]) -> Result<()> {
    let decoded = decode(bytes).context("解析 Wasm Component WIT 失败")?;
    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => bail!("artifact 是 WIT package，不是 Wasm Component"),
    };
    let world = &resolve.worlds[world_id];
    ensure!(
        world.imports.is_empty(),
        "Wasm Component 包含未授权导入: {}",
        world.imports.len()
    );
    ensure!(
        world.exports.len() == 2,
        "Wasm Component 必须仅导出 definition 和 handle"
    );
    let definition = exported_function(
        world.exports.get(&WorldKey::Name("definition".to_owned())),
        "definition",
    )?;
    ensure!(definition.params.is_empty(), "definition 不能声明参数");
    ensure!(
        definition.result.as_ref() == Some(&Type::String),
        "definition 必须返回 string"
    );
    let handle = exported_function(
        world.exports.get(&WorldKey::Name("handle".to_owned())),
        "handle",
    )?;
    ensure!(
        handle.params.len() == 1 && handle.params[0].1 == Type::String,
        "handle 必须接受一个 string 参数"
    );
    ensure!(
        handle.result.as_ref() == Some(&Type::String),
        "handle 必须返回 string"
    );
    Ok(())
}

fn exported_function<'a>(item: Option<&'a WorldItem>, name: &str) -> Result<&'a Function> {
    let Some(WorldItem::Function(function)) = item else {
        bail!("Wasm Component 缺少 {name} 函数导出");
    };
    ensure!(
        function.kind == FunctionKind::Freestanding,
        "Wasm Component {name} 必须是同步独立函数"
    );
    Ok(function)
}

fn validate_source_package(package: &RepositoryPackage) -> Result<()> {
    ensure!(
        package.runtime == PluginRuntime::RustSource,
        "plugin.client 和 plugin.server 只能使用 rust-source"
    );
    validate_relative_package_path(&package.path)
}

fn validate_relative_package_path(relative: &str) -> Result<()> {
    ensure!(!relative.trim().is_empty(), "Rust 插件包路径不能为空");
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute(),
        "Rust 插件包路径不能是绝对路径: {relative}"
    );
    ensure!(
        path.components()
            .all(|component| matches!(component, PathComponent::CurDir | PathComponent::Normal(_))),
        "Rust 插件包路径不能离开仓库: {relative}"
    );
    Ok(())
}

fn validate_relative_path(relative: &str, label: &str) -> Result<()> {
    ensure!(!relative.trim().is_empty(), "{label} 不能为空");
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, PathComponent::Normal(_))),
        "{label} 必须是仓库内相对路径: {relative}"
    );
    Ok(())
}

fn validate_capabilities(values: &[String], label: &str) -> Result<()> {
    let mut unique = HashSet::new();
    for value in values {
        validate_name(value, label)?;
        ensure!(unique.insert(value.as_str()), "{label}重复: {value}");
    }
    Ok(())
}

fn validate_subplugins(manifest: &RepositoryManifest) -> Result<()> {
    let plugins = manifest
        .plugin
        .subplugins
        .iter()
        .map(|plugin| (plugin.id.as_str(), plugin))
        .collect::<HashMap<_, _>>();
    ensure!(
        plugins.len() == manifest.plugin.subplugins.len(),
        "子插件 id 重复"
    );
    let mut pages = HashSet::new();
    let mut routes = HashSet::new();
    let mut account_actions = HashSet::new();
    for plugin in &manifest.plugin.subplugins {
        validate_name(&plugin.id, "子插件 id")?;
        validate_contributions(&plugin.pages, "页面", &mut pages)?;
        validate_contributions(&plugin.routes, "路由", &mut routes)?;
        validate_contributions(&plugin.account_actions, "账户动作", &mut account_actions)?;
        let mut dependencies = HashSet::new();
        for dependency in &plugin.dependencies {
            validate_name(dependency, "子插件依赖")?;
            ensure!(
                dependencies.insert(dependency.as_str()),
                "子插件 {} 重复依赖: {dependency}",
                plugin.id
            );
            ensure!(
                plugins.contains_key(dependency.as_str()),
                "子插件 {} 依赖不存在: {dependency}",
                plugin.id
            );
        }
    }
    let mut states = HashMap::new();
    for id in plugins.keys() {
        visit(id, &plugins, &mut states)?;
    }
    Ok(())
}

fn validate_contributions<'a>(
    values: &'a [String],
    label: &str,
    unique: &mut HashSet<&'a str>,
) -> Result<()> {
    for value in values {
        validate_name(value, label)?;
        ensure!(
            unique.insert(value.as_str()),
            "子插件{label}声明重复: {value}"
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Visited,
}

fn visit<'a>(
    id: &'a str,
    plugins: &HashMap<&'a str, &'a crate::SubpluginManifest>,
    states: &mut HashMap<&'a str, VisitState>,
) -> Result<()> {
    match states.get(id) {
        Some(VisitState::Visiting) => bail!("子插件依赖存在循环: {id}"),
        Some(VisitState::Visited) => return Ok(()),
        None => {}
    }
    states.insert(id, VisitState::Visiting);
    for dependency in &plugins[id].dependencies {
        visit(dependency, plugins, states)?;
    }
    states.insert(id, VisitState::Visited);
    Ok(())
}

fn declared_pages(manifest: &RepositoryManifest) -> BTreeSet<&str> {
    manifest
        .plugin
        .subplugins
        .iter()
        .flat_map(|plugin| plugin.pages.iter().map(String::as_str))
        .collect()
}

pub fn validate_declared_pages(
    manifest: &RepositoryManifest,
    pages: &[PageDefinition],
) -> Result<()> {
    let declared = declared_pages(manifest);
    let actual = pages
        .iter()
        .map(|page| page.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        declared == actual,
        "子插件页面声明与 PageDefinition 不一致: 声明={declared:?}, 产物={actual:?}"
    );
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} 不能为空");
    ensure!(value.trim() == value, "{label} 不能包含首尾空白: {value:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use tempfile::{NamedTempFile, tempdir};

    use super::*;

    const RUNTIME: &str = r#"
[plugin.runtime]
kind = "page-definition"
artifact = "dist/pages.json"
"#;

    #[test]
    fn accepts_parent_with_ordered_subplugins() -> Result<()> {
        let manifest = parse_manifest(&format!(
            "{RUNTIME}\n[[plugin.subplugins]]\nid='profile'\npages=['profile']\n[[plugin.subplugins]]\nid='account'\ndependencies=['profile']\naccount_actions=['profile']"
        ))?;
        assert_eq!(manifest.plugin.subplugins.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_missing_dependency_cycle_and_duplicate_contribution() {
        let missing = parse_manifest(&format!(
            "{RUNTIME}\n[[plugin.subplugins]]\nid='account'\ndependencies=['missing']"
        ))
        .expect_err("缺失依赖必须失败");
        assert!(missing.to_string().contains("依赖不存在"));

        let cycle = parse_manifest(&format!(
            "{RUNTIME}\n[[plugin.subplugins]]\nid='a'\ndependencies=['b']\n[[plugin.subplugins]]\nid='b'\ndependencies=['a']"
        ))
        .expect_err("循环依赖必须失败");
        assert!(cycle.to_string().contains("循环"));

        let duplicate = parse_manifest(&format!(
            "{RUNTIME}\n[[plugin.subplugins]]\nid='a'\nroutes=['echo']\n[[plugin.subplugins]]\nid='b'\nroutes=['echo']"
        ))
        .expect_err("重复路由必须失败");
        assert!(duplicate.to_string().contains("路由声明重复"));
    }

    #[test]
    fn rejects_blank_page_fields() {
        let pages = [PageDefinition {
            id: "page".to_owned(),
            label: " ".to_owned(),
            icon: None,
            scene: crate::SceneDefinition {
                id: "scene".to_owned(),
                label: "Scene".to_owned(),
            },
            required_permission: None,
            body: PageBody::Text {
                title: "Title".to_owned(),
                content: "Content".to_owned(),
            },
        }];
        let error = validate_page_definitions(&pages).expect_err("空标题必须失败");
        assert!(error.to_string().contains("页面标题"));
    }

    #[test]
    fn rejects_non_component_artifact() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        file.write_all(b"not wasm")?;
        let bytes = fs::read(file.path())?;
        assert!(validate_wasm_component(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn validates_page_artifact_against_subplugin_declarations() -> Result<()> {
        let repository = tempdir()?;
        fs::create_dir(repository.path().join("dist"))?;
        fs::write(
            repository.path().join("aio-plugin.toml"),
            format!("{RUNTIME}\n[[plugin.subplugins]]\nid='counter'\npages=['counter']"),
        )?;
        fs::write(
            repository.path().join("dist/pages.json"),
            r#"[{"id":"counter","label":"Counter","icon":null,"scene":{"id":"examples","label":"Examples"},"required_permission":null,"body":{"kind":"counter","title":"Counter","button":"+1"}}]"#,
        )?;

        let report = validate_repository(repository.path())?;
        assert_eq!(report.runtime, PluginRuntime::PageDefinition);
        assert_eq!(report.page_count, 1);

        fs::write(
            repository.path().join("aio-plugin.toml"),
            format!("{RUNTIME}\n[[plugin.subplugins]]\nid='other'\npages=['other']"),
        )?;
        let error = validate_repository(repository.path())
            .expect_err("清单与 PageDefinition 不一致必须失败");
        assert!(error.to_string().contains("不一致"));
        Ok(())
    }

    #[test]
    fn requires_explicit_process_lifecycle_contract() -> Result<()> {
        let missing =
            parse_manifest("[plugin.runtime]\nkind='process'\nartifact='dist/plugin.jar'\n")
                .expect_err("缺少进程启动契约必须失败");
        assert!(missing.to_string().contains("container_image"));

        parse_manifest(
            "[plugin.runtime]\nkind='process'\nartifact='dist/plugin.jar'\nhost_version='>=2026.5.10'\ncontainer_image='eclipse-temurin:21-jre@sha256:5c67d24ee8e3dd810b2a0cb6c3827ced2ac5d22729538f90b36c2b9d77678bb8'\nentrypoint=['java','-jar','{artifact}']\nhealth_check='/health'\nshutdown_timeout_seconds=10\n",
        )?;
        let version = parse_manifest(
            "[plugin.runtime]\nkind='process'\nartifact='dist/plugin.js'\nhost_version='soon'\ncontainer_image='node:22@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3'\nentrypoint=['node','{artifact}']\nhealth_check='/health'\n",
        )
        .expect_err("无效版本约束必须失败");
        assert!(version.to_string().contains("版本约束"));
        Ok(())
    }

    #[test]
    fn rejects_floating_process_image_and_undeclared_entrypoint() {
        let floating = parse_manifest(
            "[plugin.runtime]\nkind='process'\nartifact='dist/plugin.js'\ncontainer_image='node:22'\nentrypoint=['node','{artifact}']\nhealth_check='/health'\n",
        )
        .expect_err("浮动镜像必须失败");
        assert!(floating.to_string().contains("sha256"));

        let undeclared = parse_manifest(
            "[plugin.runtime]\nkind='process'\nartifact='dist/plugin.js'\ncontainer_image='node:22@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3'\nentrypoint=['node','src/server.js']\nhealth_check='/health'\n",
        )
        .expect_err("未引用 artifact 的启动命令必须失败");
        assert!(undeclared.to_string().contains("{artifact}"));
    }

    #[test]
    fn enforces_host_version_requirement() -> Result<()> {
        let manifest = parse_manifest(
            "[plugin.runtime]\nkind='page-definition'\nartifact='dist/pages.json'\nhost_version='>=2026.5.10, <2027.0.0'\n",
        )?;
        validate_host_compatibility(&manifest, "2026.9.8")?;
        let error = validate_host_compatibility(&manifest, "2027.1.0")
            .expect_err("不匹配的宿主版本必须失败");
        assert!(error.to_string().contains("插件需要宿主版本"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_artifact_symlink_that_leaves_repository() -> Result<()> {
        use std::os::unix::fs::symlink;

        let repository = tempdir()?;
        fs::create_dir(repository.path().join("dist"))?;
        let external = NamedTempFile::new()?;
        symlink(external.path(), repository.path().join("dist/plugin.wasm"))?;
        let error = artifact_path(repository.path(), "dist/plugin.wasm")
            .expect_err("离开仓库的符号链接必须失败");
        assert!(error.to_string().contains("符号链接"));
        Ok(())
    }
}
