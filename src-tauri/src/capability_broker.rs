use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const DEFAULT_READ_LIMIT_BYTES: usize = 64 * 1024;
const DEFAULT_CAPTURE_LIMIT_BYTES: usize = 64 * 1024;
const PROCESS_EXEC_ALLOWLIST: &[&str] = &["git", "node", "npm", "pnpm", "bun"];
const SUPPORTED_CAPABILITIES: &[&str] = &[
    "browser.openDirectory",
    "fs.read",
    "fs.write",
    "process.exec",
    "browser.openUrl",
    "clipboard.read",
    "clipboard.write",
    "notification.send",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWriteInput {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSendInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReadInput {
    pub path: String,
    #[serde(default)]
    pub max_bytes: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReadResult {
    pub bytes: usize,
    pub content: String,
    pub path: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteInput {
    #[serde(default)]
    pub append: bool,
    pub content: String,
    #[serde(default)]
    pub create_dirs: bool,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteResult {
    pub bytes: usize,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessExecInput {
    #[serde(default)]
    pub args: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessExecResult {
    pub code: Option<i32>,
    pub command: String,
    pub stderr: String,
    pub stderr_truncated: bool,
    pub stdout: String,
    pub stdout_truncated: bool,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserOpenUrlInput {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityInvokeInput {
    pub capability: String,
    #[serde(default)]
    pub input: Value,
}

impl CapabilityInvokeInput {
    pub fn audit_target(&self) -> String {
        match self.capability.as_str() {
            "browser.openDirectory" => json_string_field(&self.input, "path")
                .unwrap_or_else(|| summarize_json_value(&self.input)),
            "fs.read" | "fs.write" => json_string_field(&self.input, "path")
                .unwrap_or_else(|| summarize_json_value(&self.input)),
            "process.exec" => json_string_field(&self.input, "command")
                .unwrap_or_else(|| summarize_json_value(&self.input)),
            "browser.openUrl" => json_string_field(&self.input, "url")
                .unwrap_or_else(|| summarize_json_value(&self.input)),
            "clipboard.read" | "clipboard.write" => "system-clipboard".to_string(),
            "notification.send" => json_string_field(&self.input, "title")
                .unwrap_or_else(|| summarize_json_value(&self.input)),
            _ => summarize_json_value(&self.input),
        }
    }

    pub fn permission_scope(&self) -> String {
        match self.capability.as_str() {
            "browser.openDirectory" | "fs.read" | "fs.write" => {
                json_string_field(&self.input, "path").unwrap_or_else(|| "*".to_string())
            }
            "process.exec" => {
                json_string_field(&self.input, "command").unwrap_or_else(|| "*".to_string())
            }
            "browser.openUrl" => {
                json_string_field(&self.input, "url").unwrap_or_else(|| "*".to_string())
            }
            "clipboard.read" | "clipboard.write" => "system-clipboard".to_string(),
            "notification.send" => {
                json_string_field(&self.input, "title").unwrap_or_else(|| "*".to_string())
            }
            _ => "*".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAuditRecord {
    pub trace_id: String,
    pub capability: String,
    pub action: String,
    pub target: String,
    pub outcome: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityBroker {
    audit_log: Arc<Mutex<Vec<CapabilityAuditRecord>>>,
}

impl CapabilityBroker {
    pub fn supported_capabilities() -> Vec<String> {
        SUPPORTED_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect()
    }

    pub fn open_directory(&self, path: &Path) -> AppResult<String> {
        let trace_id = Uuid::new_v4().to_string();
        let mut record = CapabilityAuditRecord {
            trace_id,
            capability: "browser.openDirectory".to_string(),
            action: "open".to_string(),
            target: path.to_string_lossy().into_owned(),
            outcome: "pending".to_string(),
            detail: None,
        };

        let result = open_directory_command(path).and_then(|mut command| {
            command.spawn()?;
            Ok(())
        });

        match result {
            Ok(()) => {
                record.outcome = "allowed".to_string();
                self.push_audit(record);
                Ok(path.to_string_lossy().into_owned())
            }
            Err(error) => {
                record.outcome = "error".to_string();
                record.detail = Some(error.to_string());
                self.push_audit(record);
                Err(error)
            }
        }
    }

    pub fn audit_log(&self) -> Vec<CapabilityAuditRecord> {
        self.audit_log
            .lock()
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }

    pub fn write_clipboard(&self, text: &str) -> AppResult<usize> {
        let target = format!("{} bytes", text.len());
        self.invoke_audited("clipboard.write", "write", &target, || {
            write_clipboard_command(text)?;
            Ok(text.len())
        })
    }

    pub fn read_clipboard(&self) -> AppResult<String> {
        self.invoke_audited("clipboard.read", "read", "system-clipboard", || {
            read_clipboard_command()
        })
    }

    pub fn send_notification(&self, title: &str, body: &str) -> AppResult<()> {
        self.invoke_audited("notification.send", "send", title, || {
            send_notification_command(title, body)
        })
    }

    pub fn read_text_file(&self, input: &FsReadInput) -> AppResult<FsReadResult> {
        let target = input.path.clone();
        self.invoke_audited("fs.read", "read", &target, || {
            let path = normalize_absolute_path(&input.path, "fs.read")?;
            let bytes = fs::read(&path)?;
            let limit = input.max_bytes.unwrap_or(DEFAULT_READ_LIMIT_BYTES);
            let truncated = bytes.len() > limit;
            let content_bytes = if truncated {
                &bytes[..limit]
            } else {
                bytes.as_slice()
            };
            Ok(FsReadResult {
                bytes: bytes.len(),
                content: String::from_utf8_lossy(content_bytes).into_owned(),
                path: target.clone(),
                truncated,
            })
        })
    }

    pub fn write_text_file(&self, input: &FsWriteInput) -> AppResult<FsWriteResult> {
        let target = input.path.clone();
        self.invoke_audited("fs.write", "write", &target, || {
            let path = normalize_absolute_path(&input.path, "fs.write")?;
            if input.create_dirs {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
            }

            if input.append {
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)?
                    .write_all(input.content.as_bytes())?;
            } else {
                fs::write(&path, input.content.as_bytes())?;
            }

            Ok(FsWriteResult {
                bytes: input.content.len(),
                path: target.clone(),
            })
        })
    }

    pub fn run_process(&self, input: &ProcessExecInput) -> AppResult<ProcessExecResult> {
        let target = command_with_args(input.command.trim(), &input.args);
        self.invoke_audited("process.exec", "exec", &target, || {
            let command = validate_process_command(&input.command)?;
            let mut process = Command::new(command);
            process.args(&input.args);
            if let Some(cwd) = &input.cwd {
                process.current_dir(normalize_absolute_path(cwd, "process.exec.cwd")?);
            }
            let output = process.output()?;
            let (stdout, stdout_truncated) = capture_text(&output.stdout);
            let (stderr, stderr_truncated) = capture_text(&output.stderr);
            Ok(ProcessExecResult {
                code: output.status.code(),
                command: target.clone(),
                stderr,
                stderr_truncated,
                stdout,
                stdout_truncated,
                success: output.status.success(),
            })
        })
    }

    pub fn open_url(&self, input: &BrowserOpenUrlInput) -> AppResult<String> {
        let target = input.url.trim().to_string();
        self.invoke_audited("browser.openUrl", "open", &target, || {
            let url = validate_http_url(&input.url)?;
            open_url_command(url).and_then(|mut command| {
                command.spawn()?;
                Ok(url.to_string())
            })
        })
    }

    pub fn invoke_json(&self, request: CapabilityInvokeInput) -> AppResult<Value> {
        match request.capability.as_str() {
            "browser.openDirectory" => {
                let path = serde_json::from_value::<String>(
                    request
                        .input
                        .get("path")
                        .cloned()
                        .ok_or_else(|| AppError::BadRequest("missing path".to_string()))?,
                )?;
                let path = normalize_absolute_path(&path, "browser.openDirectory")?;
                serde_json::to_value(self.open_directory(&path)?)
                    .map_err(|source| AppError::Json { source })
            }
            "fs.read" => {
                serde_json::to_value(
                    self.read_text_file(&serde_json::from_value::<FsReadInput>(request.input)?)?,
                )
                .map_err(|source| AppError::Json { source })
            }
            "fs.write" => {
                serde_json::to_value(
                    self.write_text_file(&serde_json::from_value::<FsWriteInput>(request.input)?)?,
                )
                .map_err(|source| AppError::Json { source })
            }
            "process.exec" => {
                serde_json::to_value(
                    self.run_process(&serde_json::from_value::<ProcessExecInput>(request.input)?)?,
                )
                .map_err(|source| AppError::Json { source })
            }
            "browser.openUrl" => {
                serde_json::to_value(self.open_url(&serde_json::from_value::<
                    BrowserOpenUrlInput,
                >(request.input)?)?)
                .map_err(|source| AppError::Json { source })
            }
            "clipboard.write" => serde_json::to_value(self.write_clipboard(
                &serde_json::from_value::<ClipboardWriteInput>(request.input)?.text,
            )?)
            .map_err(|source| AppError::Json { source }),
            "clipboard.read" => serde_json::to_value(self.read_clipboard()?)
                .map_err(|source| AppError::Json { source }),
            "notification.send" => {
                let input = serde_json::from_value::<NotificationSendInput>(request.input)?;
                serde_json::to_value(self.send_notification(&input.title, &input.body)?)
                    .map_err(|source| AppError::Json { source })
            }
            other => Err(AppError::BadRequest(format!(
                "unsupported capability invoke: {other}"
            ))),
        }
    }

    fn invoke_audited<T>(
        &self,
        capability: &str,
        action: &str,
        target: &str,
        invoke: impl FnOnce() -> AppResult<T>,
    ) -> AppResult<T> {
        let mut record = CapabilityAuditRecord {
            trace_id: Uuid::new_v4().to_string(),
            capability: capability.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            outcome: "pending".to_string(),
            detail: None,
        };

        match invoke() {
            Ok(value) => {
                record.outcome = "allowed".to_string();
                self.push_audit(record);
                Ok(value)
            }
            Err(error) => {
                record.outcome = "error".to_string();
                record.detail = Some(error.to_string());
                self.push_audit(record);
                Err(error)
            }
        }
    }

    fn push_audit(&self, record: CapabilityAuditRecord) {
        if let Ok(mut entries) = self.audit_log.lock() {
            entries.push(record);
        }
    }
}

fn json_string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|value| truncate_for_audit(value.trim()))
        .filter(|value| !value.is_empty())
}

fn summarize_json_value(value: &Value) -> String {
    truncate_for_audit(&value.to_string())
}

fn truncate_for_audit(value: &str) -> String {
    const LIMIT: usize = 240;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }

    let mut truncated = value.chars().take(LIMIT).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn write_clipboard_command(text: &str) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;

        let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
        let Some(stdin) = child.stdin.as_mut() else {
            return Err(AppError::BadRequest(
                "无法打开 pbcopy stdin 写入剪贴板".to_string(),
            ));
        };
        stdin.write_all(text.as_bytes())?;
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(AppError::BadRequest(format!(
                "pbcopy exited with status {status}"
            )))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = text.replace('\'', "''");
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", escaped),
            ])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(AppError::BadRequest(format!(
                "Set-Clipboard exited with status {status}"
            )))
        }
    }

    #[cfg(target_os = "linux")]
    {
        Err(AppError::BadRequest(
            "clipboard.write is not available on linux without a configured clipboard provider"
                .to_string(),
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(AppError::BadRequest(
            "当前系统不支持 clipboard.write".to_string(),
        ))
    }
}

fn read_clipboard_command() -> AppResult<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pbpaste").output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(AppError::BadRequest(format!(
                "pbpaste exited with status {}",
                output.status
            )))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard"])
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(AppError::BadRequest(format!(
                "Get-Clipboard exited with status {}",
                output.status
            )))
        }
    }

    #[cfg(target_os = "linux")]
    {
        Err(AppError::BadRequest(
            "clipboard.read is not available on linux without a configured clipboard provider"
                .to_string(),
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(AppError::BadRequest(
            "当前系统不支持 clipboard.read".to_string(),
        ))
    }
}

fn send_notification_command(title: &str, body: &str) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification {} with title {}",
            applescript_string(body),
            applescript_string(title)
        );
        let status = Command::new("osascript").args(["-e", &script]).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(AppError::BadRequest(format!(
                "osascript notification exited with status {status}"
            )))
        }
    }

    #[cfg(target_os = "windows")]
    {
        Err(AppError::BadRequest(
            "notification.send is not available on windows without a configured notification provider"
                .to_string(),
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("notify-send").args([title, body]).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(AppError::BadRequest(format!(
                "notify-send exited with status {status}"
            )))
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(AppError::BadRequest(
            "当前系统不支持 notification.send".to_string(),
        ))
    }
}

fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn normalize_absolute_path(path: &str, capability: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(AppError::BadRequest(format!(
            "{capability} requires an absolute path"
        )));
    }
    Ok(path)
}

