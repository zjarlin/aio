use std::{fs, path::Path};

use anyhow::{Context as _, Result};

pub fn write_schemas(output: &Path) -> Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("创建插件 Schema 目录失败: {}", output.display()))?;
    for document in az_plugin_manifest::schemas() {
        let path = output.join(document.file_name);
        let json = serde_json::to_string_pretty(&document.schema)?;
        fs::write(&path, format!("{json}\n"))
            .with_context(|| format!("写入插件 Schema 失败: {}", path.display()))?;
    }
    println!("已生成插件 Schema: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn writes_all_language_neutral_contracts() -> Result<()> {
        let root = tempdir()?;
        write_schemas(root.path())?;

        for file_name in [
            "repository-manifest.schema.json",
            "page-definitions.schema.json",
            "plugin-request.schema.json",
            "component-response.schema.json",
            "page-action-result.schema.json",
        ] {
            let value = serde_json::from_slice::<serde_json::Value>(&fs::read(
                root.path().join(file_name),
            )?)?;
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(
                value["$id"],
                format!(
                    "https://raw.githubusercontent.com/zjarlin/aio-platform/main/docs/plugin/schema/{file_name}"
                )
            );
        }
        Ok(())
    }
}
