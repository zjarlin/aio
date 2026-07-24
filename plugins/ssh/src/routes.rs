//! SSH 服务器运维 REST API 与 SSR 表单操作。

use anyhow::{Context, Result, anyhow};
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use az_aio_platform::core::api_error::{ApiError, ApiForm, ApiJson, ApiResponse, ok_json};
use serde::Deserialize;

use crate::{
    contract::{
        APPLY_TEMPLATE_PATH, AUTH_PRIVATE_KEY, ApplySshTemplateRequest, COLLECT_PATH,
        COMMAND_KIND_MONITOR, COMMANDS_PATH, EXECUTE_PATH, RunSshCommandsRequest, STATUS_PATH,
        SshCommandResultView, SshCommandView, SshDashboardSnapshot, SshTargetView,
        SshTemplateApplyResult, TARGETS_PATH, UI_ACTION_PATH, UpsertSshCommandRequest,
        UpsertSshTargetRequest,
    },
    service::SshService,
};

/// SSH 运维 API 状态。
#[derive(Clone)]
pub struct SshApiState {
    service: SshService,
}

impl SshApiState {
    /// 创建 SSH 运维 API 状态。
    pub fn new(service: SshService) -> Self {
        Self { service }
    }
}

/// 构建 SSH 服务器运维路由。
pub fn ssh_router(state: SshApiState) -> Router {
    Router::new()
        .route(STATUS_PATH, get(status_handler))
        .route(APPLY_TEMPLATE_PATH, post(apply_template_handler))
        .route(TARGETS_PATH, post(upsert_target_handler))
        .route(COMMANDS_PATH, post(upsert_command_handler))
        .route(COLLECT_PATH, post(run_commands_handler))
        .route(EXECUTE_PATH, post(run_commands_handler))
        .route(UI_ACTION_PATH, post(ui_action_handler))
        .with_state(state)
}