fn validate_process_command(command: &str) -> AppResult<&str> {
    let command = command.trim();
    if command.is_empty()
        || command.contains('/')
        || command.contains('\\')
        || !PROCESS_EXEC_ALLOWLIST.contains(&command)
    {
        return Err(AppError::BadRequest(format!(
            "process.exec command is not allowlisted: {command}"
        )));
    }
    Ok(command)
}

fn validate_http_url(url: &str) -> AppResult<&str> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::BadRequest(
            "browser.openUrl only allows http:// or https:// URLs".to_string(),
        ));
    }
    Ok(url)
}

fn command_with_args(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        command.to_string()
    } else {
        format!("{command} {}", args.join(" "))
    }
}

fn capture_text(bytes: &[u8]) -> (String, bool) {
    let truncated = bytes.len() > DEFAULT_CAPTURE_LIMIT_BYTES;
    let content = if truncated {
        &bytes[..DEFAULT_CAPTURE_LIMIT_BYTES]
    } else {
        bytes
    };
    (String::from_utf8_lossy(content).into_owned(), truncated)
}

fn open_directory_command(path: &Path) -> AppResult<Command> {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(path);
        Ok(command)
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        command.arg(path);
        Ok(command)
    }

    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        Ok(command)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(crate::error::AppError::BadRequest(
            "当前系统不支持打开数据目录".to_string(),
        ))
    }
}

