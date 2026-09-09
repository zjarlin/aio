use std::{collections::HashSet, fs, path::Path};

use anyhow::{Context as _, Result, ensure};
use az_plugin_manifest::{CapabilityManifest, PluginRuntime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MarketplaceEntry {
    git: String,
    rev: String,
    title: String,
    summary: String,
    license: String,
    tags: Vec<String>,
    runtime: PluginRuntime,
    capabilities: CapabilityManifest,
}

pub fn build(registry: &Path, output: &Path) -> Result<()> {
    ensure!(registry.is_dir(), "市场目录不存在: {}", registry.display());
    let mut paths = fs::read_dir(registry)
        .with_context(|| format!("读取市场目录失败: {}", registry.display()))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();

    let mut sources = HashSet::new();
    let mut entries = Vec::new();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取市场条目失败: {}", path.display()))?;
        let entry = toml::from_str::<MarketplaceEntry>(&text)
            .with_context(|| format!("解析市场条目失败: {}", path.display()))?;
        validate_entry(&entry, &path)?;
        ensure!(
            sources.insert(entry.git.clone()),
            "市场包含重复 Git 来源: {}",
            entry.git
        );
        entries.push(entry);
    }
    ensure!(!entries.is_empty(), "市场至少需要一个插件条目");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建市场索引目录失败: {}", parent.display()))?;
    }
    let mut json = serde_json::to_string_pretty(&entries).context("序列化市场索引失败")?;
    json.push('\n');
    let temporary = output.with_extension("json.next");
    fs::write(&temporary, json)
        .with_context(|| format!("写入市场临时索引失败: {}", temporary.display()))?;
    fs::rename(&temporary, output)
        .with_context(|| format!("发布市场索引失败: {}", output.display()))?;
    println!("已生成市场索引: {} 个插件", entries.len());
    Ok(())
}

fn validate_entry(entry: &MarketplaceEntry, path: &Path) -> Result<()> {
    ensure!(
        entry.git.starts_with("https://") && entry.git.ends_with(".git"),
        "市场 Git 来源必须是公开 HTTPS 仓库: {}",
        path.display()
    );
    ensure!(
        entry.rev.len() == 40 && entry.rev.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "市场 revision 必须是 40 位提交 SHA: {}",
        path.display()
    );
    for (field, value) in [
        ("title", entry.title.as_str()),
        ("summary", entry.summary.as_str()),
        ("license", entry.license.as_str()),
    ] {
        ensure!(
            !value.trim().is_empty(),
            "市场字段 {field} 不能为空: {}",
            path.display()
        );
    }
    ensure!(
        !entry.tags.is_empty(),
        "市场 tags 不能为空: {}",
        path.display()
    );
    ensure!(
        entry.runtime != PluginRuntime::RustSource,
        "市场条目不能直接发布 rust-source 插件: {}",
        path.display()
    );
    ensure!(
        entry
            .capabilities
            .network
            .iter()
            .all(|value| !value.trim().is_empty())
            && entry
                .capabilities
                .filesystem
                .iter()
                .all(|value| !value.trim().is_empty()),
        "市场能力声明不能包含空值: {}",
        path.display()
    );
    let mut tags = HashSet::new();
    for tag in &entry.tags {
        ensure!(
            !tag.trim().is_empty() && tags.insert(tag),
            "市场 tag 不能为空或重复: {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn builds_sorted_marketplace_index() -> Result<()> {
        let root = tempdir()?;
        let registry = root.path().join("registry");
        let output = root.path().join("index.json");
        fs::create_dir_all(&registry)?;
        fs::write(
            registry.join("hello.toml"),
            "git = \"https://example.com/hello.git\"\nrev = \"0123456789012345678901234567890123456789\"\ntitle = \"Hello\"\nsummary = \"Example\"\nlicense = \"MIT\"\ntags = [\"example\"]\nruntime = \"page-definition\"\n[capabilities]\nnetwork = []\nfilesystem = []\ndatabase = false\n",
        )?;

        build(&registry, &output)?;

        let entries = serde_json::from_str::<Vec<MarketplaceEntry>>(&fs::read_to_string(output)?)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Hello");
        Ok(())
    }
}
