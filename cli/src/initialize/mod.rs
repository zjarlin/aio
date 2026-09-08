pub(crate) mod template;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail, ensure};

pub struct ApplicationOptions {
    pub path: PathBuf,
    pub name: Option<String>,
    pub title: Option<String>,
}

pub struct RepositoryPluginOptions {
    pub path: PathBuf,
    pub name: Option<String>,
    pub title: Option<String>,
}

pub fn application(options: ApplicationOptions) -> Result<()> {
    let name = resolve_package_name(&options.path, options.name)?;
    let title = options.title.unwrap_or_else(|| name.clone());
    prepare_directory(&options.path)?;
    create_directory(&options.path.join("src/pages"))?;
    write(
        &options.path.join("Cargo.toml"),
        &template::application_cargo(&name),
    )?;
    write(
        &options.path.join("Dioxus.toml"),
        &template::dioxus_manifest(&name, &title),
    )?;
    write(&options.path.join("index.html"), template::INDEX_HTML)?;
    write(
        &options.path.join("rust-toolchain.toml"),
        template::RUST_TOOLCHAIN,
    )?;
    write(
        &options.path.join("aio.toml"),
        &template::aio_manifest(&name, &title),
    )?;
    write(&options.path.join(".gitignore"), template::GITIGNORE)?;
    write(
        &options.path.join("README.md"),
        &template::application_readme(&title),
    )?;
    write(
        &options.path.join("src/main.rs"),
        &template::application_main(&title),
    )?;
    write(
        &options.path.join("src/plugins.rs"),
        &template::plugins_source(&[], &[]),
    )?;
    write(
        &options.path.join("src/server.rs"),
        &template::application_server(&name),
    )?;
    write(&options.path.join("src/pages/mod.rs"), "pub mod home;\n")?;
    write(&options.path.join("src/pages/home.rs"), template::HOME_PAGE)?;
    write(
        &options.path.join("src/pages/README.md"),
        template::PAGES_README,
    )?;
    write(
        &options.path.join("Dockerfile"),
        &template::dockerfile(&name),
    )?;
    write(
        &options.path.join("compose.yaml"),
        template::COMPOSE_MANIFEST,
    )?;
    println!("已初始化应用: {}", options.path.display());
    Ok(())
}

pub fn repository_plugin(options: RepositoryPluginOptions) -> Result<()> {
    let name = resolve_package_name(&options.path, options.name)?;
    let title = options.title.unwrap_or_else(|| name.clone());
    let client_name = format!("{name}-client");
    let server_name = format!("{name}-server");
    prepare_directory(&options.path)?;
    create_directory(&options.path.join("client/src"))?;
    create_directory(&options.path.join("server/src"))?;
    write(
        &options.path.join("Cargo.toml"),
        &template::repository_workspace(&client_name, &server_name),
    )?;
    write(
        &options.path.join("client/Cargo.toml"),
        &template::repository_client_cargo(&client_name),
    )?;
    write(
        &options.path.join("server/Cargo.toml"),
        &template::repository_server_cargo(&server_name),
    )?;
    write(
        &options.path.join("rust-toolchain.toml"),
        template::RUST_TOOLCHAIN,
    )?;
    write(
        &options.path.join("aio-plugin.toml"),
        template::repository_manifest(),
    )?;
    write(&options.path.join(".gitignore"), "/target\n")?;
    write(
        &options.path.join("README.md"),
        &template::repository_plugin_readme(&title),
    )?;
    write(
        &options.path.join("client/README.md"),
        template::repository_client_readme(),
    )?;
    write(
        &options.path.join("client/src/lib.rs"),
        &template::repository_client_source(&title),
    )?;
    write(
        &options.path.join("server/README.md"),
        template::repository_server_readme(),
    )?;
    write(
        &options.path.join("server/src/lib.rs"),
        &template::repository_server_source(&name),
    )?;
    println!("已初始化全栈插件: {}", options.path.display());
    Ok(())
}

fn prepare_directory(path: &Path) -> Result<()> {
    if path.exists() {
        ensure!(path.is_dir(), "初始化路径不是目录: {}", path.display());
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("读取初始化目录失败: {}", path.display()))?;
        ensure!(
            entries.next().is_none(),
            "初始化目录必须为空: {}",
            path.display()
        );
        return Ok(());
    }
    create_directory(path)
}

fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("创建目录失败: {}", path.display()))
}

fn write(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).with_context(|| format!("写入文件失败: {}", path.display()))
}

fn resolve_package_name(path: &Path, configured: Option<String>) -> Result<String> {
    let name = configured.or_else(|| {
        path.file_name()
            .and_then(|value| value.to_str())
            .map(str::to_owned)
    });
    let Some(name) = name else {
        bail!("无法从目录推导包名，请传入 --name");
    };
    validate_package_name(&name)?;
    Ok(name)
}

fn validate_package_name(name: &str) -> Result<()> {
    let mut characters = name.chars();
    let valid_first = characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase());
    let valid_rest = characters.all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    });
    ensure!(
        valid_first && valid_rest,
        "包名必须以小写字母开头，且只包含小写字母、数字和连字符: {name}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn initializes_application_with_page_plugin_entry() -> Result<()> {
        let root = tempdir()?;
        let path = root.path().join("demo-app");

        application(ApplicationOptions {
            path: path.clone(),
            name: None,
            title: Some("演示应用".to_owned()),
        })?;

        assert!(path.join("aio.toml").is_file());
        assert!(path.join("index.html").is_file());
        assert!(path.join("rust-toolchain.toml").is_file());
        assert!(path.join("src/pages/home.rs").is_file());
        assert!(path.join("src/server.rs").is_file());
        assert!(path.join("Dockerfile").is_file());
        assert!(fs::read_to_string(path.join("src/plugins.rs"))?.contains("home::register"));
        Ok(())
    }

    #[test]
    fn initializes_repository_plugin_with_client_and_server() -> Result<()> {
        let root = tempdir()?;
        let path = root.path().join("hello-plugin");

        repository_plugin(RepositoryPluginOptions {
            path: path.clone(),
            name: None,
            title: Some("问候".to_owned()),
        })?;

        assert!(path.join("client/src/lib.rs").is_file());
        assert!(path.join("server/src/lib.rs").is_file());
        assert!(path.join("client/README.md").is_file());
        assert!(path.join("server/README.md").is_file());
        let manifest = fs::read_to_string(path.join("aio-plugin.toml"))?;
        assert!(manifest.contains("plugin.client"));
        assert!(manifest.contains("plugin.server"));
        Ok(())
    }
}
