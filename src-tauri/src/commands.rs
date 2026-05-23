use serde::Serialize;
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
    asset_items::{
        self, AssetItemDeployInput, AssetItemDeployPreview, AssetItemDeployPreviewRequest,
        AssetItemImportRequest, AssetItemImportResult, AssetItemInput, AssetItemPageRequest,
        AssetItemRecord, AssetItemToggleInput, AssetItemUpdateInput, AssetVariableRefreshResult,
    },
    asset_variables::{self, AssetVariableInput, AssetVariablePageRequest, AssetVariableRecord},
    auth::{self, LoginRequest, LoginResult, UserInfo},
    dictionary::{
        self, DictItemInput, DictItemPageRequest, DictItemRecord, DictItemUpdateInput,
        DictTypeInput, DictTypeRecord, DictTypeUpdateInput,
    },
    error::{AppError, CommandError},
    notes::{self, NoteFlagInput, NoteInput, NotePageRequest, NoteRecord, NoteUpdateInput},
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
        auth::current_user(&state.pool, token).await?;
        app_paths::open_data_dir(&state.data_dir)
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
