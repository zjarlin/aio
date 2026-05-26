use std::{net::SocketAddr, path::PathBuf};

use axum::{
    extract::{Json, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tower_http::cors::{Any, CorsLayer};

use acp_openai_assistant::{
    ask_assistant, preview_page_context, AssistantChatRequest, PageContextInput,
};

use crate::{
    agent_preferences::{
        self, AgentPreferenceInput, AgentPreferencePageRequest, AgentPreferenceToggleInput,
        AgentPreferenceUpdateInput,
    },
    app_paths,
    app_runtime::{
        AppRuntimeReloadInput, AppRuntimeSessionInput, AppRuntimeStartInput, AppRuntimeStopInput,
        AppRuntimeWorkspaceInput,
    },
    asset_items::{
        self, AssetItemDeployInput, AssetItemDeployPreviewRequest, AssetItemImportRequest,
        AssetItemInput, AssetItemPageRequest, AssetItemToggleInput, AssetItemUpdateInput,
    },
    asset_variables::{self, AssetVariableInput, AssetVariablePageRequest},
    auth::{self, LoginRequest},
    capability_broker::{
        BrowserOpenUrlInput, CapabilityBroker, CapabilityInvokeInput, ClipboardWriteInput,
        FsReadInput, FsWriteInput, NotificationSendInput, ProcessExecInput,
    },
    dictionary::{
        self, DictItemInput, DictItemPageRequest, DictItemUpdateInput, DictTypeInput,
        DictTypeUpdateInput,
    },
    error::{AppError, AppResult},
    event_bus::{EventBusPublishInput, EventBusSnapshotRequest, EventBusStreamInput},
    extension_host::ExtensionHostSourceInput,
    notes::{self, NoteFlagInput, NoteInput, NotePageRequest, NoteUpdateInput},
    permission_approval::{self, PermissionApprovalDecisionInput, PermissionApprovalRequestInput},
    permission_consent::{self, PermissionConsentGrantInput, PermissionConsentRevokeInput},
    permission_core::PermissionCore,
    plugin_factory::{
        self, PluginCreateFromPromptInput, PluginRepairFromDiagnosticsInput, PluginVerifyDraftInput,
    },
    plugin_registry,
    plugin_store::{
        ChildCapabilityApprovalInput, PluginRegistryRollbackInput, PluginRegistryStore,
    },
    rbac::{
        self, AssignPermissionsInput, PageRequest, PermissionInput, RoleInput, RoleUpdateInput,
        UserInput, UserPasswordInput, UserUpdateInput,
    },
    skills::{self, SkillInput, SkillPageRequest, SkillToggleInput, SkillUpdateInput},
    state::AppState,
};

const DEFAULT_BRIDGE_PORT: u16 = 18777;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeRequest {
    command: String,
    #[serde(default)]
    payload: Value,
}

pub fn spawn(state: AppState) {
    std::thread::spawn(
        move || match Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => {
                runtime.block_on(async move {
                    if let Err(error) = run(state).await {
                        eprintln!("AIO browser command bridge failed: {error}");
                    }
                });
            }
            Err(error) => {
                eprintln!("AIO browser command bridge runtime init failed: {error}");
            }
        },
    );
}

pub fn serve_headless(data_dir: PathBuf) -> AppResult<()> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(async move {
        let state = AppState::from_data_dir(data_dir).await?;
        run(state).await
    })
}

async fn run(state: AppState) -> AppResult<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], bridge_port()));
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("AIO browser command bridge bind failed on {addr}: {error}");
            return Err(AppError::BadRequest(format!(
                "AIO browser command bridge bind failed on {addr}: {error}"
            )));
        }
    };

    eprintln!("AIO browser command bridge listening on http://{addr}/__aio/command");

    let app = Router::new()
        .route("/__aio/command", post(handle_command))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::POST])
                .allow_headers([header::CONTENT_TYPE]),
        )
        .with_state(state);

    axum::serve(listener, app).await?;
    Ok(())
}

