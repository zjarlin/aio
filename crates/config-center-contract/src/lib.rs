#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 配置中心运行目录。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCenterPaths {
    pub data_dir: String,
    pub config_dir: String,
    pub state_dir: String,
    pub cache_dir: String,
}

/// 配置中心运行状态。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCenterStatus {
    pub ok: bool,
    pub database_configured: bool,
    pub store_connected: bool,
    pub table_prefix: String,
    pub paths: ConfigCenterPaths,
}

/// PostgreSQL 配置条目投影。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigEntrySummary {
    pub id: String,
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

/// 新建或更新配置条目的请求。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigEntryUpsertInput {
    pub id: Option<String>,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    pub key: String,
    pub value: String,
}

/// 当前机器配对身份。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingLocalInfo {
    pub device_name: String,
    pub fingerprint: String,
    pub home_path: String,
    pub metadata_path: String,
}

/// Dotfiles 巡检状态。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesMonitorStatus {
    pub root: String,
    pub source_home: String,
    pub home: String,
    pub baseline_path: String,
    pub devices: Vec<DotfilesPeerDevice>,
    pub watched_files: usize,
    pub changed_files: usize,
    pub conflict_files: usize,
    pub pending_files: Vec<DotfilesWatchedFile>,
    pub conflicts: Vec<DotfilesConflict>,
    pub updated_at: String,
}

/// Dotfiles 待处理文件。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesWatchedFile {
    pub relative_path: String,
    pub repo_path: String,
    pub target_path: String,
    pub target_name: String,
    pub status: String,
    pub detail: String,
}

/// Dotfiles 冲突摘要与合并上下文。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesConflict {
    pub id: String,
    pub relative_path: String,
    pub repo_path: String,
    pub left_label: String,
    pub right_label: String,
    pub left_path: String,
    pub right_path: String,
    pub title: String,
    pub reason: String,
    pub risk: String,
    pub risk_class: String,
    pub suggestion: String,
    pub local_time: String,
    pub remote_time: String,
    pub local_text: String,
    pub remote_text: String,
    pub base_text: String,
    pub line_start: usize,
    pub line_end: usize,
}

/// Dotfiles 对端设备。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesPeerDevice {
    pub id: String,
    pub name: String,
    pub home_path: String,
    pub enabled: bool,
    pub last_seen: String,
}

fn default_namespace() -> String {
    "az-aio".to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn config_contract_uses_defaults_and_frontend_field_names() -> Result<(), serde_json::Error> {
        let input = serde_json::from_value::<ConfigEntryUpsertInput>(json!({
            "key": "studio.theme",
            "value": "dark"
        }))?;
        let summary = ConfigEntrySummary {
            id: "entry-id".to_string(),
            namespace: input.namespace.clone(),
            key: input.key,
            value: input.value,
            updated_at: "1".to_string(),
        };

        assert_eq!(input.namespace, "az-aio");
        assert_eq!(serde_json::to_value(summary)?["updatedAt"], json!("1"));
        Ok(())
    }
}
