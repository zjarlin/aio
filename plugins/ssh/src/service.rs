//! 基于 EngineStore 和 az-ssh 的服务器运维领域服务。

use std::{
    collections::BTreeSet,
    env,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use az_engine::{DataRecordView, EngineStore, FieldInput, ModelInput, PageParams};
use az_ssh::client::{SshConfig, SshSession};
use serde_json::{Value, json};

use crate::{
    command_catalog::builtin_commands,
    contract::{
        AUTH_PASSWORD_ENV, AUTH_PRIVATE_KEY, ApplySshTemplateRequest, COMMAND_KIND_MONITOR,
        COMMAND_KIND_OPERATION, COMMAND_MODEL, RESULT_MODEL, RunSshCommandsRequest, STATUS_FAILED,
        STATUS_SUCCESS, STATUS_UNSUPPORTED, SshCommandResultView, SshCommandView,
        SshDashboardSnapshot, SshTargetView, SshTemplateApplyResult, TARGET_MODEL,
        UpsertSshCommandRequest, UpsertSshTargetRequest,
    },
};

const MAX_RECORDS: usize = 2_000;
const MAX_OUTPUT_BYTES: usize = 64 * 1_024;

#[derive(Clone, Copy)]
struct FieldDefinition {
    name: &'static str,
    display_name: &'static str,
    field_type: &'static str,
    required: bool,
}

#[derive(Clone, Copy)]
struct ModelDefinition {
    name: &'static str,
    display_name: &'static str,
    fields: &'static [FieldDefinition],
}

const TARGET_FIELDS: &[FieldDefinition] = &[
    field("code", "目标编码", "string", true),
    field("name", "显示名称", "string", true),
    field("host", "主机名或 IP", "string", true),
    field("port", "SSH 端口", "int", true),
    field("username", "登录用户", "string", true),
    field("auth_type", "认证方式", "string", true),
    field("private_key_path", "私钥路径", "string", false),
    field("password_env", "密码环境变量", "string", false),
    field("passphrase_env", "私钥口令环境变量", "string", false),
    field("description", "备注", "string", false),
    field("enabled", "启用", "boolean", true),
];

const COMMAND_FIELDS: &[FieldDefinition] = &[
    field("code", "命令编码", "string", true),
    field("name", "命令名称", "string", true),
    field("category", "分类", "string", true),
    field("hardware_family", "硬件族", "string", true),
    field("detect_script", "适配探测脚本", "string", false),
    field("command_script", "执行脚本", "string", true),
    field("kind", "命令类型", "string", true),
    field("timeout_secs", "超时秒数", "int", true),
    field("enabled", "启用", "boolean", true),
    field("order_index", "排序", "int", true),
];

const RESULT_FIELDS: &[FieldDefinition] = &[
    field("target_code", "目标编码", "string", true),
    field("target_name", "目标名称", "string", true),
    field("command_code", "命令编码", "string", true),
    field("command_name", "命令名称", "string", true),
    field("category", "分类", "string", true),
    field("hardware_family", "硬件族", "string", true),
    field("status", "执行状态", "string", true),
    field("exit_code", "退出码", "int", true),
    field("stdout", "标准输出", "string", false),
    field("stderr", "标准错误", "string", false),
    field("duration_ms", "耗时毫秒", "int", true),
    field("collected_at_ms", "采集时间", "datetime", true),
];

const MODEL_DEFINITIONS: &[ModelDefinition] = &[
    model(TARGET_MODEL, "SSH 目标", TARGET_FIELDS),
    model(COMMAND_MODEL, "SSH 监测命令", COMMAND_FIELDS),
    model(RESULT_MODEL, "SSH 最近执行结果", RESULT_FIELDS),
];

const fn field(
    name: &'static str,
    display_name: &'static str,
    field_type: &'static str,
    required: bool,
) -> FieldDefinition {
    FieldDefinition {
        name,
        display_name,
        field_type,
        required,
    }
}

const fn model(
    name: &'static str,
    display_name: &'static str,
    fields: &'static [FieldDefinition],
) -> ModelDefinition {
    ModelDefinition {
        name,
        display_name,
        fields,
    }
}

/// 使用共享 EngineStore 管理 SSH 目标、命令和最近执行结果。
#[derive(Clone)]
pub struct SshService {
    store: EngineStore,
}

impl SshService {
    /// 创建服务器运维服务。
    pub fn new(store: EngineStore) -> Self {
        Self { store }
    }

    /// 初始化三个低代码模型，并按需写入跨硬件命令目录。
    pub async fn apply_template(
        &self,
        request: ApplySshTemplateRequest,
    ) -> Result<SshTemplateApplyResult> {
        let mut created_models = 0;
        let mut created_fields = 0;
        for definition in MODEL_DEFINITIONS {
            if self.store.get_model(definition.name).await?.is_none() {
                let input = ModelInput {
                    name: definition.name.to_string(),
                    display_name: definition.display_name.to_string(),
                };
                self.store.create_model(input).await?;
                created_models += 1;
            }

            let existing_names = self
                .store
                .list_fields(definition.name)
                .await?
                .into_iter()
                .map(|item| item.name)
                .collect::<BTreeSet<_>>();
            for (order_index, field) in definition.fields.iter().enumerate() {
                if existing_names.contains(field.name) {
                    continue;
                }
                let input = FieldInput {
                    name: field.name.to_string(),
                    display_name: field.display_name.to_string(),
                    field_type: field.field_type.to_string(),
                    is_required: field.required,
                    expression: None,
                    dependency_json: None,
                    order_index: order_index as i32,
                };
                self.store.create_field(definition.name, input).await?;
                created_fields += 1;
            }
        }

        let seeded_commands = if request.seed_builtin_commands {
            self.seed_builtin_commands().await?
        } else {
            0
        };
        Ok(SshTemplateApplyResult {
            created_models,
            created_fields,
            seeded_commands,
            model_names: MODEL_DEFINITIONS
                .iter()
                .map(|definition| definition.name.to_string())
                .collect(),
        })
    }

    /// 返回目标、命令和最近执行结果组成的工作台快照。
    pub async fn dashboard(&self) -> Result<SshDashboardSnapshot> {
        if !self.template_ready().await? {
            return Ok(SshDashboardSnapshot::default());
        }

        let mut targets = self
            .list_records(TARGET_MODEL)
            .await?
            .into_iter()
            .map(target_view)
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.code.cmp(&right.code));

        let mut commands = self
            .list_records(COMMAND_MODEL)
            .await?
            .into_iter()
            .map(command_view)
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            left.order_index
                .cmp(&right.order_index)
                .then(left.code.cmp(&right.code))
        });

        let mut results = self
            .list_records(RESULT_MODEL)
            .await?
            .into_iter()
            .map(result_view)
            .collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .collected_at_ms
                .cmp(&left.collected_at_ms)
                .then(left.target_code.cmp(&right.target_code))
                .then(left.command_code.cmp(&right.command_code))
        });

        Ok(SshDashboardSnapshot {
            template_ready: true,
            targets,
            commands,
            results,
        })
    }

    /// 按目标编码新建或更新 SSH 连接配置。
    pub async fn upsert_target(&self, request: UpsertSshTargetRequest) -> Result<SshTargetView> {
        let request = normalize_target(request);
        validate_target(&request)?;
        self.ensure_template_ready().await?;

        let payload = json!({
            "code": request.code,
            "name": request.name,
            "host": request.host,
            "port": request.port,
            "username": request.username,
            "auth_type": request.auth_type,
            "private_key_path": request.private_key_path,
            "password_env": request.password_env,
            "passphrase_env": request.passphrase_env,
            "description": request.description,
            "enabled": request.enabled,
        });
        let record = self
            .upsert_record(TARGET_MODEL, "code", &request.code, payload)
            .await?;
        Ok(target_view(record))
    }

    /// 按命令编码新建或更新低代码命令。
    pub async fn upsert_command(&self, request: UpsertSshCommandRequest) -> Result<SshCommandView> {
        let request = normalize_command(request);
        validate_command(&request)?;
        self.ensure_template_ready().await?;

        let payload = command_payload(&request);
        let record = self
            .upsert_record(COMMAND_MODEL, "code", &request.code, payload)
            .await?;
        Ok(command_view(record))
    }

    /// 对指定目标执行全部监测命令或一条明确命令。
    pub async fn run_commands(
        &self,
        request: RunSshCommandsRequest,
    ) -> Result<Vec<SshCommandResultView>> {
        self.ensure_template_ready().await?;
        let target_code = request.target_code.trim();
        if target_code.is_empty() {
            bail!("目标编码不能为空");
        }

        let target_record = self
            .find_record(TARGET_MODEL, "code", target_code)
            .await?
            .ok_or_else(|| anyhow!("SSH 目标不存在: {target_code}"))?;
        let target = target_view(target_record);
        if !target.enabled {
            bail!("SSH 目标已禁用: {}", target.code);
        }

        let command_code = request
            .command_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let mut commands = self
            .list_records(COMMAND_MODEL)
            .await?
            .into_iter()
            .map(command_view)
            .filter(|command| command.enabled)
            .filter(|command| match command_code {
                Some(code) => command.code == code,
                None => command.kind == COMMAND_KIND_MONITOR,
            })
            .collect::<Vec<_>>();
        commands.sort_by_key(|command| command.order_index);
        if commands.is_empty() {
            match command_code {
                Some(code) => bail!("未找到已启用的 SSH 命令: {code}"),
                None => bail!("没有已启用的 SSH 监测命令"),
            }
        }

        let executions =
            tokio::task::spawn_blocking(move || execute_target_commands_blocking(target, commands))
                .await
                .context("SSH 采集任务异常终止")??;

        let mut results = Vec::with_capacity(executions.len());
        for execution in executions {
            let result = self.persist_result(execution).await?;
            results.push(result);
        }
        Ok(results)
    }

    async fn template_ready(&self) -> Result<bool> {
        for definition in MODEL_DEFINITIONS {
            if self.store.get_model(definition.name).await?.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn ensure_template_ready(&self) -> Result<()> {
        if self.template_ready().await? {
            return Ok(());
        }
        bail!("SSH 低代码模板尚未初始化");
    }

    async fn seed_builtin_commands(&self) -> Result<usize> {
        let mut inserted = 0;
        for request in builtin_commands() {
            if self
                .find_record(COMMAND_MODEL, "code", &request.code)
                .await?
                .is_some()
            {
                continue;
            }
            let payload = command_payload(&request);
            self.store
                .executor()
                .insert_record(COMMAND_MODEL, payload)
                .await
                .with_context(|| format!("写入内置 SSH 命令失败: {}", request.code))?;
            inserted += 1;
        }
        Ok(inserted)
    }

    async fn persist_result(&self, execution: RawExecution) -> Result<SshCommandResultView> {
        let payload = json!({
            "target_code": execution.target_code,
            "target_name": execution.target_name,
            "command_code": execution.command_code,
            "command_name": execution.command_name,
            "category": execution.category,
            "hardware_family": execution.hardware_family,
            "status": execution.status,
            "exit_code": execution.exit_code,
            "stdout": truncate_output(&execution.stdout),
            "stderr": truncate_output(&execution.stderr),
            "duration_ms": execution.duration_ms,
            "collected_at_ms": execution.collected_at_ms,
        });
        let existing = self
            .list_records(RESULT_MODEL)
            .await?
            .into_iter()
            .find(|record| {
                text(&record.payload, "target_code") == execution.target_code
                    && text(&record.payload, "command_code") == execution.command_code
            });
        let record = match existing {
            Some(existing) => {
                self.store
                    .executor()
                    .update_record(RESULT_MODEL, &existing.id, payload)
                    .await?
            }
            None => {
                self.store
                    .executor()
                    .insert_record(RESULT_MODEL, payload)
                    .await?
            }
        };
        Ok(result_view(record))
    }

    async fn upsert_record(
        &self,
        model_name: &str,
        key: &str,
        value: &str,
        payload: Value,
    ) -> Result<DataRecordView> {
        let existing = self.find_record(model_name, key, value).await?;
        match existing {
            Some(existing) => {
                self.store
                    .executor()
                    .update_record(model_name, &existing.id, payload)
                    .await
            }
            None => {
                self.store
                    .executor()
                    .insert_record(model_name, payload)
                    .await
            }
        }
    }

    async fn find_record(
        &self,
        model_name: &str,
        key: &str,
        value: &str,
    ) -> Result<Option<DataRecordView>> {
        Ok(self
            .list_records(model_name)
            .await?
            .into_iter()
            .find(|record| text(&record.payload, key) == value))
    }

    async fn list_records(&self, model_name: &str) -> Result<Vec<DataRecordView>> {
        let page = self
            .store
            .executor()
            .list_records(
                model_name,
                PageParams {
                    o: 0,
                    s: MAX_RECORDS,
                },
            )
            .await?;
        Ok(page.d)
    }
}

#[derive(Clone, Debug)]
struct RawExecution {
    target_code: String,
    target_name: String,
    command_code: String,
    command_name: String,
    category: String,
    hardware_family: String,
    status: String,
    exit_code: i64,
    stdout: String,
    stderr: String,
    duration_ms: i64,
    collected_at_ms: i64,
}

fn execute_target_commands_blocking(
    target: SshTargetView,
    commands: Vec<SshCommandView>,
) -> Result<Vec<RawExecution>> {
    let config = ssh_config(&target, &commands)?;
    let session = SshSession::connect(config).with_context(|| {
        format!(
            "连接 SSH 目标失败: {}@{}:{}",
            target.username, target.host, target.port
        )
    })?;
    let mut executions = Vec::with_capacity(commands.len());
    for command in commands {
        executions.push(execute_command(&session, &target, &command));
    }
    Ok(executions)
}

fn ssh_config(target: &SshTargetView, commands: &[SshCommandView]) -> Result<SshConfig> {
    let port = u16::try_from(target.port).context("SSH 端口超出有效范围")?;
    let max_timeout_secs = commands
        .iter()
        .map(|command| command.timeout_secs)
        .max()
        .unwrap_or(15)
        .clamp(1, 300);
    let read_timeout_ms = u32::try_from(max_timeout_secs.saturating_add(5) * 1_000)
        .context("SSH 读取超时超出有效范围")?;
    let mut config = SshConfig::builder(&target.host, &target.username)
        .port(port)
        .connect_timeout_ms(10_000)
        .read_timeout_ms(read_timeout_ms);

    match target.auth_type.as_str() {
        AUTH_PRIVATE_KEY => {
            config = config.private_key_path(&target.private_key_path);
            if !target.passphrase_env.is_empty() {
                let passphrase = read_secret_env(&target.passphrase_env, "私钥口令")?;
                config = config.private_key_passphrase(passphrase);
            }
        }
        AUTH_PASSWORD_ENV => {
            let password = read_secret_env(&target.password_env, "SSH 密码")?;
            config = config.password(password);
        }
        other => bail!("不支持的 SSH 认证方式: {other}"),
    }
    config.build()
}

fn read_secret_env(name: &str, label: &str) -> Result<String> {
    env::var(name)
        .with_context(|| format!("{label}环境变量未设置: {name}"))
        .and_then(|value| {
            if value.is_empty() {
                bail!("{label}环境变量为空: {name}");
            }
            Ok(value)
        })
}

fn execute_command(
    session: &SshSession,
    target: &SshTargetView,
    command: &SshCommandView,
) -> RawExecution {
    let started = Instant::now();
    if !command.detect_script.trim().is_empty() {
        let detect_script =
            wrap_remote_script(&command.detect_script, command.timeout_secs.min(10));
        match session.execute_sync(&detect_script) {
            Ok(result) if !result.is_success() => {
                return raw_execution(
                    target,
                    command,
                    STATUS_UNSUPPORTED,
                    i64::from(result.exit_code),
                    result.stdout,
                    result.stderr,
                    started,
                );
            }
            Err(error) => {
                return raw_execution(
                    target,
                    command,
                    STATUS_FAILED,
                    -1,
                    String::new(),
                    error.to_string(),
                    started,
                );
            }
            Ok(_) => {}
        }
    }

    let script = wrap_remote_script(&command.command_script, command.timeout_secs);
    match session.execute_sync(&script) {
        Ok(result) => {
            let status = if result.is_success() {
                STATUS_SUCCESS
            } else {
                STATUS_FAILED
            };
            raw_execution(
                target,
                command,
                status,
                i64::from(result.exit_code),
                result.stdout,
                result.stderr,
                started,
            )
        }
        Err(error) => raw_execution(
            target,
            command,
            STATUS_FAILED,
            -1,
            String::new(),
            error.to_string(),
            started,
        ),
    }
}

fn raw_execution(
    target: &SshTargetView,
    command: &SshCommandView,
    status: &str,
    exit_code: i64,
    stdout: String,
    stderr: String,
    started: Instant,
) -> RawExecution {
    RawExecution {
        target_code: target.code.clone(),
        target_name: target.name.clone(),
        command_code: command.code.clone(),
        command_name: command.name.clone(),
        category: command.category.clone(),
        hardware_family: command.hardware_family.clone(),
        status: status.to_string(),
        exit_code,
        stdout,
        stderr,
        duration_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
        collected_at_ms: timestamp_ms(),
    }
}

fn wrap_remote_script(script: &str, timeout_secs: i64) -> String {
    let timeout_secs = timeout_secs.clamp(1, 300);
    let quoted = shell_quote(script);
    format!(
        "export PATH=/usr/local/hyhal/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH; if command -v timeout >/dev/null 2>&1; then timeout {timeout_secs}s sh -lc {quoted}; else sh -lc {quoted}; fi"
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn command_payload(request: &UpsertSshCommandRequest) -> Value {
    json!({
        "code": request.code,
        "name": request.name,
        "category": request.category,
        "hardware_family": request.hardware_family,
        "detect_script": request.detect_script,
        "command_script": request.command_script,
        "kind": request.kind,
        "timeout_secs": request.timeout_secs,
        "enabled": request.enabled,
        "order_index": request.order_index,
    })
}

fn normalize_target(mut request: UpsertSshTargetRequest) -> UpsertSshTargetRequest {
    request.code = request.code.trim().to_string();
    request.name = request.name.trim().to_string();
    request.host = request.host.trim().to_string();
    request.username = request.username.trim().to_string();
    request.auth_type = request.auth_type.trim().to_string();
    request.private_key_path = request.private_key_path.trim().to_string();
    request.password_env = request.password_env.trim().to_string();
    request.passphrase_env = request.passphrase_env.trim().to_string();
    request.description = request.description.trim().to_string();
    request
}

fn normalize_command(mut request: UpsertSshCommandRequest) -> UpsertSshCommandRequest {
    request.code = request.code.trim().to_string();
    request.name = request.name.trim().to_string();
    request.category = request.category.trim().to_string();
    request.hardware_family = request.hardware_family.trim().to_string();
    request.detect_script = request.detect_script.trim().to_string();
    request.command_script = request.command_script.trim().to_string();
    request.kind = request.kind.trim().to_string();
    request
}

fn validate_target(request: &UpsertSshTargetRequest) -> Result<()> {
    validate_code(&request.code, "目标编码")?;
    require_text(&request.name, "目标名称")?;
    require_text(&request.host, "主机名或 IP")?;
    require_text(&request.username, "登录用户")?;
    if !(1..=65_535).contains(&request.port) {
        bail!("SSH 端口必须在 1 到 65535 之间");
    }

    match request.auth_type.as_str() {
        AUTH_PRIVATE_KEY => require_text(&request.private_key_path, "私钥路径")?,
        AUTH_PASSWORD_ENV => validate_env_name(&request.password_env, "密码环境变量")?,
        other => bail!("认证方式只支持 {AUTH_PRIVATE_KEY} 或 {AUTH_PASSWORD_ENV}: {other}"),
    }
    if !request.passphrase_env.is_empty() {
        validate_env_name(&request.passphrase_env, "私钥口令环境变量")?;
    }
    Ok(())
}

fn validate_command(request: &UpsertSshCommandRequest) -> Result<()> {
    validate_code(&request.code, "命令编码")?;
    require_text(&request.name, "命令名称")?;
    require_text(&request.category, "命令分类")?;
    require_text(&request.hardware_family, "硬件族")?;
    require_text(&request.command_script, "执行脚本")?;
    if !matches!(
        request.kind.as_str(),
        COMMAND_KIND_MONITOR | COMMAND_KIND_OPERATION
    ) {
        bail!("命令类型只支持 monitor 或 operation");
    }
    if !(1..=300).contains(&request.timeout_secs) {
        bail!("命令超时必须在 1 到 300 秒之间");
    }
    Ok(())
}

fn require_text(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label}不能为空");
    }
    Ok(())
}

fn validate_code(value: &str, label: &str) -> Result<()> {
    require_text(value, label)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Ok(());
    }
    bail!("{label}只能包含英文字母、数字、短横线和下划线");
}