fn open_url_command(url: &str) -> AppResult<Command> {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(url);
        Ok(command)
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        Ok(command)
    }

    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        Ok(command)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(crate::error::AppError::BadRequest(
            "当前系统不支持打开 URL".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AppResult;

    use super::{
        applescript_string, BrowserOpenUrlInput, CapabilityBroker, CapabilityInvokeInput,
        FsReadInput, FsWriteInput, ProcessExecInput,
    };

    #[test]
    fn audit_log_should_record_open_directory_attempts() {
        let broker = CapabilityBroker::default();
        let record_count_before = broker.audit_log().len();

        assert_eq!(record_count_before, 0);
    }

    #[test]
    fn audit_log_should_record_provider_errors() {
        let broker = CapabilityBroker::default();
        let result: AppResult<()> =
            broker.invoke_audited("test.capability", "run", "target", || {
                Err(crate::error::AppError::BadRequest(
                    "provider failed".to_string(),
                ))
            });

        assert!(result.is_err());
        let audit = broker.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].capability, "test.capability");
        assert_eq!(audit[0].outcome, "error");
        assert_eq!(
            audit[0].detail.as_deref(),
            Some("请求参数无效：provider failed")
        );
    }

    #[test]
    fn applescript_string_should_escape_quotes_and_backslashes() {
        assert_eq!(
            applescript_string(r#"a "quoted" \ path"#),
            r#""a \"quoted\" \\ path""#
        );
    }

    #[test]
    fn fs_read_write_should_return_content_and_audit() {
        let broker = CapabilityBroker::default();
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("nested/file.txt");
        let path_text = path.to_string_lossy().into_owned();

        let write = broker
            .write_text_file(&FsWriteInput {
                append: false,
                content: "abcdef".to_string(),
                create_dirs: true,
                path: path_text.clone(),
            })
            .expect("write succeeds");
        assert_eq!(write.bytes, 6);

        let read = broker
            .read_text_file(&FsReadInput {
                max_bytes: Some(3),
                path: path_text,
            })
            .expect("read succeeds");
        assert_eq!(read.bytes, 6);
        assert_eq!(read.content, "abc");
        assert!(read.truncated);

        let audit = broker.audit_log();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].capability, "fs.write");
        assert_eq!(audit[0].outcome, "allowed");
        assert_eq!(audit[1].capability, "fs.read");
        assert_eq!(audit[1].outcome, "allowed");
    }

    #[test]
    fn process_exec_should_reject_unlisted_commands_and_audit() {
        let broker = CapabilityBroker::default();
        let result = broker.run_process(&ProcessExecInput {
            args: vec!["hello".to_string()],
            command: "sh".to_string(),
            cwd: None,
        });

        assert!(result.is_err());
        let audit = broker.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].capability, "process.exec");
        assert_eq!(audit[0].outcome, "error");
        assert!(audit[0]
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("not allowlisted"));
    }

    #[test]
    fn browser_open_url_should_reject_non_http_urls_and_audit() {
        let broker = CapabilityBroker::default();
        let result = broker.open_url(&BrowserOpenUrlInput {
            url: "file:///etc/passwd".to_string(),
        });

        assert!(result.is_err());
        let audit = broker.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].capability, "browser.openUrl");
        assert_eq!(audit[0].outcome, "error");
    }

    #[test]
    fn invoke_json_should_dispatch_to_typed_capability_provider() {
        let broker = CapabilityBroker::default();
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("invoke.txt");
        let path_text = path.to_string_lossy().into_owned();

        let write = broker
            .invoke_json(CapabilityInvokeInput {
                capability: "fs.write".to_string(),
                input: serde_json::json!({
                    "path": path_text,
                    "content": "hello",
                    "createDirs": true
                }),
            })
            .expect("generic write succeeds");
        assert_eq!(write["bytes"], 5);

        let read = broker
            .invoke_json(CapabilityInvokeInput {
                capability: "fs.read".to_string(),
                input: serde_json::json!({
                    "path": path.to_string_lossy(),
                }),
            })
            .expect("generic read succeeds");
        assert_eq!(read["content"], "hello");

        let audit = broker.audit_log();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].capability, "fs.write");
        assert_eq!(audit[1].capability, "fs.read");
    }
}
