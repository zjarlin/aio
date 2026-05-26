use std::net::SocketAddr;

use axum::{
    extract::{Json, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use acp_openai_assistant::{
    ask_assistant, preview_page_context, AssistantChatRequest, AssistantKnowledgeContext,
    AssistantOpenAIConfig, PageContextInput,
};

use crate::{
    app_settings::{self, OpenAISettingsInput},
    agent_preferences::{
        self, AgentPreferenceInput, AgentPreferencePageRequest, AgentPreferenceToggleInput,
        AgentPreferenceUpdateInput,
    },
    app_paths,
    asset_items::{
        self, AssetItemDeployInput, AssetItemDeployPreviewRequest, AssetItemImportRequest,
        AssetItemInput, AssetItemPageRequest, AssetItemToggleInput, AssetItemUpdateInput,
    },
    asset_variables::{self, AssetVariableInput, AssetVariablePageRequest},
    auth::{self, LoginRequest},
    dictionary::{
        self, DictItemInput, DictItemPageRequest, DictItemUpdateInput, DictTypeInput,
        DictTypeUpdateInput,
    },
    dotfiles::{self, ComputerInput, DotfileSnapshotPageRequest},
    error::{AppError, AppResult},
    knowledge::{self, KnowledgeSearchRequest},
    notes::{self, NoteFlagInput, NoteInput, NotePageRequest, NoteUpdateInput},
    rbac::{
        self, AssignPermissionsInput, PageRequest, PermissionInput, RoleInput, RoleUpdateInput,
        UserInput, UserPasswordInput, UserUpdateInput,
    },
    skills::{self, SkillInput, SkillPageRequest, SkillToggleInput, SkillUpdateInput},
    state::AppState,
};

const BRIDGE_PORT: u16 = 18777;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeRequest {
    command: String,
    #[serde(default)]
    payload: Value,
}

pub fn spawn(state: AppState) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run(state).await {
            eprintln!("AIO browser command bridge failed: {error}");
        }
    });
}

async fn run(state: AppState) -> AppResult<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], BRIDGE_PORT));
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("AIO browser command bridge bind failed on {addr}: {error}");
            return Ok(());
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
            auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            serde_json::to_value(app_paths::open_data_dir(&state.data_dir)?)
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
            let user = auth::current_user(pool, payload_field::<String>(&payload, "token")?).await?;
            boxed!(async {
                let input = payload_field::<AssistantChatRequest>(&payload, "input")?;
                let retrieved =
                    knowledge::retrieve_for_user(pool, &user.user_id, &input.question, Some(6))
                        .await?;
                let knowledge_context = AssistantKnowledgeContext {
                    summary: knowledge::format_retrieved_chunks(&retrieved),
                };
                let settings = app_settings::resolve_openai_settings(pool).await?;
                ask_assistant(
                    input,
                    Some(knowledge_context),
                    Some(AssistantOpenAIConfig {
                        api_key: Some(settings.api_key),
                        base_url: settings.base_url,
                        model: settings.model,
                    }),
                )
                    .await
                    .map_err(|error| AppError::Assistant(error.to_string()))
            })
        }
        "openai_settings_get" => boxed!(app_settings::get_openai_settings(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "openai_settings_save" => boxed!(app_settings::save_openai_settings(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<OpenAISettingsInput>(&payload, "input")?
        )),
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
        "knowledge_search" => boxed!(knowledge::search(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<KnowledgeSearchRequest>(&payload, "request")?
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
        "skill_sync_sources" => boxed!(skills::sync_sources(
            pool,
            payload_field::<String>(&payload, "token")?
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
        "dotfile_computer_list" => boxed!(dotfiles::computer_list(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "dotfile_computer_upsert" => boxed!(dotfiles::computer_upsert(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<ComputerInput>(&payload, "input")?
        )),
        "dotfile_metadata_import" => boxed!(dotfiles::metadata_import(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        "dotfile_scan_computer" => boxed!(dotfiles::scan_computer(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_optional_field::<String>(&payload, "computerId")?
        )),
        "dotfile_snapshot_page" => boxed!(dotfiles::snapshot_page(
            pool,
            payload_field::<String>(&payload, "token")?,
            payload_field::<DotfileSnapshotPageRequest>(&payload, "request")?
        )),
        "dotfile_fusion_list" => boxed!(dotfiles::fusion_list(
            pool,
            payload_field::<String>(&payload, "token")?
        )),
        other => Err(AppError::BadRequest(format!("不支持的命令：{other}"))),
    }
}

fn payload_optional_field<T: DeserializeOwned>(
    payload: &Value,
    key: &str,
) -> AppResult<Option<T>> {
    match payload.get(key).cloned() {
        Some(value) if !value.is_null() => {
            serde_json::from_value(value).map(Some).map_err(|source| AppError::Json { source })
        }
        _ => Ok(None),
    }
}

fn payload_field<T: DeserializeOwned>(payload: &Value, key: &str) -> AppResult<T> {
    let value = payload
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::BadRequest(format!("缺少参数：{key}")))?;
    serde_json::from_value(value).map_err(|source| AppError::Json { source })
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
