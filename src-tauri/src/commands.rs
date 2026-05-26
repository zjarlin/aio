use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

use acp_openai_assistant::{
    ask_assistant, preview_page_context, AssistantAnswer, AssistantChatRequest, PageContextInput,
    PageContextPreview,
};

use crate::{
    agent_preferences::{
        self, AgentPreferenceInput, AgentPreferencePageRequest, AgentPreferenceRecord,
        AgentPreferenceToggleInput, AgentPreferenceUpdateInput,
    },
    app_paths,
    app_runtime::{
        AppRuntimeLifecycleRecord, AppRuntimeReloadInput, AppRuntimeSessionInput,
        AppRuntimeSnapshot, AppRuntimeStartInput, AppRuntimeStopInput, AppRuntimeWorkspaceInput,
    },
    asset_items::{
        self, AssetItemDeployInput, AssetItemDeployPreview, AssetItemDeployPreviewRequest,
        AssetItemImportRequest, AssetItemImportResult, AssetItemInput, AssetItemPageRequest,
        AssetItemRecord, AssetItemToggleInput, AssetItemUpdateInput, AssetVariableRefreshResult,
    },
    asset_variables::{self, AssetVariableInput, AssetVariablePageRequest, AssetVariableRecord},
    auth::{self, LoginRequest, LoginResult, UserInfo},
    capability_broker::{
        BrowserOpenUrlInput, CapabilityAuditRecord, CapabilityInvokeInput, ClipboardWriteInput,
        FsReadInput, FsReadResult, FsWriteInput, FsWriteResult, NotificationSendInput,
        ProcessExecInput, ProcessExecResult,
    },
    dictionary::{
        self, DictItemInput, DictItemPageRequest, DictItemRecord, DictItemUpdateInput,
        DictTypeInput, DictTypeRecord, DictTypeUpdateInput,
    },
    error::{AppError, CommandError},
    event_bus::{
        EventBusPublishInput, EventBusSnapshotRequest, EventBusStreamInput, PlatformEventRecord,
    },
    extension_host::{ExtensionHostPluginRecord, ExtensionHostSourceInput},
    notes::{self, NoteFlagInput, NoteInput, NotePageRequest, NoteRecord, NoteUpdateInput},
    permission_approval::{
        self, PermissionApprovalDecisionInput, PermissionApprovalRecord,
        PermissionApprovalRequestInput,
    },
    permission_consent::{
        self, PermissionConsentGrantInput, PermissionConsentRecord, PermissionConsentRevokeInput,
    },
    permission_core::{PermissionCore, PermissionDecisionRecord},
    plugin_factory::{
        self, PluginCreateFromPromptInput, PluginDraftCreationResult, PluginPublishGateReport,
        PluginRepairFromDiagnosticsInput, PluginVerificationReport, PluginVerifyDraftInput,
    },
    plugin_registry::{self, PluginRegistrySnapshot},
    plugin_store::{
        ChildCapabilityApprovalInput, ChildCapabilityApprovalRecord, InstalledPluginRecord,
        PluginRegistryLocalState, PluginRegistryRollbackInput, PluginRegistryRollbackResult,
        PluginRegistryStore, RegistryAuditRecord,
    },
    rbac::{
        self, AssignPermissionsInput, PageRequest, PageResult, PermissionInput, PermissionNode,
        RoleInput, RoleRecord, RoleUpdateInput, RouteMenu, UserInput, UserPasswordInput,
        UserRecord, UserUpdateInput,
    },
    skills::{self, SkillInput, SkillPageRequest, SkillRecord, SkillToggleInput, SkillUpdateInput},
    state::AppState,
};

type CommandResult<T> = Result<T, CommandError>;

fn map_result<T>(result: Result<T, AppError>) -> CommandResult<T> {
    result.map_err(AppError::into_command_error)
}

