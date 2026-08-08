#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// PostgreSQL 资产注册表投影。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub source: String,
    pub updated_at: String,
}

/// 新建或更新资产的请求。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUpsertInput {
    pub id: Option<String>,
    pub kind: String,
    pub title: String,
    #[serde(default = "default_asset_status")]
    pub status: String,
    #[serde(default = "default_asset_source")]
    pub source: String,
}

/// 本地技能目录扫描摘要。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedSkillSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub source: String,
    pub origin: String,
    pub tags: Vec<String>,
    pub status: String,
}

fn default_asset_status() -> String {
    "active".to_string()
}

fn default_asset_source() -> String {
    "asset-hub".to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn asset_contract_uses_frontend_field_names_and_defaults() -> Result<(), serde_json::Error> {
        let input = serde_json::from_value::<AssetUpsertInput>(json!({
            "kind": "skill",
            "title": "Rust 规范"
        }))?;
        let summary = AssetSummary {
            id: "skill-rust".to_string(),
            kind: input.kind.clone(),
            title: input.title.clone(),
            status: input.status.clone(),
            source: input.source.clone(),
            updated_at: "1".to_string(),
        };
        let scanned = ScannedSkillSummary {
            id: "skill-rust".to_string(),
            name: "rust".to_string(),
            asset_type: "skill".to_string(),
            source: "/skills/rust/SKILL.md".to_string(),
            origin: "Skill directory scan".to_string(),
            tags: vec!["rust".to_string()],
            status: "synced".to_string(),
        };

        assert_eq!(input.status, "active");
        assert_eq!(input.source, "asset-hub");
        assert_eq!(serde_json::to_value(summary)?["updatedAt"], json!("1"));
        assert_eq!(serde_json::to_value(scanned)?["type"], json!("skill"));
        Ok(())
    }
}
