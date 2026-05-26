use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
    plugin_registry::{PluginFormula, SystemCapsule},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginRecord {
    pub content_hash: String,
    pub enabled: bool,
    pub id: String,
    pub installed_at: i64,
    pub kind: String,
    pub registry_path: String,
    pub schema_version: String,
    #[serde(default)]
    pub signature_path: String,
    pub source_path: String,
    pub updated_at: i64,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryLockRecord {
    pub content_hash: String,
    pub enabled: bool,
    pub id: String,
    pub locked_at: i64,
    pub registry_path: String,
    pub schema_version: String,
    #[serde(default)]
    pub signature_path: String,
    pub source_path: String,
    pub updated_at: i64,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryAuditRecord {
    pub action: String,
    pub content_hash: Option<String>,
    pub detail: Option<String>,
    pub id: String,
    pub path: Option<String>,
    pub status: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageSignaturePlaceholder {
    pub algorithm: String,
    pub content_hash: String,
    pub created_at: i64,
    pub key_id: String,
    pub plugin_id: String,
    pub reason: String,
    pub schema_version: String,
    pub signature: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistryVersionRecord {
    pub content_hash: String,
    pub created_at: i64,
    pub id: String,
    pub kind: String,
    pub registry_path: String,
    pub schema_version: String,
    pub signature_path: String,
    pub source_path: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildCapabilityApprovalRecord {
    pub capability: String,
    pub child_plugin_id: String,
    pub created_at: i64,
    pub parent_plugin_id: String,
    pub reason: String,
    pub revoked_at: Option<i64>,
    pub revoked_reason: String,
    pub status: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildCapabilityApprovalInput {
    pub capability: String,
    pub child_plugin_id: String,
    pub parent_plugin_id: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistryRollbackInput {
    pub id: String,
    #[serde(default)]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistryRollbackResult {
    pub previous_content_hash: String,
    pub restored: InstalledPluginRecord,
    pub selected_history: PluginRegistryVersionRecord,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistryLocalState {
    pub audits: Vec<RegistryAuditRecord>,
    #[serde(default)]
    pub child_capability_approvals: Vec<ChildCapabilityApprovalRecord>,
    #[serde(default)]
    pub history: Vec<PluginRegistryVersionRecord>,
    pub installed: Vec<InstalledPluginRecord>,
    pub locks: Vec<RegistryLockRecord>,
}

#[derive(Debug, Clone)]
pub struct PluginRegistryStore {
    root: PathBuf,
}

impl PluginRegistryStore {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            root: data_dir.as_ref().join("plugin-registry"),
        }
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }

    pub fn history_dir(&self) -> PathBuf {
        self.root.join("history")
    }

    pub fn signatures_dir(&self) -> PathBuf {
        self.root.join("signatures")
    }

    pub fn remote_protocol_path(&self) -> PathBuf {
        self.root.join("remote-registry.protocol.json")
    }

    pub fn installed_path(&self) -> PathBuf {
        self.root.join("installed.json")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join("lock.json")
    }

    pub fn audit_path(&self) -> PathBuf {
        self.root.join("audit.jsonl")
    }

    pub fn child_capability_approvals_path(&self) -> PathBuf {
        self.root.join("child-capability-approvals.json")
    }

    pub fn ensure_layout(&self) -> AppResult<()> {
        fs::create_dir_all(self.plugins_dir())?;
        fs::create_dir_all(self.history_dir())?;
        fs::create_dir_all(self.signatures_dir())?;
        self.ensure_remote_protocol_draft()?;
        Ok(())
    }

    pub fn load_installed(&self) -> AppResult<Vec<InstalledPluginRecord>> {
        read_json_list(&self.installed_path())
    }

    pub fn load_lock(&self) -> AppResult<Vec<RegistryLockRecord>> {
        read_json_list(&self.lock_path())
    }

    pub fn load_audits(&self) -> AppResult<Vec<RegistryAuditRecord>> {
        if !self.audit_path().is_file() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(self.audit_path())?;
        let reader = BufReader::new(file);
        let mut audits = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            audits.push(serde_json::from_str(&line).map_err(|source| {
                AppError::BadRequest(format!(
                    "解析 {} 失败：{source}",
                    self.audit_path().display()
                ))
            })?);
        }
        Ok(audits)
    }

    pub fn state(&self) -> AppResult<PluginRegistryLocalState> {
        Ok(PluginRegistryLocalState {
            audits: self.load_audits()?,
            child_capability_approvals: self.load_child_capability_approvals()?,
            history: self.load_history()?,
            installed: self.load_installed()?,
            locks: self.load_lock()?,
        })
    }

    pub fn enabled_ids(&self) -> AppResult<HashMap<String, bool>> {
        Ok(self
            .load_installed()?
            .into_iter()
            .map(|entry| (entry.id.clone(), entry.enabled))
            .collect())
    }

    pub fn load_child_capability_approvals(&self) -> AppResult<Vec<ChildCapabilityApprovalRecord>> {
        read_json_list(&self.child_capability_approvals_path())
    }

    pub fn approve_child_capability(
        &self,
        input: ChildCapabilityApprovalInput,
    ) -> AppResult<ChildCapabilityApprovalRecord> {
        self.ensure_layout()?;
        let parent_plugin_id = normalize_required(&input.parent_plugin_id, "parentPluginId")?;
        let child_plugin_id = normalize_required(&input.child_plugin_id, "childPluginId")?;
        let capability = normalize_required(&input.capability, "capability")?;
        let now = now_millis();
        let mut approvals = self.load_child_capability_approvals()?;
        let record = ChildCapabilityApprovalRecord {
            capability: capability.clone(),
            child_plugin_id: child_plugin_id.clone(),
            created_at: approvals
                .iter()
                .find(|approval| {
                    approval.parent_plugin_id == parent_plugin_id
                        && approval.child_plugin_id == child_plugin_id
                        && approval.capability == capability
                })
                .map(|approval| approval.created_at)
                .unwrap_or(now),
            parent_plugin_id: parent_plugin_id.clone(),
            reason: input.reason.trim().to_string(),
            revoked_at: None,
            revoked_reason: String::new(),
            status: "approved".to_string(),
            updated_at: now,
        };
        upsert_child_capability_approval(&mut approvals, record.clone());
        save_json_list(&self.child_capability_approvals_path(), &approvals)?;
        self.append_audit(RegistryAuditRecord {
            action: "approve-child-capability".to_string(),
            content_hash: None,
            detail: Some(format!(
                "parent={} child={} capability={} reason={}",
                parent_plugin_id, child_plugin_id, capability, record.reason
            )),
            id: child_plugin_id,
            path: Some(
                self.child_capability_approvals_path()
                    .to_string_lossy()
                    .into_owned(),
            ),
            status: "ok".to_string(),
            timestamp: now,
        })?;
        Ok(record)
    }

    pub fn revoke_child_capability(
        &self,
        input: ChildCapabilityApprovalInput,
    ) -> AppResult<ChildCapabilityApprovalRecord> {
        self.ensure_layout()?;
        let parent_plugin_id = normalize_required(&input.parent_plugin_id, "parentPluginId")?;
        let child_plugin_id = normalize_required(&input.child_plugin_id, "childPluginId")?;
        let capability = normalize_required(&input.capability, "capability")?;
        let now = now_millis();
        let mut approvals = self.load_child_capability_approvals()?;
        let Some(record) = approvals.iter_mut().find(|approval| {
            approval.parent_plugin_id == parent_plugin_id
                && approval.child_plugin_id == child_plugin_id
                && approval.capability == capability
        }) else {
            return Err(AppError::NotFound);
        };
        record.status = "revoked".to_string();
        record.revoked_at = Some(now);
        record.revoked_reason = input.reason.trim().to_string();
        record.updated_at = now;
        let record = record.clone();
        save_json_list(&self.child_capability_approvals_path(), &approvals)?;
        self.append_audit(RegistryAuditRecord {
            action: "revoke-child-capability".to_string(),
            content_hash: None,
            detail: Some(format!(
                "parent={} child={} capability={} reason={}",
                parent_plugin_id, child_plugin_id, capability, record.revoked_reason
            )),
            id: child_plugin_id,
            path: Some(
                self.child_capability_approvals_path()
                    .to_string_lossy()
                    .into_owned(),
            ),
            status: "ok".to_string(),
            timestamp: now,
        })?;
        Ok(record)
    }

    pub fn install(&self, source_path: impl AsRef<Path>) -> AppResult<InstalledPluginRecord> {
        self.ensure_layout()?;
        let source_path = source_path.as_ref();
        let manifest = read_manifest(source_path)?;
        let id = manifest.id;
        let kind = manifest.kind;
        let schema_version = manifest.schema_version;
        let version = manifest.version;
        let content_hash = hash_directory(source_path)?;
        let registry_path = self.plugins_dir().join(&id);
        let history_root = self.history_path(&id, now_millis(), &content_hash);
        let history_package_path = history_root.join("package");
        let signature = self.write_signature_placeholder(&id, &content_hash)?;
        copy_directory(source_path, &registry_path)?;
        copy_directory(source_path, &history_package_path)?;

        let now = now_millis();
        let record = InstalledPluginRecord {
            content_hash: content_hash.clone(),
            enabled: true,
            id: id.clone(),
            installed_at: now,
            kind: kind.clone(),
            registry_path: registry_path.to_string_lossy().into_owned(),
            schema_version: schema_version.clone(),
            signature_path: signature_path_from_placeholder(&signature),
            source_path: source_path.to_string_lossy().into_owned(),
            updated_at: now,
            version: version.clone(),
        };
        let lock = RegistryLockRecord {
            content_hash: content_hash.clone(),
            enabled: true,
            id,
            locked_at: now,
            registry_path: registry_path.to_string_lossy().into_owned(),
            schema_version,
            signature_path: signature_path_from_placeholder(&signature),
            source_path: source_path.to_string_lossy().into_owned(),
            updated_at: now,
            version,
        };
        let history = PluginRegistryVersionRecord {
            content_hash: content_hash.clone(),
            created_at: now,
            id: record.id.clone(),
            kind: kind.clone(),
            registry_path: history_package_path.to_string_lossy().into_owned(),
            schema_version: record.schema_version.clone(),
            signature_path: record.signature_path.clone(),
            source_path: source_path.to_string_lossy().into_owned(),
            version: record.version.clone(),
        };
        fs::write(
            history_root.join("version.json"),
            serde_json::to_string_pretty(&history)?,
        )?;

        let mut installed = self.load_installed()?;
        upsert_installed(&mut installed, record.clone());
        save_json_list(&self.installed_path(), &installed)?;

        let mut locks = self.load_lock()?;
        upsert_lock(&mut locks, lock);
        save_json_list(&self.lock_path(), &locks)?;

        self.append_audit(RegistryAuditRecord {
            action: "install".to_string(),
            content_hash: Some(content_hash),
            detail: Some(format!(
                "version={} history={} signature={}",
                record.version, history.registry_path, record.signature_path
            )),
            id: record.id.clone(),
            path: Some(registry_path.to_string_lossy().into_owned()),
            status: "ok".to_string(),
            timestamp: now,
        })?;

        Ok(record)
    }

    pub fn rollback(
        &self,
        input: PluginRegistryRollbackInput,
    ) -> AppResult<PluginRegistryRollbackResult> {
        self.ensure_layout()?;
        let mut installed = self.load_installed()?;
        let Some(index) = installed.iter().position(|entry| entry.id == input.id) else {
            return Err(AppError::NotFound);
        };
        let previous = installed[index].clone();
        let selected_history = self.select_history(
            &input.id,
            input.content_hash.as_deref(),
            &previous.content_hash,
        )?;
        let registry_path = self.plugins_dir().join(&input.id);
        copy_directory(Path::new(&selected_history.registry_path), &registry_path)?;

        let now = now_millis();
        let restored = InstalledPluginRecord {
            content_hash: selected_history.content_hash.clone(),
            enabled: previous.enabled,
            id: selected_history.id.clone(),
            installed_at: previous.installed_at,
            kind: selected_history.kind.clone(),
            registry_path: registry_path.to_string_lossy().into_owned(),
            schema_version: selected_history.schema_version.clone(),
            signature_path: selected_history.signature_path.clone(),
            source_path: selected_history.source_path.clone(),
            updated_at: now,
            version: selected_history.version.clone(),
        };
        installed[index] = restored.clone();
        save_json_list(&self.installed_path(), &installed)?;

        let mut locks = self.load_lock()?;
        upsert_lock(
            &mut locks,
            RegistryLockRecord {
                content_hash: restored.content_hash.clone(),
                enabled: restored.enabled,
                id: restored.id.clone(),
                locked_at: now,
                registry_path: restored.registry_path.clone(),
                schema_version: restored.schema_version.clone(),
                signature_path: restored.signature_path.clone(),
                source_path: restored.source_path.clone(),
                updated_at: now,
                version: restored.version.clone(),
            },
        );
        save_json_list(&self.lock_path(), &locks)?;

        self.append_audit(RegistryAuditRecord {
            action: "rollback".to_string(),
            content_hash: Some(restored.content_hash.clone()),
            detail: Some(format!(
                "previousContentHash={} restoredFrom={}",
                previous.content_hash, selected_history.registry_path
            )),
            id: restored.id.clone(),
            path: Some(restored.registry_path.clone()),
            status: "ok".to_string(),
            timestamp: now,
        })?;

        Ok(PluginRegistryRollbackResult {
            previous_content_hash: previous.content_hash,
            restored,
            selected_history,
        })
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> AppResult<InstalledPluginRecord> {
        self.ensure_layout()?;
        let mut installed = self.load_installed()?;
        let now = now_millis();
        let Some(index) = installed.iter().position(|entry| entry.id == id) else {
            return Err(AppError::NotFound);
        };

        let parent_by_child = installed_parent_links(&installed)?;
        if enabled {
            ensure_installed_ancestors_enabled(&installed, &parent_by_child, id)?;
        }

        let cascade_ids = if enabled {
            HashSet::new()
        } else {
            installed_descendant_ids(&parent_by_child, id)
        };
        let mut changed_ids = HashSet::new();
        let mut cascade_disabled = Vec::new();

        installed[index].enabled = enabled;
        installed[index].updated_at = now;
        changed_ids.insert(installed[index].id.clone());
        for entry in &mut installed {
            if cascade_ids.contains(&entry.id) && entry.enabled {
                entry.enabled = false;
                entry.updated_at = now;
                changed_ids.insert(entry.id.clone());
                cascade_disabled.push(entry.clone());
            }
        }

        let record = installed[index].clone();
        save_json_list(&self.installed_path(), &installed)?;

        let mut locks = self.load_lock()?;
        for lock in &mut locks {
            if changed_ids.contains(&lock.id) {
                if let Some(installed_record) = installed.iter().find(|entry| entry.id == lock.id) {
                    lock.enabled = installed_record.enabled;
                    lock.updated_at = now;
                }
            }
        }
        save_json_list(&self.lock_path(), &locks)?;

        self.append_audit(RegistryAuditRecord {
            action: if enabled {
                "enable".to_string()
            } else {
                "disable".to_string()
            },
            content_hash: Some(record.content_hash.clone()),
            detail: if cascade_disabled.is_empty() {
                None
            } else {
                Some(format!(
                    "cascadeDisabled={}",
                    cascade_disabled
                        .iter()
                        .map(|entry| entry.id.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
            },
            id: record.id.clone(),
            path: Some(record.registry_path.clone()),
            status: "ok".to_string(),
            timestamp: now,
        })?;

        for child in cascade_disabled {
            self.append_audit(RegistryAuditRecord {
                action: "disable-cascade".to_string(),
                content_hash: Some(child.content_hash),
                detail: Some(format!("parent={id}")),
                id: child.id,
                path: Some(child.registry_path),
                status: "ok".to_string(),
                timestamp: now,
            })?;
        }

        Ok(record)
    }

    pub fn uninstall(&self, id: &str) -> AppResult<()> {
        let mut installed = self.load_installed()?;
        let Some(index) = installed.iter().position(|entry| entry.id == id) else {
            return Err(AppError::NotFound);
        };
        let record = installed.remove(index);
        save_json_list(&self.installed_path(), &installed)?;

        let mut locks = self.load_lock()?;
        locks.retain(|entry| entry.id != id);
        save_json_list(&self.lock_path(), &locks)?;

        let registry_path = PathBuf::from(&record.registry_path);
        if registry_path.exists() {
            fs::remove_dir_all(&registry_path)?;
        }

        self.append_audit(RegistryAuditRecord {
            action: "uninstall".to_string(),
            content_hash: Some(record.content_hash),
            detail: None,
            id: record.id,
            path: Some(record.registry_path),
            status: "ok".to_string(),
            timestamp: now_millis(),
        })?;

        Ok(())
    }

    pub fn append_audit(&self, record: RegistryAuditRecord) -> AppResult<()> {
        self.ensure_layout()?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.audit_path())?;
        writeln!(file, "{}", serde_json::to_string(&record)?)?;
        Ok(())
    }

    pub fn load_history(&self) -> AppResult<Vec<PluginRegistryVersionRecord>> {
        let history_dir = self.history_dir();
        if !history_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut history = Vec::new();
        for entry in WalkDir::new(&history_dir).min_depth(2).max_depth(3) {
            let entry = entry.map_err(|error| {
                AppError::BadRequest(format!(
                    "扫描插件历史目录失败：{}: {error}",
                    history_dir.display()
                ))
            })?;
            if entry.file_type().is_file() && entry.file_name().to_string_lossy() == "version.json"
            {
                history.push(read_json_file(entry.path())?);
            }
        }
        history.sort_by(|left: &PluginRegistryVersionRecord, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(history)
    }

    fn select_history(
        &self,
        id: &str,
        content_hash: Option<&str>,
        current_hash: &str,
    ) -> AppResult<PluginRegistryVersionRecord> {
        self.load_history()?
            .into_iter()
            .find(|record| {
                record.id == id
                    && content_hash
                        .map(|hash| record.content_hash == hash)
                        .unwrap_or_else(|| record.content_hash != current_hash)
            })
            .ok_or(AppError::NotFound)
    }

    fn history_path(&self, id: &str, timestamp: i64, content_hash: &str) -> PathBuf {
        let hash_prefix = content_hash.chars().take(12).collect::<String>();
        self.history_dir()
            .join(id)
            .join(format!("{timestamp}-{hash_prefix}"))
    }

    fn write_signature_placeholder(
        &self,
        plugin_id: &str,
        content_hash: &str,
    ) -> AppResult<PluginPackageSignaturePlaceholder> {
        fs::create_dir_all(self.signatures_dir())?;
        let created_at = now_millis();
        let signature = PluginPackageSignaturePlaceholder {
            algorithm: "sha256-placeholder".to_string(),
            content_hash: content_hash.to_string(),
            created_at,
            key_id: "local-dev-placeholder".to_string(),
            plugin_id: plugin_id.to_string(),
            reason: "Local registry keeps a signature placeholder until real signing keys are configured.".to_string(),
            schema_version: "plugin-signature-placeholder/v1".to_string(),
            signature: format!("unsigned:{content_hash}"),
            status: "placeholder".to_string(),
        };
        let signature_path = self
            .signatures_dir()
            .join(format!("{plugin_id}-{content_hash}.signature.json"));
        fs::write(&signature_path, serde_json::to_string_pretty(&signature)?)?;
        Ok(signature)
    }

    fn ensure_remote_protocol_draft(&self) -> AppResult<()> {
        if self.remote_protocol_path().is_file() {
            return Ok(());
        }
        let draft = serde_json::json!({
            "schemaVersion": "aio-plugin-remote-registry-protocol/v0",
            "status": "draft",
            "transport": "https+json",
            "endpoints": [
                { "method": "GET", "path": "/v1/plugins", "purpose": "List searchable plugin metadata." },
                { "method": "GET", "path": "/v1/plugins/{id}/versions/{version}", "purpose": "Read a version manifest including formula hash and compatibility." },
                { "method": "GET", "path": "/v1/packages/{id}/{contentHash}", "purpose": "Download immutable plugin package content." },
                { "method": "POST", "path": "/v1/plugins/{id}/publish", "purpose": "Upload a package after publish gate passes." }
            ],
            "packageRequiredFiles": [
                "formula.json",
                "verification.json",
                "signature.json",
                "lock.json",
                "audit.jsonl"
            ],
            "publishGateChecks": [
                "formula.schema",
                "permission.plan",
                "platform.matrix",
                "smoke.test",
                "lock.present",
                "signature.placeholder",
                "audit.intent"
            ]
        });
        fs::create_dir_all(&self.root)?;
        fs::write(
            self.remote_protocol_path(),
            serde_json::to_string_pretty(&draft)?,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ManifestSummary {
    id: String,
    kind: String,
    schema_version: String,
    version: String,
}

fn read_manifest(source_path: &Path) -> AppResult<ManifestSummary> {
    let formula_path = source_path.join("formula.json");
    if formula_path.is_file() {
        let manifest: PluginFormula = read_json_file(&formula_path)?;
        return Ok(ManifestSummary {
            id: manifest.id,
            kind: manifest.kind,
            schema_version: manifest.schema_version,
            version: manifest.version,
        });
    }

    let capsule_path = source_path.join("system-capsule.json");
    if capsule_path.is_file() {
        let manifest: SystemCapsule = read_json_file(&capsule_path)?;
        return Ok(ManifestSummary {
            id: manifest.id,
            kind: manifest.kind,
            schema_version: manifest.schema_version,
            version: String::new(),
        });
    }

    Err(AppError::BadRequest(format!(
        "未找到插件公式或系统胶囊：{}",
        source_path.display()
    )))
}

fn read_json_list<T>(path: &Path) -> AppResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let value = fs::read_to_string(path)?;
    serde_json::from_str(&value)
        .map_err(|source| AppError::BadRequest(format!("解析 {} 失败：{source}", path.display())))
}

fn read_json_file<T>(path: &Path) -> AppResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let value = fs::read_to_string(path)?;
    serde_json::from_str(&value)
        .map_err(|source| AppError::BadRequest(format!("解析 {} 失败：{source}", path.display())))
}

fn save_json_list<T>(path: &Path, value: &[T]) -> AppResult<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn installed_parent_links(
    installed: &[InstalledPluginRecord],
) -> AppResult<HashMap<String, String>> {
    let mut links = HashMap::new();
    for entry in installed {
        if let Some(parent_id) = read_installed_parent_id(entry)? {
            links.insert(entry.id.clone(), parent_id);
        }
    }
    Ok(links)
}

fn read_installed_parent_id(entry: &InstalledPluginRecord) -> AppResult<Option<String>> {
    let formula_path = Path::new(&entry.registry_path).join("formula.json");
    if !formula_path.is_file() {
        return Ok(None);
    }

    let formula: PluginFormula = read_json_file(&formula_path)?;
    Ok(formula.parent.and_then(|parent| {
        let parent_id = parent.plugin_id.trim().to_string();
        if parent_id.is_empty() {
            None
        } else {
            Some(parent_id)
        }
    }))
}

fn ensure_installed_ancestors_enabled(
    installed: &[InstalledPluginRecord],
    parent_by_child: &HashMap<String, String>,
    id: &str,
) -> AppResult<()> {
    let installed_by_id = installed
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let mut current = id;
    let mut seen = HashSet::new();
    while let Some(parent_id) = parent_by_child.get(current) {
        if !seen.insert(parent_id.clone()) {
            break;
        }
        if let Some(parent) = installed_by_id.get(parent_id.as_str()) {
            if !parent.enabled {
                return Err(AppError::Conflict(format!(
                    "child plugin {id} cannot be enabled while parent plugin {parent_id} is disabled"
                )));
            }
        }
        current = parent_id;
    }
    Ok(())
}

fn installed_descendant_ids(
    parent_by_child: &HashMap<String, String>,
    parent_id: &str,
) -> HashSet<String> {
    let mut children_by_parent: HashMap<String, Vec<String>> = HashMap::new();
    for (child_id, parent_id) in parent_by_child {
        children_by_parent
            .entry(parent_id.clone())
            .or_default()
            .push(child_id.clone());
    }

    let mut result = HashSet::new();
    let mut stack = children_by_parent
        .get(parent_id)
        .cloned()
        .unwrap_or_default();
    while let Some(child_id) = stack.pop() {
        if result.insert(child_id.clone()) {
            if let Some(children) = children_by_parent.get(&child_id) {
                stack.extend(children.iter().cloned());
            }
        }
    }
    result
}

fn upsert_installed(installed: &mut Vec<InstalledPluginRecord>, record: InstalledPluginRecord) {
    if let Some(existing) = installed.iter_mut().find(|entry| entry.id == record.id) {
        *existing = record;
    } else {
        installed.push(record);
    }
}

fn upsert_lock(locks: &mut Vec<RegistryLockRecord>, record: RegistryLockRecord) {
    if let Some(existing) = locks.iter_mut().find(|entry| entry.id == record.id) {
        *existing = record;
    } else {
        locks.push(record);
    }
}

fn upsert_child_capability_approval(
    approvals: &mut Vec<ChildCapabilityApprovalRecord>,
    record: ChildCapabilityApprovalRecord,
) {
    if let Some(existing) = approvals.iter_mut().find(|entry| {
        entry.parent_plugin_id == record.parent_plugin_id
            && entry.child_plugin_id == record.child_plugin_id
            && entry.capability == record.capability
    }) {
        *existing = record;
    } else {
        approvals.push(record);
    }
    approvals.sort_by(|left, right| {
        left.parent_plugin_id
            .cmp(&right.parent_plugin_id)
            .then_with(|| left.child_plugin_id.cmp(&right.child_plugin_id))
            .then_with(|| left.capability.cmp(&right.capability))
    });
}

fn normalize_required(value: &str, field: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn copy_directory(source: &Path, destination: &Path) -> AppResult<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;

    let mut paths = Vec::new();
    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!("扫描插件目录失败：{}: {error}", source.display()))
        })?;
        if entry.file_type().is_file() {
            paths.push(entry.into_path());
        }
    }
    paths.sort();

    for path in paths {
        let relative = path.strip_prefix(source).map_err(|error| {
            AppError::BadRequest(format!(
                "复制插件目录失败：{} -> {}: {error}",
                source.display(),
                destination.display()
            ))
        })?;
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&path, &target)?;
    }

    Ok(())
}

fn hash_directory(path: &Path) -> AppResult<String> {
    let mut hasher = Sha256::new();
    let mut files = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!("扫描插件目录失败：{}: {error}", path.display()))
        })?;
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if matches!(
                relative.as_str(),
                "audit.jsonl"
                    | "diagnostics.json"
                    | "lock.json"
                    | "signature.json"
                    | "verification.json"
            ) {
                continue;
            }
            files.push(entry.into_path());
        }
    }
    files.sort();

    for file in files {
        let relative = file.strip_prefix(path).unwrap_or(&file);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(fs::read(&file)?);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn signature_path_from_placeholder(signature: &PluginPackageSignaturePlaceholder) -> String {
    format!(
        "signatures/{}-{}.signature.json",
        signature.plugin_id, signature.content_hash
    )
}

#[cfg(test)]
mod tests {
    use super::{ChildCapabilityApprovalInput, PluginRegistryStore};
    use crate::error::AppError;
    use std::fs;

    #[test]
    fn store_should_create_registry_root_from_data_dir() {
        let store = PluginRegistryStore::new("/private/tmp/plugin-registry-test");
        assert!(store
            .root()
            .to_string_lossy()
            .contains("plugin-registry-test"));
    }

    #[test]
    fn install_should_write_registry_files_and_toggle_state() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source = tempdir.path().join("source-plugin");
        fs::create_dir_all(&source).expect("source dir");
        fs::write(
            source.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "test.plugin",
  "kind": "plugin",
  "displayName": "Test Plugin"
}"#,
        )
        .expect("formula");

        let store = PluginRegistryStore::new(tempdir.path());
        let installed = store.install(&source).expect("install");
        assert_eq!(installed.id, "test.plugin");
        assert!(store.installed_path().is_file());
        assert!(store.lock_path().is_file());
        assert!(store.audit_path().is_file());

        let state = store.state().expect("state");
        assert_eq!(state.installed.len(), 1);
        assert_eq!(state.locks.len(), 1);
        assert_eq!(state.audits.len(), 1);
        assert_eq!(state.history.len(), 1);
        assert!(store.remote_protocol_path().is_file());

        let disabled = store.set_enabled("test.plugin", false).expect("disable");
        assert!(!disabled.enabled);

        store.uninstall("test.plugin").expect("uninstall");
        let state = store.state().expect("state after uninstall");
        assert!(state.installed.is_empty());
    }

    #[test]
    fn disabling_parent_plugin_disables_installed_child_subtree() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let parent = tempdir.path().join("parent-plugin");
        let child = tempdir.path().join("child-plugin");
        let grandchild = tempdir.path().join("grandchild-plugin");
        fs::create_dir_all(&parent).expect("parent dir");
        fs::create_dir_all(&child).expect("child dir");
        fs::create_dir_all(&grandchild).expect("grandchild dir");
        fs::write(
            parent.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "parent.plugin",
  "kind": "plugin",
  "displayName": "Parent Plugin",
  "version": "1.0.0"
}"#,
        )
        .expect("parent formula");
        fs::write(
            child.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "parent.plugin.child",
  "kind": "child-plugin",
  "displayName": "Child Plugin",
  "parent": {
    "pluginId": "parent.plugin",
    "mount": "parent.plugin.children",
    "compatibleParentRange": "^1.0.0"
  }
}"#,
        )
        .expect("child formula");
        fs::write(
            grandchild.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "parent.plugin.child.grandchild",
  "kind": "child-plugin",
  "displayName": "Grandchild Plugin",
  "parent": {
    "pluginId": "parent.plugin.child",
    "mount": "parent.plugin.child.children",
    "compatibleParentRange": "^1.0.0"
  }
}"#,
        )
        .expect("grandchild formula");

        let store = PluginRegistryStore::new(tempdir.path());
        store.install(&parent).expect("install parent");
        store.install(&child).expect("install child");
        store.install(&grandchild).expect("install grandchild");

        store
            .set_enabled("parent.plugin", false)
            .expect("disable parent");
        let state = store.state().expect("disabled state");
        assert!(
            !state
                .installed
                .iter()
                .find(|entry| entry.id == "parent.plugin")
                .expect("parent installed")
                .enabled
        );
        assert!(
            !state
                .installed
                .iter()
                .find(|entry| entry.id == "parent.plugin.child")
                .expect("child installed")
                .enabled
        );
        assert!(
            !state
                .installed
                .iter()
                .find(|entry| entry.id == "parent.plugin.child.grandchild")
                .expect("grandchild installed")
                .enabled
        );
        assert!(state
            .locks
            .iter()
            .filter(|entry| entry.id.starts_with("parent.plugin"))
            .all(|entry| !entry.enabled));
        assert!(
            state
                .audits
                .iter()
                .any(|record| record.action == "disable-cascade"
                    && record.id == "parent.plugin.child")
        );
        assert!(state
            .audits
            .iter()
            .any(|record| record.action == "disable-cascade"
                && record.id == "parent.plugin.child.grandchild"));

        let error = store
            .set_enabled("parent.plugin.child", true)
            .expect_err("disabled parent should block child enable");
        assert!(matches!(error, AppError::Conflict(_)));

        store
            .set_enabled("parent.plugin", true)
            .expect("enable parent");
        store
            .set_enabled("parent.plugin.child", true)
            .expect("enable child after parent");
        let state = store.state().expect("reenabled state");
        assert!(
            state
                .installed
                .iter()
                .find(|entry| entry.id == "parent.plugin.child")
                .expect("child installed")
                .enabled
        );
        assert!(
            !state
                .installed
                .iter()
                .find(|entry| entry.id == "parent.plugin.child.grandchild")
                .expect("grandchild installed")
                .enabled
        );
    }

    #[test]
    fn rollback_should_restore_previous_history_entry() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source = tempdir.path().join("source-plugin");
        fs::create_dir_all(&source).expect("source dir");
        fs::write(
            source.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "test.rollback",
  "kind": "plugin",
  "displayName": "Rollback Plugin",
  "version": "0.1.0"
}"#,
        )
        .expect("formula v1");

        let store = PluginRegistryStore::new(tempdir.path());
        let first = store.install(&source).expect("install first");
        fs::write(
            source.join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "test.rollback",
  "kind": "plugin",
  "displayName": "Rollback Plugin",
  "version": "0.2.0"
}"#,
        )
        .expect("formula v2");
        let second = store.install(&source).expect("install second");
        assert_ne!(first.content_hash, second.content_hash);

        let result = store
            .rollback(super::PluginRegistryRollbackInput {
                id: "test.rollback".to_string(),
                content_hash: Some(first.content_hash.clone()),
            })
            .expect("rollback");

        assert_eq!(result.restored.content_hash, first.content_hash);
        assert_eq!(result.previous_content_hash, second.content_hash);
        assert!(fs::read_to_string(source.join("formula.json"))
            .expect("source remains readable")
            .contains("0.2.0"));
        let active_formula = fs::read_to_string(
            tempdir
                .path()
                .join("plugin-registry/plugins/test.rollback/formula.json"),
        )
        .expect("active formula");
        assert!(active_formula.contains("0.1.0"));
    }

    #[test]
    fn child_capability_approvals_can_be_approved_and_revoked() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let store = PluginRegistryStore::new(tempdir.path());

        let approved = store
            .approve_child_capability(ChildCapabilityApprovalInput {
                capability: "process.exec".to_string(),
                child_plugin_id: "git-suite.github-provider".to_string(),
                parent_plugin_id: "git-suite".to_string(),
                reason: "allow git process bridge".to_string(),
            })
            .expect("approve");
        assert_eq!(approved.status, "approved");
        assert_eq!(
            store
                .state()
                .expect("state")
                .child_capability_approvals
                .len(),
            1
        );

        let revoked = store
            .revoke_child_capability(ChildCapabilityApprovalInput {
                capability: "process.exec".to_string(),
                child_plugin_id: "git-suite.github-provider".to_string(),
                parent_plugin_id: "git-suite".to_string(),
                reason: "remove git process bridge".to_string(),
            })
            .expect("revoke");
        assert_eq!(revoked.status, "revoked");
        assert_eq!(
            store.state().expect("state").child_capability_approvals[0].status,
            "revoked"
        );
    }
}
