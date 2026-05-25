use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
    plugin_registry::{PluginEntry, PluginFormula},
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionHostSourceInput {
    pub source_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionHostPluginRecord {
    pub plugin_id: String,
    pub display_name: String,
    pub host_kind: String,
    pub entry_path: String,
    pub source_path: String,
    pub state: String,
    pub loaded_at: i64,
    pub activated_at: Option<i64>,
    pub deactivated_at: Option<i64>,
    pub disposed_at: Option<i64>,
    pub reload_count: u32,
    pub last_error: Option<String>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ExtensionHostRuntime {
    records: Arc<Mutex<HashMap<String, ExtensionHostPluginRecord>>>,
}

impl ExtensionHostRuntime {
    pub fn load(&self, source_path: impl AsRef<Path>) -> AppResult<ExtensionHostPluginRecord> {
        let source_path = normalize_source_path(source_path.as_ref())?;
        let formula = read_formula(&source_path)?;
        let loaded_at = now_millis();
        let (host_kind, entry_path) = resolve_entry(&source_path, formula.entry.as_ref())?;
        let mut record = ExtensionHostPluginRecord {
            plugin_id: formula.id,
            display_name: formula.display_name,
            host_kind,
            entry_path,
            source_path: source_path.to_string_lossy().into_owned(),
            state: "loaded".to_string(),
            loaded_at,
            activated_at: None,
            deactivated_at: None,
            disposed_at: None,
            reload_count: 0,
            last_error: None,
            logs: vec![format!("loaded at {loaded_at}")],
        };
        if record.host_kind == "metadata-only" {
            record
                .logs
                .push("loaded without executable entry".to_string());
        }

        self.with_records(|records| {
            records.insert(record.plugin_id.clone(), record.clone());
            Ok(record)
        })
    }

    pub fn activate(&self, plugin_id: &str) -> AppResult<ExtensionHostPluginRecord> {
        self.transition(plugin_id, "activated", |record, now| {
            if record.state == "disposed" {
                return Err(AppError::Conflict(format!(
                    "plugin {} has been disposed",
                    record.plugin_id
                )));
            }
            run_lifecycle_hook(record, "activate")?;
            record.state = "activated".to_string();
            record.activated_at = Some(now);
            record.disposed_at = None;
            record.last_error = None;
            record.logs.push(format!("activated at {now}"));
            Ok(())
        })
    }

    pub fn deactivate(&self, plugin_id: &str) -> AppResult<ExtensionHostPluginRecord> {
        self.transition(plugin_id, "deactivated", |record, now| {
            if record.state == "disposed" {
                return Err(AppError::Conflict(format!(
                    "plugin {} has been disposed",
                    record.plugin_id
                )));
            }
            run_lifecycle_hook(record, "deactivate")?;
            record.state = "deactivated".to_string();
            record.deactivated_at = Some(now);
            record.logs.push(format!("deactivated at {now}"));
            Ok(())
        })
    }

    pub fn reload(&self, source_path: impl AsRef<Path>) -> AppResult<ExtensionHostPluginRecord> {
        let source_path = normalize_source_path(source_path.as_ref())?;
        let formula = read_formula(&source_path)?;
        let plugin_id = formula.id.clone();
        let (host_kind, entry_path) = resolve_entry(&source_path, formula.entry.as_ref())?;
        self.with_records(|records| {
            let now = now_millis();
            let reload_count = records
                .get(&plugin_id)
                .map(|record| record.reload_count.saturating_add(1))
                .unwrap_or(1);
            let mut logs = records
                .get(&plugin_id)
                .map(|record| record.logs.clone())
                .unwrap_or_default();
            logs.push(format!("reloaded at {now}"));
            let record = ExtensionHostPluginRecord {
                plugin_id: plugin_id.clone(),
                display_name: formula.display_name,
                host_kind,
                entry_path,
                source_path: source_path.to_string_lossy().into_owned(),
                state: "loaded".to_string(),
                loaded_at: now,
                activated_at: None,
                deactivated_at: None,
                disposed_at: None,
                reload_count,
                last_error: None,
                logs,
            };
            records.insert(plugin_id, record.clone());
            Ok(record)
        })
    }

    pub fn dispose(&self, plugin_id: &str) -> AppResult<ExtensionHostPluginRecord> {
        self.transition(plugin_id, "disposed", |record, now| {
            if record.state != "disposed" {
                run_lifecycle_hook(record, "dispose")?;
            }
            record.state = "disposed".to_string();
            record.disposed_at = Some(now);
            record.logs.push(format!("disposed at {now}"));
            record.logs.push("resources cleaned".to_string());
            Ok(())
        })
    }

    pub fn snapshot(&self) -> Vec<ExtensionHostPluginRecord> {
        self.records
            .lock()
            .map(|records| {
                let mut entries = records.values().cloned().collect::<Vec<_>>();
                entries.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
                entries
            })
            .unwrap_or_default()
    }

    pub fn lifecycle_cycle(
        &self,
        source_path: impl AsRef<Path>,
    ) -> AppResult<Vec<ExtensionHostPluginRecord>> {
        let loaded = self.load(source_path.as_ref())?;
        let activated = self.activate(&loaded.plugin_id)?;
        let deactivated = self.deactivate(&loaded.plugin_id)?;
        let reloaded = self.reload(source_path.as_ref())?;
        let disposed = self.dispose(&loaded.plugin_id)?;
        Ok(vec![loaded, activated, deactivated, reloaded, disposed])
    }

    fn transition<F>(
        &self,
        plugin_id: &str,
        target_state: &str,
        transition: F,
    ) -> AppResult<ExtensionHostPluginRecord>
    where
        F: FnOnce(&mut ExtensionHostPluginRecord, i64) -> AppResult<()>,
    {
        self.with_records(|records| {
            let now = now_millis();
            let Some(record) = records.get_mut(plugin_id) else {
                return Err(AppError::NotFound);
            };
            if let Err(error) = transition(record, now) {
                record.state = "error".to_string();
                record.last_error = Some(error.to_string());
                record
                    .logs
                    .push(format!("{target_state} failed at {now}: {error}"));
                return Err(error);
            }
            Ok(record.clone())
        })
    }

    fn with_records<T, F>(&self, action: F) -> AppResult<T>
    where
        F: FnOnce(&mut HashMap<String, ExtensionHostPluginRecord>) -> AppResult<T>,
    {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AppError::Conflict("extension host lock poisoned".to_string()))?;
        action(&mut records)
    }
}

fn normalize_source_path(path: &Path) -> AppResult<PathBuf> {
    if !path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "extension host source is not a directory: {}",
            path.display()
        )));
    }
    fs::canonicalize(path).map_err(AppError::from)
}

