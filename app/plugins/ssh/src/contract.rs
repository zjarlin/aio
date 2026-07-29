//! SSH 服务器运维插件共享契约。

use serde::{Deserialize, Serialize};

pub const ROUTE: &str = "/ssh";
pub const STATUS_PATH: &str = "/api/ssh/status";
pub const APPLY_TEMPLATE_PATH: &str = "/api/ssh/templates/default/apply";
pub const TARGETS_PATH: &str = "/api/ssh/targets";
pub const COMMANDS_PATH: &str = "/api/ssh/commands";
pub const COLLECT_PATH: &str = "/api/ssh/collect";
pub const EXECUTE_PATH: &str = "/api/ssh/execute";
pub const UI_ACTION_PATH: &str = "/api/ssh/ui-action";

pub const OP_TEMPLATE_APPLY: &str = "ssh.templates.default.apply";
pub const OP_TARGET_UPSERT: &str = "ssh.targets.upsert";
pub const OP_COMMAND_UPSERT: &str = "ssh.commands.upsert";
pub const OP_COLLECT: &str = "ssh.commands.collect";
pub const OP_EXECUTE: &str = "ssh.commands.execute";

pub const TARGET_MODEL: &str = "ssh_target";
pub const COMMAND_MODEL: &str = "ssh_command";
pub const RESULT_MODEL: &str = "ssh_command_result";

pub const AUTH_PRIVATE_KEY: &str = "private_key";
pub const AUTH_PASSWORD_ENV: &str = "password_env";
pub const COMMAND_KIND_MONITOR: &str = "monitor";
pub const COMMAND_KIND_OPERATION: &str = "operation";
pub const STATUS_SUCCESS: &str = "success";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_UNSUPPORTED: &str = "unsupported";

/// 初始化 SSH 低代码模板的请求。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySshTemplateRequest {
    #[serde(default = "default_true")]
    pub seed_builtin_commands: bool,
}

/// SSH 低代码模板初始化结果。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTemplateApplyResult {
    pub created_models: usize,
    pub created_fields: usize,
    pub seeded_commands: usize,
    pub model_names: Vec<String>,
}

/// 新建或更新 SSH 目标的请求。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertSshTargetRequest {
    pub code: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: i64,
    pub username: String,
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    #[serde(default)]
    pub private_key_path: String,
    #[serde(default)]
    pub password_env: String,
    #[serde(default)]
    pub passphrase_env: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// 新建或更新低代码 SSH 命令的请求。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertSshCommandRequest {
    pub code: String,
    pub name: String,
    pub category: String,
    pub hardware_family: String,
    #[serde(default)]
    pub detect_script: String,
    pub command_script: String,
    #[serde(default = "default_command_kind")]
    pub kind: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub order_index: i64,
}

/// 执行目标命令的请求，`command_code` 为空时执行全部监测命令。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSshCommandsRequest {
    pub target_code: String,
    #[serde(default)]
    pub command_code: Option<String>,
}

/// SSH 目标页面投影。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTargetView {
    pub record_id: String,
    pub code: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: String,
    pub password_env: String,
    pub passphrase_env: String,
    pub description: String,
    pub enabled: bool,
}

/// SSH 命令页面投影。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshCommandView {
    pub record_id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub hardware_family: String,
    pub detect_script: String,
    pub command_script: String,
    pub kind: String,
    pub timeout_secs: i64,
    pub enabled: bool,
    pub order_index: i64,
}

/// 单条 SSH 命令执行结果。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshCommandResultView {
    pub record_id: String,
    pub target_code: String,
    pub target_name: String,
    pub command_code: String,
    pub command_name: String,
    pub category: String,
    pub hardware_family: String,
    pub status: String,
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i64,
    pub collected_at_ms: i64,
}

/// 页面和 API 共用的 SSH 运维聚合快照。
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshDashboardSnapshot {
    pub template_ready: bool,
    pub targets: Vec<SshTargetView>,
    pub commands: Vec<SshCommandView>,
    pub results: Vec<SshCommandResultView>,
}

fn default_true() -> bool {
    true
}

fn default_ssh_port() -> i64 {
    22
}

fn default_auth_type() -> String {
    AUTH_PRIVATE_KEY.to_string()
}

fn default_command_kind() -> String {
    COMMAND_KIND_MONITOR.to_string()
}

fn default_timeout_secs() -> i64 {
    15
}