fn validate_env_name(value: &str, label: &str) -> Result<()> {
    require_text(value, label)?;
    let mut chars = value.chars();
    let first_valid = chars
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    if first_valid && chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Ok(());
    }
    bail!("{label}不是有效的环境变量名");
}

fn target_view(record: DataRecordView) -> SshTargetView {
    SshTargetView {
        record_id: record.id,
        code: text(&record.payload, "code"),
        name: text(&record.payload, "name"),
        host: text(&record.payload, "host"),
        port: integer(&record.payload, "port"),
        username: text(&record.payload, "username"),
        auth_type: text(&record.payload, "auth_type"),
        private_key_path: text(&record.payload, "private_key_path"),
        password_env: text(&record.payload, "password_env"),
        passphrase_env: text(&record.payload, "passphrase_env"),
        description: text(&record.payload, "description"),
        enabled: boolean(&record.payload, "enabled"),
    }
}

fn command_view(record: DataRecordView) -> SshCommandView {
    SshCommandView {
        record_id: record.id,
        code: text(&record.payload, "code"),
        name: text(&record.payload, "name"),
        category: text(&record.payload, "category"),
        hardware_family: text(&record.payload, "hardware_family"),
        detect_script: text(&record.payload, "detect_script"),
        command_script: text(&record.payload, "command_script"),
        kind: text(&record.payload, "kind"),
        timeout_secs: integer(&record.payload, "timeout_secs"),
        enabled: boolean(&record.payload, "enabled"),
        order_index: integer(&record.payload, "order_index"),
    }
}