fn read_formula(source_path: &Path) -> AppResult<PluginFormula> {
    let formula_path = source_path.join("formula.json");
    if !formula_path.is_file() {
        return Err(AppError::BadRequest(format!(
            "extension host source missing formula.json: {}",
            source_path.display()
        )));
    }
    let value = fs::read_to_string(&formula_path)?;
    serde_json::from_str(&value).map_err(|source| {
        AppError::BadRequest(format!("解析 {} 失败：{source}", formula_path.display()))
    })
}

fn resolve_entry(source_path: &Path, entry: Option<&PluginEntry>) -> AppResult<(String, String)> {
    let Some(entry) = entry else {
        return Ok((
            "metadata-only".to_string(),
            source_path.to_string_lossy().into_owned(),
        ));
    };
    for (host_kind, relative) in [
        ("node", entry.node.as_str()),
        ("worker", entry.worker.as_str()),
        ("browser", entry.browser.as_str()),
    ] {
        if relative.trim().is_empty() {
            continue;
        }
        let path = source_path.join(relative.trim_start_matches("./"));
        if !path.is_file() {
            return Err(AppError::BadRequest(format!(
                "{host_kind} entry does not exist: {}",
                path.display()
            )));
        }
        return Ok((host_kind.to_string(), path.to_string_lossy().into_owned()));
    }
    let remote = entry.remote.trim();
    if !remote.is_empty() {
        if !is_remote_url(remote) {
            return Err(AppError::BadRequest(format!(
                "remote entry must be an http(s) URL: {remote}"
            )));
        }
        return Ok(("remote".to_string(), remote.to_string()));
    }
    Ok((
        "metadata-only".to_string(),
        source_path.to_string_lossy().into_owned(),
    ))
}