fn bridge_port() -> u16 {
    std::env::var("AIO_COMMAND_BRIDGE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_BRIDGE_PORT)
}

async fn handle_command(
    State(state): State<AppState>,
    Json(request): Json<BridgeRequest>,
) -> Response {
    match dispatch(&state, &request.command, request.payload).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => (status_from(&error), Json(error.into_command_error())).into_response(),
    }
}

async fn dispatch(state: &AppState, command: &str, payload: Value) -> AppResult<Value> {
    macro_rules! boxed {
        ($future:expr) => {{
            serde_json::to_value($future.await?).map_err(|source| AppError::Json { source })
        }};
    }

    let pool = &state.pool;

    match command {
        "agent_preference_page" => boxed!(agent_preferences::page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AgentPreferencePageRequest>(&payload, "request")?
        )),
        "agent_preference_create" => boxed!(agent_preferences::create(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AgentPreferenceInput>(&payload, "input")?
        )),
        "agent_preference_update" => boxed!(agent_preferences::update(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AgentPreferenceUpdateInput>(&payload, "input")?
        )),
        "agent_preference_delete" => boxed!(agent_preferences::delete(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "agent_preference_toggle" => boxed!(agent_preferences::toggle(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AgentPreferenceToggleInput>(&payload, "input")?
        )),
        "app_open_data_dir" => {
            let token = payload_field::<String>(&payload, "token")?;
            authorize_broker_capability(
                state,
                &token,
                "browser.openDirectory",
                state.data_dir.to_string_lossy().into_owned(),
                state.data_dir.to_string_lossy().into_owned(),
            )
            .await?;
            serde_json::to_value(app_paths::open_data_dir(
                &state.data_dir,
                &state.capability_broker,
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "capability_audit_log" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(state.capability_broker.audit_log())
                .map_err(|source| AppError::Json { source })
        }
        "permission_audit_log" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            let consent_granted = permission_consent::is_granted(
                pool,
                &user.user_id,
                "platform.permission-core",
                "permission.audit",
                "*",
            )
            .await?;
            let mut request = PermissionCore::system_request(
                user.user_id,
                "platform.permission-core",
                "permission.audit",
                "permission-decision-history",
                ["permission.evaluate", "permission.audit"],
            );
            request.consent_granted = consent_granted;
            let policies = plugin_registry::registry_from_workspace(
                plugin_registry::default_project_root(),
                &state.data_dir,
            )?
            .policies;
            state
                .permission_core
                .evaluate_runtime_with_policies(request, &policies)?;
            serde_json::to_value(state.permission_core.audit_log())
                .map_err(|source| AppError::Json { source })
        }
        "capability_clipboard_write" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<ClipboardWriteInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "clipboard.write",
                format!("{} bytes", input.text.len()),
                "system-clipboard".to_string(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.write_clipboard(&input.text)?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_clipboard_read" => {
            let token = payload_field::<String>(&payload, "token")?;
            authorize_broker_capability(
                state,
                &token,
                "clipboard.read",
                "system-clipboard".to_string(),
                "system-clipboard".to_string(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.read_clipboard()?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_notification_send" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<NotificationSendInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "notification.send",
                input.title.clone(),
                input.title.clone(),
            )
            .await?;
            serde_json::to_value(
                state
                    .capability_broker
                    .send_notification(&input.title, &input.body)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "capability_fs_read" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<FsReadInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "fs.read",
                input.path.clone(),
                input.path.clone(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.read_text_file(&input)?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_fs_write" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<FsWriteInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "fs.write",
                input.path.clone(),
                input.path.clone(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.write_text_file(&input)?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_process_exec" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<ProcessExecInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "process.exec",
                input.command.clone(),
                input.command.clone(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.run_process(&input)?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_browser_open_url" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<BrowserOpenUrlInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                "browser.openUrl",
                input.url.clone(),
                input.url.clone(),
            )
            .await?;
            serde_json::to_value(state.capability_broker.open_url(&input)?)
                .map_err(|source| AppError::Json { source })
        }
        "capability_invoke" => {
            let token = payload_field::<String>(&payload, "token")?;
            let input = payload_field::<CapabilityInvokeInput>(&payload, "input")?;
            authorize_broker_capability(
                state,
                &token,
                &input.capability,
                input.audit_target(),
                input.permission_scope(),
            )
            .await?;
            state.capability_broker.invoke_json(input)
        }
        "permission_consent_list" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(permission_consent::list_for_user(pool, &user.user_id).await?)
                .map_err(|source| AppError::Json { source })
        }
        "permission_consent_grant" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(
                permission_consent::grant_for_user(
                    pool,
                    &user.user_id,
                    payload_field::<PermissionConsentGrantInput>(&payload, "input")?,
                )
                .await?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "permission_consent_revoke" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(
                permission_consent::revoke_for_user(
                    pool,
                    &user.user_id,
                    payload_field::<PermissionConsentRevokeInput>(&payload, "input")?,
                )
                .await?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "permission_approval_list" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(permission_approval::list_for_user(pool, &user.user_id).await?)
                .map_err(|source| AppError::Json { source })
        }
        "permission_approval_approve" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(
                permission_approval::approve_for_user(
                    pool,
                    &user.user_id,
                    payload_field::<PermissionApprovalDecisionInput>(&payload, "input")?,
                )
                .await?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "permission_approval_deny" => {
            let token = payload_field::<String>(&payload, "token")?;
            let user = auth::require_session(pool, &token).await?;
            serde_json::to_value(
                permission_approval::deny_for_user(
                    pool,
                    &user.user_id,
                    payload_field::<PermissionApprovalDecisionInput>(&payload, "input")?,
                )
                .await?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "app_runtime_snapshot" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(state.app_runtime.snapshot())
                .map_err(|source| AppError::Json { source })
        }
        "app_runtime_start" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let record = state
                .app_runtime
                .start(payload_field::<AppRuntimeStartInput>(&payload, "input")?)?;
            state.event_bus.publish(record.event_input())?;
            serde_json::to_value(record).map_err(|source| AppError::Json { source })
        }
        "app_runtime_stop" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let record = state
                .app_runtime
                .stop(payload_field::<AppRuntimeStopInput>(&payload, "input")?)?;
            state.event_bus.publish(record.event_input())?;
            serde_json::to_value(record).map_err(|source| AppError::Json { source })
        }
        "app_runtime_reload" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let record = state
                .app_runtime
                .reload(payload_field::<AppRuntimeReloadInput>(&payload, "input")?)?;
            state.event_bus.publish(record.event_input())?;
            serde_json::to_value(record).map_err(|source| AppError::Json { source })
        }
        "app_runtime_workspace" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let record = state.app_runtime.set_workspace(payload_field::<
                AppRuntimeWorkspaceInput,
            >(&payload, "input")?)?;
            state.event_bus.publish(record.event_input())?;
            serde_json::to_value(record).map_err(|source| AppError::Json { source })
        }
        "app_runtime_session" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let record = state
                .app_runtime
                .set_session(payload_field::<AppRuntimeSessionInput>(&payload, "input")?)?;
            state.event_bus.publish(record.event_input())?;
            serde_json::to_value(record).map_err(|source| AppError::Json { source })
        }
        "event_bus_publish" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state
                    .event_bus
                    .publish(payload_field::<EventBusPublishInput>(&payload, "input")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "event_bus_stream" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state
                    .event_bus
                    .stream(payload_field::<EventBusStreamInput>(&payload, "input")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "event_bus_snapshot" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            let request = payload_field::<EventBusSnapshotRequest>(&payload, "request")?;
            serde_json::to_value(
                state
                    .event_bus
                    .snapshot(request.event_type.as_deref(), request.limit),
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_load" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state.extension_host.load(
                    payload_field::<ExtensionHostSourceInput>(&payload, "input")?.source_path,
                )?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_activate" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state
                    .extension_host
                    .activate(&payload_field::<String>(&payload, "pluginId")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_deactivate" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state
                    .extension_host
                    .deactivate(&payload_field::<String>(&payload, "pluginId")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_reload" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(state.extension_host.reload(
                payload_field::<ExtensionHostSourceInput>(&payload, "input")?.source_path,
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_dispose" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                state
                    .extension_host
                    .dispose(&payload_field::<String>(&payload, "pluginId")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_host_snapshot" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(state.extension_host.snapshot())
                .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_local_state" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(PluginRegistryStore::new(&state.data_dir).state()?)
                .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_install" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir)
                    .install(payload_field::<String>(&payload, "sourcePath")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_enable" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir)
                    .set_enabled(&payload_field::<String>(&payload, "id")?, true)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_disable" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir)
                    .set_enabled(&payload_field::<String>(&payload, "id")?, false)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_uninstall" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir)
                    .uninstall(&payload_field::<String>(&payload, "id")?)?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_registry_rollback" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(PluginRegistryStore::new(&state.data_dir).rollback(
                payload_field::<PluginRegistryRollbackInput>(&payload, "input")?,
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_create_from_prompt" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(plugin_factory::create_from_prompt_and_write(
                payload_field::<PluginCreateFromPromptInput>(&payload, "input")?,
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_publish_gate" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(plugin_factory::publish_gate(
                payload_field::<String>(&payload, "sourcePath")?,
                &state.data_dir,
                payload
                    .get("write")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_publish_local" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(plugin_factory::publish_local(
                payload_field::<String>(&payload, "sourcePath")?,
                &state.data_dir,
            )?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_repair_from_diagnostics" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(plugin_factory::repair_from_diagnostics(payload_field::<
                PluginRepairFromDiagnosticsInput,
            >(
                &payload, "input",
            )?)?)
            .map_err(|source| AppError::Json { source })
        }
        "plugin_verify_draft" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(plugin_factory::verify_plugin_draft(payload_field::<
                PluginVerifyDraftInput,
            >(
                &payload, "input"
            )?)?)
            .map_err(|source| AppError::Json { source })
        }
        "openai_assistant_preview_context" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            boxed!(async {
                let input = payload_field::<PageContextInput>(&payload, "input")?;
                preview_page_context(&input)
                    .await
                    .map_err(|error| AppError::Assistant(error.to_string()))
            })
        }
        "openai_assistant_chat" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            boxed!(async {
                ask_assistant(payload_field::<AssistantChatRequest>(&payload, "input")?)
                    .await
                    .map_err(|error| AppError::Assistant(error.to_string()))
            })
        }
        "auth_login" => boxed!(auth::login(
            pool,
            payload_field::<LoginRequest>(&payload, "request")?
        )),
        "auth_logout" => boxed!(auth::logout(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "auth_current_user" => boxed!(auth::current_user(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "auth_access_codes" => boxed!(rbac::access_codes(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "menu_list" => boxed!(rbac::menus(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "plugin_registry_snapshot" => boxed!(plugin_registry::snapshot(
            pool,
            payload_field::<String>(&payload, "token")?,
            &state.data_dir,
        )),
        "plugin_registry_reload" => {
            let snapshot = plugin_registry::snapshot(
                pool,
                payload_field::<String>(&payload, "token")?,
                &state.data_dir,
            )
            .await?;
            let event_detail = json!({
                "plugins": snapshot.plugins.len(),
                "systemCapsules": snapshot.system_capsules.len(),
                "commands": snapshot.commands.len(),
                "tools": snapshot.tools.len(),
                "settings": snapshot.settings.len(),
                "resources": snapshot.resources.len(),
                "routes": snapshot.routes.len(),
                "views": snapshot.views.len(),
            });
            PluginRegistryStore::new(&state.data_dir).append_audit(
                crate::plugin_store::RegistryAuditRecord {
                    action: "reload".to_string(),
                    content_hash: None,
                    detail: Some(format!(
                        "plugins={},systemCapsules={},commands={},tools={},settings={},resources={},routes={},views={}",
                        snapshot.plugins.len(),
                        snapshot.system_capsules.len(),
                        snapshot.commands.len(),
                        snapshot.tools.len(),
                        snapshot.settings.len(),
                        snapshot.resources.len(),
                        snapshot.routes.len(),
                        snapshot.views.len()
                    )),
                    id: "registry".to_string(),
                    path: Some(
                        state
                            .data_dir
                            .join("plugin-registry")
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    status: "ok".to_string(),
                    timestamp: crate::db::now_millis(),
                },
            )?;
            state.event_bus.publish(EventBusPublishInput {
                event_type: "registry.reloaded".to_string(),
                payload: event_detail,
                source: "platform.registry".to_string(),
                target: None,
                parent_trace_id: None,
                permissions: None,
                schema: Some("schemas/events/registry-reloaded.v1.schema.json".to_string()),
            })?;
            serde_json::to_value(snapshot).map_err(|source| AppError::Json { source })
        }
        "plugin_child_capability_approve" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir).approve_child_capability(
                    payload_field::<ChildCapabilityApprovalInput>(&payload, "input")?,
                )?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "plugin_child_capability_revoke" => {
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(
                PluginRegistryStore::new(&state.data_dir).revoke_child_capability(
                    payload_field::<ChildCapabilityApprovalInput>(&payload, "input")?,
                )?,
            )
            .map_err(|source| AppError::Json { source })
        }
        "user_page" => boxed!(rbac::user_page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<PageRequest>(&payload, "request")?
        )),
        "user_create" => boxed!(rbac::create_user(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<UserInput>(&payload, "input")?
        )),
        "user_update" => boxed!(rbac::update_user(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<UserUpdateInput>(&payload, "input")?
        )),
        "user_disable" => boxed!(rbac::disable_user(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "user_reset_password" => boxed!(rbac::reset_password(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<UserPasswordInput>(&payload, "input")?
        )),
        "user_delete" => boxed!(rbac::delete_user(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "role_page" => boxed!(rbac::role_page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<PageRequest>(&payload, "request")?
        )),
        "role_create" => boxed!(rbac::create_role(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<RoleInput>(&payload, "input")?
        )),
        "role_update" => boxed!(rbac::update_role(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<RoleUpdateInput>(&payload, "input")?
        )),
        "role_delete" => boxed!(rbac::delete_role(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "role_assign_permissions" => boxed!(rbac::assign_permissions(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssignPermissionsInput>(&payload, "input")?
        )),
        "role_permission_ids" => boxed!(rbac::role_permission_ids(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "roleId")?
        )),
        "permission_tree" => boxed!(rbac::permission_tree(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "permission_save" => boxed!(rbac::save_permission(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<PermissionInput>(&payload, "input")?
        )),
        "dict_type_page" => boxed!(dictionary::type_page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<PageRequest>(&payload, "request")?
        )),
        "dict_type_create" => boxed!(dictionary::create_type(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DictTypeInput>(&payload, "input")?
        )),
        "dict_type_update" => boxed!(dictionary::update_type(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DictTypeUpdateInput>(&payload, "input")?
        )),
        "dict_type_delete" => boxed!(dictionary::delete_type(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "dict_item_page" => boxed!(dictionary::item_page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DictItemPageRequest>(&payload, "request")?
        )),
        "dict_item_create" => boxed!(dictionary::create_item(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DictItemInput>(&payload, "input")?
        )),
        "dict_item_update" => boxed!(dictionary::update_item(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DictItemUpdateInput>(&payload, "input")?
        )),
        "dict_item_delete" => boxed!(dictionary::delete_item(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "note_page" => boxed!(notes::page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<NotePageRequest>(&payload, "request")?
        )),
        "note_create" => boxed!(notes::create(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<NoteInput>(&payload, "input")?
        )),
        "note_update" => boxed!(notes::update(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<NoteUpdateInput>(&payload, "input")?
        )),
        "note_delete" => boxed!(notes::delete(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "note_archive" => boxed!(notes::set_archived(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<NoteFlagInput>(&payload, "input")?
        )),
        "note_favorite" => boxed!(notes::set_favorite(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<NoteFlagInput>(&payload, "input")?
        )),
        "skill_page" => boxed!(skills::page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<SkillPageRequest>(&payload, "request")?
        )),
        "skill_create" => boxed!(skills::create(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<SkillInput>(&payload, "input")?
        )),
        "skill_update" => boxed!(skills::update(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<SkillUpdateInput>(&payload, "input")?
        )),
        "skill_delete" => boxed!(skills::delete(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "skill_toggle" => boxed!(skills::toggle(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<SkillToggleInput>(&payload, "input")?
        )),
        "asset_item_page" => boxed!(asset_items::page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemPageRequest>(&payload, "request")?
        )),
        "asset_item_import_directory" => boxed!(asset_items::import_directory(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemImportRequest>(&payload, "request")?
        )),
        "asset_item_create" => boxed!(asset_items::create(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemInput>(&payload, "input")?
        )),
        "asset_item_update" => boxed!(asset_items::update(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemUpdateInput>(&payload, "input")?
        )),
        "asset_item_delete" => boxed!(asset_items::delete(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "asset_item_toggle" => boxed!(asset_items::toggle(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemToggleInput>(&payload, "input")?
        )),
        "asset_item_deploy_preview" => boxed!(asset_items::deploy_preview(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemDeployPreviewRequest>(&payload, "request")?
        )),
        "asset_item_deploy_save" => boxed!(asset_items::deploy_save(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetItemDeployInput>(&payload, "input")?
        )),
        "asset_variable_page" => boxed!(asset_variables::page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetVariablePageRequest>(&payload, "request")?
        )),
        "asset_variable_upsert" => boxed!(asset_variables::upsert(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<AssetVariableInput>(&payload, "input")?
        )),
        "asset_variable_delete" => boxed!(asset_variables::delete(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<String>(&payload, "id")?
        )),
        "asset_variable_refresh_page_globals" => {
            boxed!(asset_items::refresh_page_global_variables_for_user(
                pool,
                payload_field::<String>(&payload, "token")?
            ))
        }
        other => Err(AppError::BadRequest(format!("不支持的命令：{other}"))),
    }
}

fn payload_field<T: DeserializeOwned>(payload: &Value, key: &str) -> AppResult<T> {
    let value = payload
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::BadRequest(format!("缺少参数：{key}")))?;
    serde_json::from_value(value).map_err(|source| AppError::Json { source })
}

async fn authorize_broker_capability(
    state: &AppState,
    token: &str,
    capability: &str,
    target: String,
    scope: String,
) -> AppResult<()> {
    let user = auth::require_session(&state.pool, token).await?;
    let consent_granted = permission_consent::is_granted(
        &state.pool,
        &user.user_id,
        "platform.capability-broker",
        capability,
        &scope,
    )
    .await?;
    if !consent_granted {
        permission_approval::request_for_user(
            &state.pool,
            &user.user_id,
            PermissionApprovalRequestInput {
                source_id: "platform.capability-broker".to_string(),
                source_kind: "system".to_string(),
                capability: capability.to_string(),
                scope: scope.clone(),
                target: target.clone(),
                reason: format!("missing consent for runtime capability {capability}"),
            },
        )
        .await?;
    }
    let mut request = PermissionCore::system_request(
        user.user_id,
        "platform.capability-broker",
        capability,
        target,
        CapabilityBroker::supported_capabilities(),
    );
    request.scope = scope;
    request.consent_granted = consent_granted;
    let policies = plugin_registry::registry_from_workspace(
        plugin_registry::default_project_root(),
        &state.data_dir,
    )?
    .policies;
    state
        .permission_core
        .evaluate_runtime_with_policies(request, &policies)?;
    Ok(())
}

fn status_from(error: &AppError) -> StatusCode {
    match error {
        AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::Forbidden => StatusCode::FORBIDDEN,
        AppError::NotFound => StatusCode::NOT_FOUND,
        AppError::Conflict(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
