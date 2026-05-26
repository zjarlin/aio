use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    error::{AppError, AppResult},
    plugin_registry::{PluginFormula, SystemCapsule},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMigrationInput {
    pub source_path: PathBuf,
    #[serde(default)]
    pub write: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMigrationReport {
    pub changed_keys: Vec<String>,
    pub manifest_path: String,
    pub schema_version: String,
    pub status: String,
    pub wrote: bool,
}

pub fn migrate_plugin_manifest(input: PluginMigrationInput) -> AppResult<PluginMigrationReport> {
    let manifest_path = resolve_manifest_path(input.source_path)?;
    let original = fs::read_to_string(&manifest_path)?;
    let mut value: Value = serde_json::from_str(&original).map_err(|source| {
        AppError::BadRequest(format!("解析 {} 失败：{source}", manifest_path.display()))
    })?;
    let mut changed_keys = Vec::new();
    migrate_value(&mut value, "$", &mut changed_keys);
    ensure_schema_version(&mut value, &manifest_path, &mut changed_keys)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    validate_migrated_manifest(&value, &schema_version, &manifest_path)?;

    let migrated = serde_json::to_string_pretty(&value)?;
    let changed = normalize_json_string(&original) != normalize_json_string(&migrated);
    if input.write && changed {
        fs::write(&manifest_path, format!("{migrated}\n"))?;
    }

    Ok(PluginMigrationReport {
        changed_keys,
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        schema_version,
        status: if changed { "migrated" } else { "unchanged" }.to_string(),
        wrote: input.write && changed,
    })
}

fn resolve_manifest_path(source_path: PathBuf) -> AppResult<PathBuf> {
    if source_path.is_file() {
        return Ok(source_path);
    }
    if !source_path.is_dir() {
        return Err(AppError::NotFound);
    }
    let formula = source_path.join("formula.json");
    if formula.is_file() {
        return Ok(formula);
    }
    let capsule = source_path.join("system-capsule.json");
    if capsule.is_file() {
        return Ok(capsule);
    }
    Err(AppError::BadRequest(format!(
        "未找到 formula.json 或 system-capsule.json：{}",
        source_path.display()
    )))
}

fn migrate_value(value: &mut Value, path: &str, changed_keys: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            let original = std::mem::take(object);
            let mut migrated = Map::new();
            for (key, mut child) in original {
                let new_key = legacy_key(&key).unwrap_or(key.as_str()).to_string();
                let child_path = format!("{path}.{new_key}");
                migrate_value(&mut child, &child_path, changed_keys);
                if new_key != key {
                    changed_keys.push(format!("{path}.{key}->{new_key}"));
                }
                migrated.insert(new_key, child);
            }
            *object = migrated;
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                migrate_value(item, &format!("{path}[{index}]"), changed_keys);
            }
        }
        _ => {}
    }
}

fn ensure_schema_version(
    value: &mut Value,
    manifest_path: &std::path::Path,
    changed_keys: &mut Vec<String>,
) -> AppResult<()> {
    let Some(object) = value.as_object_mut() else {
        return Err(AppError::BadRequest(format!(
            "{} 必须是 JSON object",
            manifest_path.display()
        )));
    };
    if object.contains_key("schemaVersion") {
        return Ok(());
    }
    let inferred = if manifest_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "system-capsule.json")
        .unwrap_or(false)
        || object.get("kind").and_then(Value::as_str) == Some("system")
    {
        "system-capsule/v1"
    } else {
        "plugin-formula/v1"
    };
    object.insert(
        "schemaVersion".to_string(),
        Value::String(inferred.to_string()),
    );
    changed_keys.push("$.schemaVersion".to_string());
    Ok(())
}

fn validate_migrated_manifest(
    value: &Value,
    schema_version: &str,
    manifest_path: &std::path::Path,
) -> AppResult<()> {
    match schema_version {
        "plugin-formula/v1" => {
            let _: PluginFormula = serde_json::from_value(value.clone()).map_err(|source| {
                AppError::BadRequest(format!(
                    "迁移后插件公式仍无效：{}: {source}",
                    manifest_path.display()
                ))
            })?;
        }
        "system-capsule/v1" => {
            let _: SystemCapsule = serde_json::from_value(value.clone()).map_err(|source| {
                AppError::BadRequest(format!(
                    "迁移后系统胶囊仍无效：{}: {source}",
                    manifest_path.display()
                ))
            })?;
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "不支持的 schemaVersion：{other}"
            )));
        }
    }
    Ok(())
}