fn run_lifecycle_hook(record: &mut ExtensionHostPluginRecord, action: &str) -> AppResult<()> {
    if record.host_kind == "metadata-only" {
        record
            .logs
            .push(format!("{action} skipped for metadata-only host"));
        return Ok(());
    }

    let script = if is_worker_host(&record.host_kind) {
        worker_lifecycle_runner()
    } else if record.host_kind == "remote" {
        return run_remote_lifecycle_hook(record, action);
    } else {
        node_lifecycle_runner()
    };
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .arg("--")
        .arg(action)
        .arg(&record.entry_path)
        .arg(&record.plugin_id)
        .current_dir(&record.source_path)
        .output()
        .map_err(|source| {
            AppError::BadRequest(format!(
                "node lifecycle runner failed for {}: {source}",
                record.plugin_id
            ))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stdout.is_empty() {
        record.logs.push(format!("{action} stdout: {stdout}"));
    }
    if !stderr.is_empty() {
        record.logs.push(format!("{action} stderr: {stderr}"));
    }
    if output.status.success() {
        record.logs.push(format!("{action} hook completed"));
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "{action} hook failed for {} with status {}",
            record.plugin_id, output.status
        )))
    }
}

fn is_worker_host(host_kind: &str) -> bool {
    matches!(host_kind, "worker" | "browser")
}

fn run_remote_lifecycle_hook(
    record: &mut ExtensionHostPluginRecord,
    action: &str,
) -> AppResult<()> {
    let url = remote_lifecycle_url(&record.entry_path, action)?;
    let response = reqwest::blocking::Client::new()
        .post(&url)
        .json(&json!({
            "action": action,
            "pluginId": record.plugin_id,
            "sourcePath": record.source_path,
        }))
        .send()
        .map_err(|source| {
            AppError::BadRequest(format!(
                "remote lifecycle request failed for {}: {source}",
                record.plugin_id
            ))
        })?;
    let status = response.status();
    let body = response.text().map_err(|source| {
        AppError::BadRequest(format!(
            "remote lifecycle response read failed for {}: {source}",
            record.plugin_id
        ))
    })?;
    if !body.trim().is_empty() {
        record
            .logs
            .push(format!("{action} remote: {}", body.trim()));
    }
    if status.is_success() {
        record.logs.push(format!("{action} hook completed"));
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "{action} remote hook failed for {} with status {}",
            record.plugin_id, status
        )))
    }
}

fn remote_lifecycle_url(base: &str, action: &str) -> AppResult<String> {
    if !is_remote_url(base) {
        return Err(AppError::BadRequest(format!(
            "remote entry must be an http(s) URL: {base}"
        )));
    }
    let base = base.trim_end_matches('/');
    Ok(format!("{base}/lifecycle/{action}"))
}

fn is_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn node_lifecycle_runner() -> &'static str {
    r#"
import { readFile } from 'node:fs/promises';

const [, action, entryPath, pluginId] = process.argv;

function toRunnableModule(source) {
  return source
    .replace(/export\s+interface\s+\w+\s*\{[\s\S]*?\}\s*/g, '')
    .replace(/\(\s*context\s*:\s*[A-Za-z0-9_<>,\s]+\s*=\s*\{\s*\}\s*\)/g, '(context = {})');
}

const source = await readFile(entryPath, 'utf8');
const transformed = entryPath.endsWith('.ts') ? toRunnableModule(source) : source;
const encoded = Buffer.from(transformed, 'utf8').toString('base64');
const mod = await import(`data:text/javascript;base64,${encoded}`);
const logs = [];
const context = {
  pluginId,
  log(message) {
    logs.push(String(message));
  }
};

if (typeof mod[action] === 'function') {
  const result = await mod[action](context);
  console.log(JSON.stringify({ action, pluginId, logs, result: result ?? null }));
} else {
  console.log(JSON.stringify({ action, pluginId, logs, skipped: true }));
}
"#
}

fn worker_lifecycle_runner() -> &'static str {
    r#"
import { Worker } from 'node:worker_threads';

const [, action, entryPath, pluginId] = process.argv;

