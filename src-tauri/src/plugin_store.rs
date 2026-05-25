use std::{
    collections::HashMap,
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
        let mut installed = self.load_installed()?;
        let now = now_millis();
        let Some(index) = installed.iter().position(|entry| entry.id == id) else {
            return Err(AppError::NotFound);
        };
        let record = &mut installed[index];
        record.enabled = enabled;
        record.updated_at = now;
        let record = record.clone();
        save_json_list(&self.installed_path(), &installed)?;

        let mut locks = self.load_lock()?;
        if let Some(lock) = locks.iter_mut().find(|entry| entry.id == id) {
            lock.enabled = enabled;
            lock.updated_at = now;
        }
        save_json_list(&self.lock_path(), &locks)?;

        self.append_audit(RegistryAuditRecord {
            action: if enabled {
                "enable".to_string()
            } else {
                "disable".to_string()
            },
            content_hash: Some(record.content_hash.clone()),
            detail: None,
            id: record.id.clone(),
            path: Some(record.registry_path.clone()),
            status: "ok".to_string(),
            timestamp: now,
        })?;

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
    use super::PluginRegistryStore;
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
}