fn legacy_key(key: &str) -> Option<&'static str> {
    Some(match key {
        "allow_third_party_children" => "allowThirdPartyChildren",
        "allowed_child_capabilities" => "allowedChildCapabilities",
        "allowed_contribution_kinds" => "allowedContributionKinds",
        "asset_item_kind" => "assetItemKind",
        "capability_mode" => "capabilityMode",
        "child_policy" => "childPolicy",
        "compatible_parent_range" => "compatibleParentRange",
        "display_name" => "displayName",
        "do_not_split_below" => "doNotSplitBelow",
        "editable_by_ai" => "editableByAI",
        "event_namespace" => "eventNamespace",
        "extension_points" => "extensionPoints",
        "managed_by" => "managedBy",
        "ordinary_user" => "ordinaryUser",
        "parent_permission_id" => "parentPermissionId",
        "permission_id" => "permissionId",
        "permission_type" => "permissionType",
        "preferred_files" => "preferredFiles",
        "primary_edit_file" => "primaryEditFile",
        "primary_output" => "primaryOutput",
        "readable_by_ai" => "readableByAI",
        "repair_strategy" => "repairStrategy",
        "requires_platform_approval_for_new_capabilities" => {
            "requiresPlatformApprovalForNewCapabilities"
        }
        "schema_version" => "schemaVersion",
        "sort_order" => "sortOrder",
        "source_id" => "sourceId",
        "source_kind" => "sourceKind",
        _ => return None,
    })
}

fn normalize_json_string(value: &str) -> String {
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{migrate_plugin_manifest, PluginMigrationInput};
    use crate::plugin_registry::PluginFormula;
    use std::fs;

    #[test]
    fn migrate_should_normalize_legacy_plugin_formula_keys() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let plugin = tempdir.path().join("legacy-plugin");
        fs::create_dir_all(&plugin).expect("plugin dir");
        fs::write(
            plugin.join("formula.json"),
            r#"{
  "schema_version": "plugin-formula/v1",
  "id": "legacy.plugin",
  "kind": "plugin",
  "display_name": "Legacy Plugin",
  "contributes": {
    "extension_points": [
      {
        "id": "legacy.plugin.children",
        "title": "Children",
        "allowed_contribution_kinds": ["command"]
      }
    ]
  },
  "child_policy": {
    "allow_third_party_children": true,
    "capability_mode": "intersection",
    "event_namespace": "legacy.plugin/*",
    "requires_platform_approval_for_new_capabilities": true,
    "allowed_child_capabilities": ["fs.read"]
  },
  "ai": {
    "editable_by_ai": true,
    "primary_edit_file": "formula.json",
    "preferred_files": ["formula.json"],
    "do_not_split_below": "single file"
  }
}"#,
        )
        .expect("legacy formula");

        let dry_run = migrate_plugin_manifest(PluginMigrationInput {
            source_path: plugin.clone(),
            write: false,
        })
        .expect("dry run");
        assert_eq!(dry_run.status, "migrated");
        assert!(!dry_run.wrote);
        assert!(fs::read_to_string(plugin.join("formula.json"))
            .expect("read original")
            .contains("schema_version"));

        let report = migrate_plugin_manifest(PluginMigrationInput {
            source_path: plugin.clone(),
            write: true,
        })
        .expect("migrate");
        assert_eq!(report.schema_version, "plugin-formula/v1");
        assert!(report.wrote);
        assert!(report
            .changed_keys
            .iter()
            .any(|key| key == "$.schema_version->schemaVersion"));

        let migrated = fs::read_to_string(plugin.join("formula.json")).expect("migrated formula");
        assert!(migrated.contains("\"schemaVersion\""));
        assert!(migrated.contains("\"displayName\""));
        assert!(migrated.contains("\"extensionPoints\""));
        assert!(migrated.contains("\"allowedContributionKinds\""));
        let formula: PluginFormula = serde_json::from_str(&migrated).expect("valid formula");
        assert_eq!(formula.display_name, "Legacy Plugin");
        assert_eq!(
            formula
                .child_policy
                .expect("child policy")
                .allowed_child_capabilities,
            vec!["fs.read".to_string()]
        );
    }

    #[test]
    fn migrate_should_infer_system_capsule_schema_version() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let capsule = tempdir.path().join("system-capsule.json");
        fs::write(
            &capsule,
            r#"{
  "id": "platform.legacy",
  "kind": "system",
  "display_name": "Legacy System",
  "managed_by": "platform"
}"#,
        )
        .expect("legacy capsule");

        let report = migrate_plugin_manifest(PluginMigrationInput {
            source_path: capsule.clone(),
            write: true,
        })
        .expect("migrate capsule");
        assert_eq!(report.schema_version, "system-capsule/v1");
        let migrated = fs::read_to_string(capsule).expect("migrated capsule");
        assert!(migrated.contains("\"schemaVersion\""));
        assert!(migrated.contains("\"displayName\""));
        assert!(migrated.contains("\"managedBy\""));
    }
}