async fn status_handler(
    State(state): State<SshApiState>,
) -> Result<Json<ApiResponse<SshDashboardSnapshot>>, ApiError> {
    state
        .service
        .dashboard()
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn apply_template_handler(
    State(state): State<SshApiState>,
    ApiJson(request): ApiJson<ApplySshTemplateRequest>,
) -> Result<Json<ApiResponse<SshTemplateApplyResult>>, ApiError> {
    state
        .service
        .apply_template(request)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn upsert_target_handler(
    State(state): State<SshApiState>,
    ApiJson(request): ApiJson<UpsertSshTargetRequest>,
) -> Result<Json<ApiResponse<SshTargetView>>, ApiError> {
    state
        .service
        .upsert_target(request)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn upsert_command_handler(
    State(state): State<SshApiState>,
    ApiJson(request): ApiJson<UpsertSshCommandRequest>,
) -> Result<Json<ApiResponse<SshCommandView>>, ApiError> {
    state
        .service
        .upsert_command(request)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn run_commands_handler(
    State(state): State<SshApiState>,
    ApiJson(request): ApiJson<RunSshCommandsRequest>,
) -> Result<Json<ApiResponse<Vec<SshCommandResultView>>>, ApiError> {
    state
        .service
        .run_commands(request)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn ui_action_handler(
    State(state): State<SshApiState>,
    ApiForm(form): ApiForm<SshUiForm>,
) -> Response {
    let result = apply_ui_action(&state.service, &form).await;
    let route = match result {
        Ok((view, message)) => {
            format!("/ssh?view={view}&message={}", urlencoding::encode(&message))
        }
        Err(error) => format!(
            "/ssh?view={}&error={}",
            form.return_view(),
            urlencoding::encode(&error.to_string())
        ),
    };
    let redirect = format!("/?route={}", urlencoding::encode(&route));
    Redirect::to(&redirect).into_response()
}

async fn apply_ui_action(service: &SshService, form: &SshUiForm) -> Result<(String, String)> {
    match form.action.as_str() {
        "apply_template" => {
            let result = service
                .apply_template(ApplySshTemplateRequest {
                    seed_builtin_commands: form.seed_builtin_commands.is_some(),
                })
                .await?;
            Ok((
                "overview".to_string(),
                format!(
                    "SSH 模板已就绪：新建 {} 个模型、{} 个字段、{} 条内置命令",
                    result.created_models, result.created_fields, result.seeded_commands
                ),
            ))
        }
        "upsert_target" => {
            let target = service.upsert_target(form.target_request()?).await?;
            Ok((
                "targets".to_string(),
                format!("目标 {} 已保存", target.code),
            ))
        }
        "upsert_command" => {
            let command = service.upsert_command(form.command_request()?).await?;
            Ok((
                "commands".to_string(),
                format!("命令 {} 已保存", command.code),
            ))
        }
        "collect" => {
            let results = service
                .run_commands(RunSshCommandsRequest {
                    target_code: form.required("target_code", &form.target_code)?,
                    command_code: None,
                })
                .await?;
            Ok((
                "results".to_string(),
                format!("采集完成，共执行 {} 条监测命令", results.len()),
            ))
        }
        "execute" => {
            let command_code = form.required("command_code", &form.command_code)?;
            let results = service
                .run_commands(RunSshCommandsRequest {
                    target_code: form.required("target_code", &form.target_code)?,
                    command_code: Some(command_code.clone()),
                })
                .await?;
            Ok((
                "results".to_string(),
                format!("命令 {command_code} 已执行，返回 {} 条结果", results.len()),
            ))
        }
        other => Err(anyhow!("未知的 SSH UI 操作: {other}")),
    }
}

#[derive(Debug, Deserialize)]
struct SshUiForm {
    action: String,
    return_view: Option<String>,
    seed_builtin_commands: Option<String>,
    target_code: Option<String>,
    code: Option<String>,
    name: Option<String>,
    host: Option<String>,
    port: Option<i64>,
    username: Option<String>,
    auth_type: Option<String>,
    private_key_path: Option<String>,
    password_env: Option<String>,
    passphrase_env: Option<String>,
    description: Option<String>,
    category: Option<String>,
    hardware_family: Option<String>,
    detect_script: Option<String>,
    command_script: Option<String>,
    kind: Option<String>,
    timeout_secs: Option<i64>,
    enabled: Option<String>,
    order_index: Option<i64>,
    command_code: Option<String>,
}

impl SshUiForm {
    fn target_request(&self) -> Result<UpsertSshTargetRequest> {
        Ok(UpsertSshTargetRequest {
            code: self.required("code", &self.code)?,
            name: self.required("name", &self.name)?,
            host: self.required("host", &self.host)?,
            port: self.port.unwrap_or(22),
            username: self.required("username", &self.username)?,
            auth_type: self
                .auth_type
                .clone()
                .unwrap_or_else(|| AUTH_PRIVATE_KEY.to_string()),
            private_key_path: self.private_key_path.clone().unwrap_or_default(),
            password_env: self.password_env.clone().unwrap_or_default(),
            passphrase_env: self.passphrase_env.clone().unwrap_or_default(),
            description: self.description.clone().unwrap_or_default(),
            enabled: self.enabled.is_some(),
        })
    }

    fn command_request(&self) -> Result<UpsertSshCommandRequest> {
        Ok(UpsertSshCommandRequest {
            code: self.required("code", &self.code)?,
            name: self.required("name", &self.name)?,
            category: self.required("category", &self.category)?,
            hardware_family: self.required("hardware_family", &self.hardware_family)?,
            detect_script: self.detect_script.clone().unwrap_or_default(),
            command_script: self.required("command_script", &self.command_script)?,
            kind: self
                .kind
                .clone()
                .unwrap_or_else(|| COMMAND_KIND_MONITOR.to_string()),
            timeout_secs: self.timeout_secs.unwrap_or(15),
            enabled: self.enabled.is_some(),
            order_index: self.order_index.unwrap_or_default(),
        })
    }

    fn required(&self, field: &str, value: &Option<String>) -> Result<String> {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .with_context(|| format!("缺少表单字段: {field}"))
    }

    fn return_view(&self) -> &str {
        self.return_view.as_deref().unwrap_or("overview")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_form_keeps_secret_values_out_of_database_request() -> Result<()> {
        let form = SshUiForm {
            action: "upsert_target".to_string(),
            return_view: None,
            seed_builtin_commands: None,
            target_code: None,
            code: Some("gpu-01".to_string()),
            name: Some("GPU 服务器".to_string()),
            host: Some("10.0.0.10".to_string()),
            port: Some(22),
            username: Some("ops".to_string()),
            auth_type: Some(AUTH_PRIVATE_KEY.to_string()),
            private_key_path: Some("~/.ssh/id_ed25519".to_string()),
            password_env: None,
            passphrase_env: Some("SSH_KEY_PASSPHRASE".to_string()),
            description: None,
            category: None,
            hardware_family: None,
            detect_script: None,
            command_script: None,
            kind: None,
            timeout_secs: None,
            enabled: Some("true".to_string()),
            order_index: None,
            command_code: None,
        };

        let request = form.target_request()?;
        assert_eq!(request.passphrase_env, "SSH_KEY_PASSPHRASE");
        assert_eq!(request.private_key_path, "~/.ssh/id_ed25519");
        Ok(())
    }
}