function runWorker(task) {
  return new Promise((resolve, reject) => {
    const workerSource = `
import { parentPort, workerData } from 'node:worker_threads';
import { readFile } from 'node:fs/promises';

function toRunnableModule(source) {
  return source
    .replace(/export\\s+interface\\s+\\w+\\s*\\{[\\s\\S]*?\\}\\s*/g, '')
    .replace(/\\(\\s*context\\s*:\\s*[A-Za-z0-9_<>,\\s]+\\s*=\\s*\\{\\s*\\}\\s*\\)/g, '(context = {})');
}

try {
  const { action, entryPath, pluginId } = workerData;
  const source = await readFile(entryPath, 'utf8');
  const transformed = entryPath.endsWith('.ts') ? toRunnableModule(source) : source;
  const encoded = Buffer.from(transformed, 'utf8').toString('base64');
  const mod = await import(\`data:text/javascript;base64,\${encoded}\`);
  const logs = [];
  const context = {
    pluginId,
    hostKind: 'worker',
    log(message) {
      logs.push(String(message));
    }
  };

  if (typeof mod[action] === 'function') {
    const result = await mod[action](context);
    parentPort.postMessage({ action, pluginId, logs, result: result ?? null });
  } else {
    parentPort.postMessage({ action, pluginId, logs, skipped: true });
  }
} catch (error) {
  parentPort.postMessage({ action: workerData.action, pluginId: workerData.pluginId, error: String(error) });
}
`;
    const worker = new Worker(new URL('data:text/javascript,' + encodeURIComponent(workerSource)), {
      type: 'module',
      workerData: task,
    });

    worker.once('message', (message) => {
      resolve(message);
      worker.terminate().catch(() => {});
    });
    worker.once('error', reject);
    worker.once('exit', (code) => {
      if (code !== 0) {
        reject(new Error(`worker exited with code ${code}`));
      }
    });
  });
}

const message = await runWorker({ action, entryPath, pluginId });
if (message?.error) {
  throw new Error(message.error);
}
console.log(JSON.stringify(message));
"#
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex},
        thread,
    };

    use super::ExtensionHostRuntime;

    #[test]
    fn lifecycle_cycle_should_track_all_states() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src dir");
        fs::write(
            tempdir.path().join("src/index.ts"),
            "export async function activate(context = {}) { context.log?.('activated hook'); }\nexport async function deactivate(context = {}) { context.log?.('deactivated hook'); }\nexport async function dispose(context = {}) { context.log?.('disposed hook'); }\n",
        )
        .expect("entry");
        fs::write(
            tempdir.path().join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.host-cycle",
  "kind": "plugin",
  "displayName": "Host Cycle",
  "entry": {
    "node": "./src/index.ts"
  }
}"#,
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let records = host.lifecycle_cycle(tempdir.path()).expect("cycle");

        assert_eq!(records.len(), 5);
        assert_eq!(records[0].state, "loaded");
        assert_eq!(records[1].state, "activated");
        assert!(records[1]
            .logs
            .iter()
            .any(|log| log.contains("activated hook")));
        assert_eq!(records[2].state, "deactivated");
        assert_eq!(records[3].reload_count, 1);
        assert_eq!(records[4].state, "disposed");
    }

    #[test]
    fn generated_typescript_stub_should_execute_node_lifecycle_hooks() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src dir");
        fs::write(
            tempdir.path().join("src/index.ts"),
            r#"export interface PluginContext {
  log?: (message: string) => void;
}

export const pluginId = "workspace.generated-node";
export const displayName = "Generated Node";
export const commands = ["workspace.generated-node.run"];

export async function activate(context: PluginContext = {}) {
  context.log?.(`${displayName} activated`);
  return { pluginId, commands };
}

export async function deactivate(context: PluginContext = {}) {
  context.log?.(`${displayName} deactivated`);
}