fn result_view(record: DataRecordView) -> SshCommandResultView {
    SshCommandResultView {
        record_id: record.id,
        target_code: text(&record.payload, "target_code"),
        target_name: text(&record.payload, "target_name"),
        command_code: text(&record.payload, "command_code"),
        command_name: text(&record.payload, "command_name"),
        category: text(&record.payload, "category"),
        hardware_family: text(&record.payload, "hardware_family"),
        status: text(&record.payload, "status"),
        exit_code: integer(&record.payload, "exit_code"),
        stdout: text(&record.payload, "stdout"),
        stderr: text(&record.payload, "stderr"),
        duration_ms: integer(&record.payload, "duration_ms"),
        collected_at_ms: integer(&record.payload, "collected_at_ms"),
    }
}

fn text(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn integer(payload: &Value, key: &str) -> i64 {
    payload.get(key).and_then(Value::as_i64).unwrap_or_default()
}

fn boolean(payload: &Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

fn truncate_output(value: &str) -> String {
    if value.len() <= MAX_OUTPUT_BYTES {
        return value.to_string();
    }
    let mut end = MAX_OUTPUT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[输出已截断]", &value[..end])
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_wrapper_quotes_configured_scripts() {
        let script = "printf '%s' \"$HOME\"";
        let wrapped = wrap_remote_script(script, 15);

        assert!(wrapped.contains("timeout 15s sh -lc"));
        assert!(wrapped.contains("'\"'\"'"));
        assert!(wrapped.contains("/usr/local/hyhal/bin"));
    }

    #[test]
    fn target_requires_portable_auth_material_reference() {
        let request = UpsertSshTargetRequest {
            code: "gpu-01".to_string(),
            name: "GPU 服务器".to_string(),
            host: "192.168.1.10".to_string(),
            port: 22,
            username: "ops".to_string(),
            auth_type: AUTH_PASSWORD_ENV.to_string(),
            private_key_path: String::new(),
            password_env: "SSH_GPU_01_PASSWORD".to_string(),
            passphrase_env: String::new(),
            description: String::new(),
            enabled: true,
        };

        assert!(validate_target(&request).is_ok());
    }

    #[test]
    fn operation_command_must_be_run_explicitly() {
        let mut command = builtin_commands().remove(0);
        command.kind = COMMAND_KIND_OPERATION.to_string();

        assert!(validate_command(&command).is_ok());
        assert_ne!(command.kind, COMMAND_KIND_MONITOR);
    }
}