async fn authorize_broker_capability(
    state: &AppState,
    token: &str,
    capability: &str,
    target: String,
    scope: String,
) -> Result<(), AppError> {
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
        crate::capability_broker::CapabilityBroker::supported_capabilities(),
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

fn publish_runtime_lifecycle(
    state: &AppState,
    record: AppRuntimeLifecycleRecord,
) -> Result<AppRuntimeLifecycleRecord, AppError> {
    state.event_bus.publish(record.event_input())?;
    Ok(record)
}

#[tauri::command]
pub async fn app_runtime_snapshot(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<AppRuntimeSnapshot> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        Ok(state.app_runtime.snapshot())
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn app_runtime_start(
    state: State<'_, AppState>,
    token: String,
    input: AppRuntimeStartInput,
) -> CommandResult<AppRuntimeLifecycleRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        let record = state.app_runtime.start(input)?;
        publish_runtime_lifecycle(&state, record)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn app_runtime_stop(
    state: State<'_, AppState>,
    token: String,
    input: AppRuntimeStopInput,
) -> CommandResult<AppRuntimeLifecycleRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        let record = state.app_runtime.stop(input)?;
        publish_runtime_lifecycle(&state, record)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn app_runtime_reload(
    state: State<'_, AppState>,
    token: String,
    input: AppRuntimeReloadInput,
) -> CommandResult<AppRuntimeLifecycleRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        let record = state.app_runtime.reload(input)?;
        publish_runtime_lifecycle(&state, record)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn app_runtime_workspace(
    state: State<'_, AppState>,
    token: String,
    input: AppRuntimeWorkspaceInput,
) -> CommandResult<AppRuntimeLifecycleRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        let record = state.app_runtime.set_workspace(input)?;
        publish_runtime_lifecycle(&state, record)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn app_runtime_session(
    state: State<'_, AppState>,
    token: String,
    input: AppRuntimeSessionInput,
) -> CommandResult<AppRuntimeLifecycleRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        let record = state.app_runtime.set_session(input)?;
        publish_runtime_lifecycle(&state, record)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn openai_assistant_preview_context(
    state: State<'_, AppState>,
    token: String,
    input: PageContextInput,
) -> CommandResult<PageContextPreview> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        preview_page_context(&input)
            .await
            .map_err(|error| AppError::Assistant(error.to_string()))
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn openai_assistant_chat(
    state: State<'_, AppState>,
    token: String,
    input: AssistantChatRequest,
) -> CommandResult<AssistantAnswer> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        ask_assistant(input)
            .await
            .map_err(|error| AppError::Assistant(error.to_string()))
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn agent_preference_page(
    state: State<'_, AppState>,
    token: String,
    request: AgentPreferencePageRequest,
) -> CommandResult<PageResult<AgentPreferenceRecord>> {
    map_result(agent_preferences::page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn agent_preference_create(
    state: State<'_, AppState>,
    token: String,
    input: AgentPreferenceInput,
) -> CommandResult<AgentPreferenceRecord> {
    map_result(agent_preferences::create(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn agent_preference_update(
    state: State<'_, AppState>,
    token: String,
    input: AgentPreferenceUpdateInput,
) -> CommandResult<AgentPreferenceRecord> {
    map_result(agent_preferences::update(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn agent_preference_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(agent_preferences::delete(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn agent_preference_toggle(
    state: State<'_, AppState>,
    token: String,
    input: AgentPreferenceToggleInput,
) -> CommandResult<AgentPreferenceRecord> {
    map_result(agent_preferences::toggle(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn app_open_data_dir(state: State<'_, AppState>, token: String) -> CommandResult<String> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "browser.openDirectory",
            state.data_dir.to_string_lossy().into_owned(),
            state.data_dir.to_string_lossy().into_owned(),
        )
        .await?;
        app_paths::open_data_dir(&state.data_dir, &state.capability_broker)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_audit_log(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<CapabilityAuditRecord>> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        Ok(state.capability_broker.audit_log())
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_clipboard_write(
    state: State<'_, AppState>,
    token: String,
    input: ClipboardWriteInput,
) -> CommandResult<usize> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "clipboard.write",
            format!("{} bytes", input.text.len()),
            "system-clipboard".to_string(),
        )
        .await?;
        state.capability_broker.write_clipboard(&input.text)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_clipboard_read(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<String> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "clipboard.read",
            "system-clipboard".to_string(),
            "system-clipboard".to_string(),
        )
        .await?;
        state.capability_broker.read_clipboard()
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_notification_send(
    state: State<'_, AppState>,
    token: String,
    input: NotificationSendInput,
) -> CommandResult<()> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "notification.send",
            input.title.clone(),
            input.title.clone(),
        )
        .await?;
        state
            .capability_broker
            .send_notification(&input.title, &input.body)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_fs_read(
    state: State<'_, AppState>,
    token: String,
    input: FsReadInput,
) -> CommandResult<FsReadResult> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "fs.read",
            input.path.clone(),
            input.path.clone(),
        )
        .await?;
        state.capability_broker.read_text_file(&input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_fs_write(
    state: State<'_, AppState>,
    token: String,
    input: FsWriteInput,
) -> CommandResult<FsWriteResult> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "fs.write",
            input.path.clone(),
            input.path.clone(),
        )
        .await?;
        state.capability_broker.write_text_file(&input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_process_exec(
    state: State<'_, AppState>,
    token: String,
    input: ProcessExecInput,
) -> CommandResult<ProcessExecResult> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "process.exec",
            input.command.clone(),
            input.command.clone(),
        )
        .await?;
        state.capability_broker.run_process(&input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_browser_open_url(
    state: State<'_, AppState>,
    token: String,
    input: BrowserOpenUrlInput,
) -> CommandResult<String> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            "browser.openUrl",
            input.url.clone(),
            input.url.clone(),
        )
        .await?;
        state.capability_broker.open_url(&input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn capability_invoke(
    state: State<'_, AppState>,
    token: String,
    input: CapabilityInvokeInput,
) -> CommandResult<Value> {
    let result = async {
        authorize_broker_capability(
            &state,
            &token,
            &input.capability,
            input.audit_target(),
            input.permission_scope(),
        )
        .await?;
        state.capability_broker.invoke_json(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_audit_log(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<PermissionDecisionRecord>> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        let consent_granted = permission_consent::is_granted(
            &state.pool,
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
        Ok(state.permission_core.audit_log())
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_consent_list(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<PermissionConsentRecord>> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_consent::list_for_user(&state.pool, &user.user_id).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_consent_grant(
    state: State<'_, AppState>,
    token: String,
    input: PermissionConsentGrantInput,
) -> CommandResult<PermissionConsentRecord> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_consent::grant_for_user(&state.pool, &user.user_id, input).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_consent_revoke(
    state: State<'_, AppState>,
    token: String,
    input: PermissionConsentRevokeInput,
) -> CommandResult<PermissionConsentRecord> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_consent::revoke_for_user(&state.pool, &user.user_id, input).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_approval_list(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<PermissionApprovalRecord>> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_approval::list_for_user(&state.pool, &user.user_id).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_approval_approve(
    state: State<'_, AppState>,
    token: String,
    input: PermissionApprovalDecisionInput,
) -> CommandResult<PermissionApprovalRecord> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_approval::approve_for_user(&state.pool, &user.user_id, input).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn permission_approval_deny(
    state: State<'_, AppState>,
    token: String,
    input: PermissionApprovalDecisionInput,
) -> CommandResult<PermissionApprovalRecord> {
    let result = async {
        let user = auth::require_session(&state.pool, &token).await?;
        permission_approval::deny_for_user(&state.pool, &user.user_id, input).await
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn event_bus_publish(
    state: State<'_, AppState>,
    token: String,
    input: EventBusPublishInput,
) -> CommandResult<PlatformEventRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.event_bus.publish(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn event_bus_stream(
    state: State<'_, AppState>,
    token: String,
    input: EventBusStreamInput,
) -> CommandResult<PlatformEventRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.event_bus.stream(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn event_bus_snapshot(
    state: State<'_, AppState>,
    token: String,
    request: EventBusSnapshotRequest,
) -> CommandResult<Vec<PlatformEventRecord>> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        Ok(state
            .event_bus
            .snapshot(request.event_type.as_deref(), request.limit))
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_load(
    state: State<'_, AppState>,
    token: String,
    input: ExtensionHostSourceInput,
) -> CommandResult<ExtensionHostPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.extension_host.load(input.source_path)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_activate(
    state: State<'_, AppState>,
    token: String,
    plugin_id: String,
) -> CommandResult<ExtensionHostPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.extension_host.activate(&plugin_id)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_deactivate(
    state: State<'_, AppState>,
    token: String,
    plugin_id: String,
) -> CommandResult<ExtensionHostPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.extension_host.deactivate(&plugin_id)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_reload(
    state: State<'_, AppState>,
    token: String,
    input: ExtensionHostSourceInput,
) -> CommandResult<ExtensionHostPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.extension_host.reload(input.source_path)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_dispose(
    state: State<'_, AppState>,
    token: String,
    plugin_id: String,
) -> CommandResult<ExtensionHostPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        state.extension_host.dispose(&plugin_id)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_host_snapshot(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<ExtensionHostPluginRecord>> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        Ok(state.extension_host.snapshot())
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> CommandResult<LoginResult> {
    map_result(auth::login(&state.pool, request).await)
}

#[tauri::command]
pub async fn auth_logout(state: State<'_, AppState>, token: String) -> CommandResult<()> {
    map_result(auth::logout(&state.pool, token).await)
}

#[tauri::command]
pub async fn auth_current_user(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<UserInfo> {
    map_result(auth::current_user(&state.pool, token).await)
}

#[tauri::command]
pub async fn auth_access_codes(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<String>> {
    map_result(rbac::access_codes(&state.pool, token).await)
}

#[tauri::command]
pub async fn menu_list(state: State<'_, AppState>, token: String) -> CommandResult<Vec<RouteMenu>> {
    map_result(rbac::menus(&state.pool, token).await)
}

#[tauri::command]
pub async fn plugin_registry_snapshot(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<PluginRegistrySnapshot> {
    map_result(plugin_registry::snapshot(&state.pool, token, &state.data_dir).await)
}

#[tauri::command]
pub async fn plugin_registry_reload(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<PluginRegistrySnapshot> {
    let result = async {
        let snapshot = plugin_registry::snapshot(&state.pool, token, &state.data_dir).await?;
        let detail = json!({
            "plugins": snapshot.plugins.len(),
            "systemCapsules": snapshot.system_capsules.len(),
            "commands": snapshot.commands.len(),
            "routes": snapshot.routes.len(),
            "views": snapshot.views.len(),
        });
        PluginRegistryStore::new(&state.data_dir).append_audit(RegistryAuditRecord {
            action: "reload".to_string(),
            content_hash: None,
            detail: Some(detail.to_string()),
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
        })?;
        state.event_bus.publish(EventBusPublishInput {
            event_type: "registry.reloaded".to_string(),
            payload: detail,
            source: "platform.registry".to_string(),
            target: None,
            parent_trace_id: None,
            permissions: None,
            schema: Some("schemas/events/registry-reloaded.v1.schema.json".to_string()),
        })?;
        Ok(snapshot)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_local_state(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<PluginRegistryLocalState> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).state()
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_child_capability_approve(
    state: State<'_, AppState>,
    token: String,
    input: ChildCapabilityApprovalInput,
) -> CommandResult<ChildCapabilityApprovalRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).approve_child_capability(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_child_capability_revoke(
    state: State<'_, AppState>,
    token: String,
    input: ChildCapabilityApprovalInput,
) -> CommandResult<ChildCapabilityApprovalRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).revoke_child_capability(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_install(
    state: State<'_, AppState>,
    token: String,
    source_path: String,
) -> CommandResult<InstalledPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).install(source_path)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_enable(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<InstalledPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).set_enabled(&id, true)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_disable(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<InstalledPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).set_enabled(&id, false)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_uninstall(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).uninstall(&id)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_registry_rollback(
    state: State<'_, AppState>,
    token: String,
    input: PluginRegistryRollbackInput,
) -> CommandResult<PluginRegistryRollbackResult> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        PluginRegistryStore::new(&state.data_dir).rollback(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_create_from_prompt(
    state: State<'_, AppState>,
    token: String,
    input: PluginCreateFromPromptInput,
) -> CommandResult<PluginDraftCreationResult> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        plugin_factory::create_from_prompt_and_write(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_publish_local(
    state: State<'_, AppState>,
    token: String,
    source_path: String,
) -> CommandResult<InstalledPluginRecord> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        plugin_factory::publish_local(source_path, &state.data_dir)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_publish_gate(
    state: State<'_, AppState>,
    token: String,
    source_path: String,
    write: bool,
) -> CommandResult<PluginPublishGateReport> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        plugin_factory::publish_gate(source_path, &state.data_dir, write)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_repair_from_diagnostics(
    state: State<'_, AppState>,
    token: String,
    input: PluginRepairFromDiagnosticsInput,
) -> CommandResult<PluginDraftCreationResult> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        plugin_factory::repair_from_diagnostics(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn plugin_verify_draft(
    state: State<'_, AppState>,
    token: String,
    input: PluginVerifyDraftInput,
) -> CommandResult<PluginVerificationReport> {
    let result = async {
        auth::current_user(&state.pool, token).await?;
        plugin_factory::verify_plugin_draft(input)
    }
    .await;
    map_result(result)
}

#[tauri::command]
pub async fn user_page(
    state: State<'_, AppState>,
    token: String,
    request: PageRequest,
) -> CommandResult<PageResult<UserRecord>> {
    map_result(rbac::user_page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn user_create(
    state: State<'_, AppState>,
    token: String,
    input: UserInput,
) -> CommandResult<UserRecord> {
    map_result(rbac::create_user(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn user_update(
    state: State<'_, AppState>,
    token: String,
    input: UserUpdateInput,
) -> CommandResult<UserRecord> {
    map_result(rbac::update_user(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn user_disable(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<UserRecord> {
    map_result(rbac::disable_user(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn user_reset_password(
    state: State<'_, AppState>,
    token: String,
    input: UserPasswordInput,
) -> CommandResult<()> {
    map_result(rbac::reset_password(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn user_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(rbac::delete_user(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn role_page(
    state: State<'_, AppState>,
    token: String,
    request: PageRequest,
) -> CommandResult<PageResult<RoleRecord>> {
    map_result(rbac::role_page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn role_create(
    state: State<'_, AppState>,
    token: String,
    input: RoleInput,
) -> CommandResult<RoleRecord> {
    map_result(rbac::create_role(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn role_update(
    state: State<'_, AppState>,
    token: String,
    input: RoleUpdateInput,
) -> CommandResult<RoleRecord> {
    map_result(rbac::update_role(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn role_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(rbac::delete_role(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn role_assign_permissions(
    state: State<'_, AppState>,
    token: String,
    input: AssignPermissionsInput,
) -> CommandResult<()> {
    map_result(rbac::assign_permissions(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn role_permission_ids(
    state: State<'_, AppState>,
    token: String,
    role_id: String,
) -> CommandResult<Vec<String>> {
    map_result(rbac::role_permission_ids(&state.pool, token, role_id).await)
}

#[tauri::command]
pub async fn permission_tree(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<Vec<PermissionNode>> {
    map_result(rbac::permission_tree(&state.pool, token).await)
}

#[tauri::command]
pub async fn permission_save(
    state: State<'_, AppState>,
    token: String,
    input: PermissionInput,
) -> CommandResult<PermissionNode> {
    map_result(rbac::save_permission(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn dict_type_page(
    state: State<'_, AppState>,
    token: String,
    request: PageRequest,
) -> CommandResult<PageResult<DictTypeRecord>> {
    map_result(dictionary::type_page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn dict_type_create(
    state: State<'_, AppState>,
    token: String,
    input: DictTypeInput,
) -> CommandResult<DictTypeRecord> {
    map_result(dictionary::create_type(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn dict_type_update(
    state: State<'_, AppState>,
    token: String,
    input: DictTypeUpdateInput,
) -> CommandResult<DictTypeRecord> {
    map_result(dictionary::update_type(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn dict_type_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(dictionary::delete_type(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn dict_item_page(
    state: State<'_, AppState>,
    token: String,
    request: DictItemPageRequest,
) -> CommandResult<PageResult<DictItemRecord>> {
    map_result(dictionary::item_page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn dict_item_create(
    state: State<'_, AppState>,
    token: String,
    input: DictItemInput,
) -> CommandResult<DictItemRecord> {
    map_result(dictionary::create_item(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn dict_item_update(
    state: State<'_, AppState>,
    token: String,
    input: DictItemUpdateInput,
) -> CommandResult<DictItemRecord> {
    map_result(dictionary::update_item(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn dict_item_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(dictionary::delete_item(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn note_page(
    state: State<'_, AppState>,
    token: String,
    request: NotePageRequest,
) -> CommandResult<PageResult<NoteRecord>> {
    map_result(notes::page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn note_create(
    state: State<'_, AppState>,
    token: String,
    input: NoteInput,
) -> CommandResult<NoteRecord> {
    map_result(notes::create(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn note_update(
    state: State<'_, AppState>,
    token: String,
    input: NoteUpdateInput,
) -> CommandResult<NoteRecord> {
    map_result(notes::update(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn note_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(notes::delete(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn note_archive(
    state: State<'_, AppState>,
    token: String,
    input: NoteFlagInput,
) -> CommandResult<NoteRecord> {
    map_result(notes::set_archived(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn note_favorite(
    state: State<'_, AppState>,
    token: String,
    input: NoteFlagInput,
) -> CommandResult<NoteRecord> {
    map_result(notes::set_favorite(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn skill_page(
    state: State<'_, AppState>,
    token: String,
    request: SkillPageRequest,
) -> CommandResult<PageResult<SkillRecord>> {
    map_result(skills::page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn skill_create(
    state: State<'_, AppState>,
    token: String,
    input: SkillInput,
) -> CommandResult<SkillRecord> {
    map_result(skills::create(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn skill_update(
    state: State<'_, AppState>,
    token: String,
    input: SkillUpdateInput,
) -> CommandResult<SkillRecord> {
    map_result(skills::update(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn skill_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(skills::delete(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn skill_toggle(
    state: State<'_, AppState>,
    token: String,
    input: SkillToggleInput,
) -> CommandResult<SkillRecord> {
    map_result(skills::toggle(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_item_page(
    state: State<'_, AppState>,
    token: String,
    request: AssetItemPageRequest,
) -> CommandResult<PageResult<AssetItemRecord>> {
    map_result(asset_items::page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn asset_item_import_directory(
    state: State<'_, AppState>,
    token: String,
    request: AssetItemImportRequest,
) -> CommandResult<AssetItemImportResult> {
    map_result(asset_items::import_directory(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn asset_item_create(
    state: State<'_, AppState>,
    token: String,
    input: AssetItemInput,
) -> CommandResult<AssetItemRecord> {
    map_result(asset_items::create(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_item_update(
    state: State<'_, AppState>,
    token: String,
    input: AssetItemUpdateInput,
) -> CommandResult<AssetItemRecord> {
    map_result(asset_items::update(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_item_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(asset_items::delete(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn asset_item_toggle(
    state: State<'_, AppState>,
    token: String,
    input: AssetItemToggleInput,
) -> CommandResult<AssetItemRecord> {
    map_result(asset_items::toggle(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_item_deploy_preview(
    state: State<'_, AppState>,
    token: String,
    request: AssetItemDeployPreviewRequest,
) -> CommandResult<AssetItemDeployPreview> {
    map_result(asset_items::deploy_preview(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn asset_item_deploy_save(
    state: State<'_, AppState>,
    token: String,
    input: AssetItemDeployInput,
) -> CommandResult<AssetItemRecord> {
    map_result(asset_items::deploy_save(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_variable_page(
    state: State<'_, AppState>,
    token: String,
    request: AssetVariablePageRequest,
) -> CommandResult<PageResult<AssetVariableRecord>> {
    map_result(asset_variables::page(&state.pool, token, request).await)
}

#[tauri::command]
pub async fn asset_variable_upsert(
    state: State<'_, AppState>,
    token: String,
    input: AssetVariableInput,
) -> CommandResult<AssetVariableRecord> {
    map_result(asset_variables::upsert(&state.pool, token, input).await)
}

#[tauri::command]
pub async fn asset_variable_delete(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> CommandResult<()> {
    map_result(asset_variables::delete(&state.pool, token, id).await)
}

#[tauri::command]
pub async fn asset_variable_refresh_page_globals(
    state: State<'_, AppState>,
    token: String,
) -> CommandResult<AssetVariableRefreshResult> {
    map_result(asset_items::refresh_page_global_variables_for_user(&state.pool, token).await)
}

fn _assert_serializable<T: Serialize>() {}