export async function dispose(context: PluginContext = {}) {
  context.log?.(`${displayName} disposed`);
}
"#,
        )
        .expect("entry");
        fs::write(
            tempdir.path().join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.generated-node",
  "kind": "plugin",
  "displayName": "Generated Node",
  "entry": {
    "node": "./src/index.ts"
  }
}"#,
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let loaded = host.load(tempdir.path()).expect("load");
        let activated = host.activate(&loaded.plugin_id).expect("activate");

        assert!(activated
            .logs
            .iter()
            .any(|log| log.contains("Generated Node activated")));
        let disposed = host.dispose(&loaded.plugin_id).expect("dispose");
        assert!(disposed
            .logs
            .iter()
            .any(|log| log.contains("Generated Node disposed")));
    }

    #[test]
    fn worker_lifecycle_cycle_should_execute_worker_hooks() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src dir");
        fs::write(
            tempdir.path().join("src/worker.ts"),
            r#"export interface PluginContext {
  log?: (message: string) => void;
}

export const pluginId = "workspace.worker-demo";
export const displayName = "Worker Demo";

export async function activate(context: PluginContext = {}) {
  context.log?.(`${displayName} activated`);
}

export async function deactivate(context: PluginContext = {}) {
  context.log?.(`${displayName} deactivated`);
}

export async function dispose(context: PluginContext = {}) {
  context.log?.(`${displayName} disposed`);
}
"#,
        )
        .expect("entry");
        fs::write(
            tempdir.path().join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.worker-demo",
  "kind": "plugin",
  "displayName": "Worker Demo",
  "entry": {
    "worker": "./src/worker.ts"
  }
}"#,
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let records = host.lifecycle_cycle(tempdir.path()).expect("cycle");

        assert_eq!(records[0].host_kind, "worker");
        assert!(records[1]
            .logs
            .iter()
            .any(|log| log.contains("Worker Demo activated")));
        assert!(records[4]
            .logs
            .iter()
            .any(|log| log.contains("Worker Demo disposed")));
    }

    #[test]
    fn remote_lifecycle_cycle_should_call_remote_hooks() {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let received_for_thread = Arc::clone(&received);
        let server = thread::spawn(move || {
            for stream in listener.incoming().take(3) {
                let mut stream = stream.expect("stream");
                let request = read_http_request(&mut stream);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or_default()
                    .to_string();
                received_for_thread
                    .lock()
                    .expect("received")
                    .push(path.clone());
                let action = path.rsplit('/').next().unwrap_or("unknown");
                let body = format!(r#"{{"ok":true,"action":"{action}"}}"#);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("response");
            }
        });

        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::write(
            tempdir.path().join("formula.json"),
            format!(
                r#"{{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.remote-demo",
  "kind": "plugin",
  "displayName": "Remote Demo",
  "entry": {{
    "remote": "http://{address}"
  }}
}}"#
            ),
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let records = host.lifecycle_cycle(tempdir.path()).expect("cycle");
        server.join().expect("server");

        assert_eq!(records[0].host_kind, "remote");
        assert_eq!(records[0].entry_path, format!("http://{address}"));
        assert!(records[1]
            .logs
            .iter()
            .any(|log| log.contains(r#""action":"activate""#)));
        assert!(records[4]
            .logs
            .iter()
            .any(|log| log.contains(r#""action":"dispose""#)));
        assert_eq!(
            received.lock().expect("received").as_slice(),
            [
                "/lifecycle/activate",
                "/lifecycle/deactivate",
                "/lifecycle/dispose"
            ]
        );
    }

    #[test]
    fn load_should_reject_missing_entry() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::write(
            tempdir.path().join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.missing-entry",
  "kind": "plugin",
  "displayName": "Missing Entry",
  "entry": {
    "node": "./src/index.ts"
  }
}"#,
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let error = host.load(tempdir.path()).expect_err("missing entry");

        assert!(error.to_string().contains("entry does not exist"));
    }

    #[test]
    fn load_should_reject_remote_entry_without_http_url() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::write(
            tempdir.path().join("formula.json"),
            r#"{
  "schemaVersion": "plugin-formula/v1",
  "id": "workspace.invalid-remote",
  "kind": "plugin",
  "displayName": "Invalid Remote",
  "entry": {
    "remote": "./remote"
  }
}"#,
        )
        .expect("formula");

        let host = ExtensionHostRuntime::default();
        let error = host.load(tempdir.path()).expect_err("invalid remote");

        assert!(error
            .to_string()
            .contains("remote entry must be an http(s) URL"));
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        loop {
            let read = stream.read(&mut buffer).expect("read");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }
}
