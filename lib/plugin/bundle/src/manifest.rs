use anyhow::{Result, ensure};
use az_plugin_contract::CapabilityGrants;
use serde::{Deserialize, Serialize};

use crate::validate_relative_path;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    pub schema_version: u32,
    pub plugin: ComponentManifest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentManifest {
    #[serde(default)]
    pub permissions: Vec<String>,
    pub marketplace: Option<MarketplaceManifest>,
    pub runtime: RuntimeManifest,
    pub frontend: FrontendManifest,
    pub database: Option<DatabaseManifest>,
    #[serde(default)]
    pub capabilities: CapabilityGrants,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketplaceManifest {
    pub title: String,
    pub summary: String,
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeManifest {
    pub artifact: String,
    pub host_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendManifest {
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseManifest {
    pub migrations: String,
}

impl BundleManifest {
    pub fn parse(text: &str) -> Result<Self> {
        ensure!(
            text.len() <= crate::model::MAX_MANIFEST_BYTES,
            "清单超过配额"
        );
        let manifest: Self = toml::from_str(text)?;
        ensure!(
            manifest.schema_version == az_plugin_contract::ABI_VERSION,
            "只接受 v2 清单"
        );
        let plugin = &manifest.plugin;
        ensure!(
            plugin.permissions.len() <= 64
                && plugin.permissions.iter().all(|p| !p.is_empty()
                    && p.len() <= 128
                    && p != "*"
                    && p.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))),
            "插件权限声明无效"
        );
        if let Some(marketplace) = &plugin.marketplace {
            ensure!(
                !marketplace.title.trim().is_empty() && marketplace.title.len() <= 256,
                "市场标题无效"
            );
            ensure!(
                marketplace.summary.len() <= 2048
                    && !marketplace.license.is_empty()
                    && marketplace.license.len() <= 128,
                "市场简介或许可证无效"
            );
            ensure!(
                marketplace.tags.len() <= 16
                    && marketplace
                        .tags
                        .iter()
                        .all(|tag| !tag.is_empty() && tag.len() <= 64),
                "市场标签超过限制"
            );
            if let Some(parent) = &marketplace.parent {
                ensure!(
                    parent.starts_with("https://github.com/")
                        && parent.ends_with(".git")
                        && parent.len() <= 256
                        && parent
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b":/._-".contains(&b)),
                    "父插件必须是规范 GitHub 仓库地址"
                );
            }
        }
        validate_relative_path(&plugin.runtime.artifact)?;
        ensure!(
            plugin.runtime.artifact.ends_with(".wasm"),
            "v2 Component 包需要 .wasm 后端产物"
        );
        semver::VersionReq::parse(&plugin.runtime.host_version)?;
        validate_relative_path(&plugin.frontend.path)?;
        ensure!(
            plugin.runtime.artifact != plugin.frontend.path
                && !plugin
                    .runtime
                    .artifact
                    .starts_with(&format!("{}/", plugin.frontend.path)),
            "后端不能放在公开前端目录"
        );
        if let Some(database) = &plugin.database {
            ensure!(plugin.capabilities.database, "迁移必须申请数据库能力");
            validate_relative_path(&database.migrations)?;
            for path in [&plugin.frontend.path, &plugin.runtime.artifact] {
                ensure!(
                    path != &database.migrations
                        && !path.starts_with(&format!("{}/", database.migrations))
                        && !database.migrations.starts_with(&format!("{path}/")),
                    "数据库迁移目录与其他产物重叠"
                );
            }
        }
        Ok(manifest)
    }
}
