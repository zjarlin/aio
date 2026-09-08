use std::{fs, path::Path};

use anyhow::{Context as _, Result, ensure};
pub use az_plugin_manifest::{PluginRuntime, RepositoryManifest, RepositoryPackage};
use serde::{Deserialize, Serialize};

pub const PROJECT_MANIFEST: &str = "aio.toml";
pub const LOCK_FILE: &str = ".aio/plugins.lock";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectManifest {
    pub application: ApplicationManifest,
    #[serde(default)]
    pub plugins: Vec<PluginSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApplicationManifest {
    pub name: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginSource {
    pub git: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PluginLock {
    #[serde(default)]
    pub plugins: Vec<InstalledPlugin>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstalledPlugin {
    pub git: String,
    pub revision: String,
    pub directory: String,
    pub client: Option<InstalledPackage>,
    pub server: Option<InstalledPackage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstalledPackage {
    pub package: String,
    pub dependency: String,
    pub package_path: String,
}

pub fn read_project(root: &Path) -> Result<ProjectManifest> {
    read_toml(&root.join(PROJECT_MANIFEST))
}

pub fn read_lock(root: &Path) -> Result<PluginLock> {
    let path = root.join(LOCK_FILE);
    if !path.exists() {
        return Ok(PluginLock::default());
    }
    read_toml(&path)
}

pub fn read_repository(root: &Path) -> Result<RepositoryManifest> {
    az_plugin_manifest::read_manifest(root)
}

pub fn read_package_name(package_root: &Path) -> Result<String> {
    let path = package_root.join("Cargo.toml");
    let document = read_toml_value(&path)?;
    document
        .get("package")
        .and_then(|value| value.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("插件 Cargo 包缺少 package.name: {}", path.display()))
}

pub fn write_project(root: &Path, manifest: &ProjectManifest) -> Result<()> {
    write_toml(&root.join(PROJECT_MANIFEST), manifest)
}

pub fn write_lock(root: &Path, lock: &PluginLock) -> Result<()> {
    write_toml(&root.join(LOCK_FILE), lock)
}

fn read_toml<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        fs::read_to_string(path).with_context(|| format!("读取 TOML 失败: {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("解析 TOML 失败: {}", path.display()))
}

fn read_toml_value(path: &Path) -> Result<toml::Value> {
    read_toml(path)
}

fn write_toml<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(value).context("序列化 TOML 失败")?;
    fs::write(path, text).with_context(|| format!("写入 TOML 失败: {}", path.display()))
}

pub fn validate_relative_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    ensure!(
        !path.is_absolute(),
        "插件包路径不能是绝对路径: {}",
        path.display()
    );
    ensure!(
        path.components().all(|component| matches!(
            component,
            std::path::Component::CurDir | std::path::Component::Normal(_)
        )),
        "插件包路径不能离开仓库: {}",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_online_runtime_without_treating_it_as_rust_source() -> Result<()> {
        let manifest = toml::from_str::<RepositoryManifest>(
            "[plugin.runtime]\nkind = \"wasm-component\"\nartifact = \"dist/client.wasm\"\n",
        )?;

        let runtime = manifest.plugin.runtime.context("缺少 runtime")?;
        assert_eq!(runtime.kind, PluginRuntime::WasmComponent);
        Ok(())
    }
}
