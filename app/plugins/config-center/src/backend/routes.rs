use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use anyhow::{Context, anyhow};
use az_plugin_core::http::{
    ApiError, ApiForm, ApiJson, ApiQuery, ApiResponse, ok_json,
};
use az_config_center_contract::{
    ConfigCenterStatus, ConfigEntrySummary, ConfigEntryUpsertInput, DotfilesMonitorStatus,
    PairingLocalInfo,
};
use serde::Deserialize;

use crate::{
    backend::{
        dotfiles_monitor::scan_dotfiles_status,
        model::TABLE_NAME_PREFIX,
        pairing::{ensure_local_pairing_device_info, local_pairing_info},
        paths::resolve_config_center_paths,
        store::ConfigCenterStore,
    },
};

#[derive(Clone)]
pub struct ConfigCenterApiState {
    database_url: Option<String>,
    store: Option<ConfigCenterStore>,
}

impl ConfigCenterApiState {
    pub fn degraded(database_url: Option<String>) -> Self {
        Self {
            database_url,
            store: None,
        }
    }

    pub fn from_store(database_url: Option<String>, store: Option<ConfigCenterStore>) -> Self {
        Self {
            database_url,
            store,
        }
    }

    pub fn status(&self) -> anyhow::Result<ConfigCenterStatus> {
        let paths = resolve_config_center_paths()?;
        Ok(ConfigCenterStatus {
            ok: true,
            database_configured: self.database_url.as_ref().is_some_and(|value| !value.is_empty()),
            store_connected: self.store.is_some(),
            table_prefix: TABLE_NAME_PREFIX.to_string(),
            paths,
        })
    }

    pub fn store(&self) -> Option<ConfigCenterStore> {
        self.store.clone()
    }
}

pub fn config_center_router(state: ConfigCenterApiState) -> Router {
    Router::new()
        .route("/api/config-center/status", get(status_handler))
        .route("/api/config-center/dotfiles", get(dotfiles_handler))
        .route("/api/config-center/pairing", get(pairing_handler))
        .route("/api/config-center/entries", get(list_entries_handler))
        .route("/api/config-center/entry", post(upsert_entry_handler))
        .route("/api/config-center/ui-action", post(ui_action_handler))
        .with_state(state)
}

async fn status_handler(
    State(state): State<ConfigCenterApiState>,
) -> Result<Json<ApiResponse<ConfigCenterStatus>>, Response> {
    state
        .status()
        .map(ok_json)
        .map_err(config_center_error_response)
}

async fn dotfiles_handler(
) -> Result<Json<ApiResponse<DotfilesMonitorStatus>>, Response> {
    scan_dotfiles_status()
        .map(ok_json)
        .map_err(config_center_error_response)
}

async fn pairing_handler() -> Result<Json<ApiResponse<PairingLocalInfo>>, Response> {
    ensure_local_pairing_device_info().map_err(config_center_error_response)?;
    local_pairing_info()
        .map(ok_json)
        .map_err(config_center_error_response)
}

async fn list_entries_handler(
    State(state): State<ConfigCenterApiState>,
    ApiQuery(query): ApiQuery<ListEntriesQuery>,
) -> Result<Json<ApiResponse<Vec<ConfigEntrySummary>>>, Response> {
    let store = state
        .store
        .ok_or_else(|| anyhow!("missing config-center database url"))
        .map_err(config_center_error_response)?;
    store
        .list_entries(query.namespace.as_deref().unwrap_or("az-aio"))
        .await
        .map(ok_json)
        .map_err(config_center_error_response)
}

async fn upsert_entry_handler(
    State(state): State<ConfigCenterApiState>,
    ApiJson(request): ApiJson<ConfigEntryUpsertInput>,
) -> Result<Json<ApiResponse<ConfigEntrySummary>>, Response> {
    let store = state
        .store
        .ok_or_else(|| anyhow!("missing config-center database url"))
        .map_err(config_center_error_response)?;
    store
        .upsert_entry(request)
        .await
        .map(ok_json)
        .map_err(config_center_error_response)
}

async fn ui_action_handler(
    State(state): State<ConfigCenterApiState>,
    ApiForm(form): ApiForm<ConfigEntryUpsertInput>,
) -> Response {
    let redirect = match apply_ui_action(state, form).await {
        Ok(()) => "/app/config".to_string(),
        Err(error) => format!(
            "/app/config?error={}",
            urlencoding::encode(&error.to_string())
        ),
    };
    Redirect::to(&redirect).into_response()
}

async fn apply_ui_action(
    state: ConfigCenterApiState,
    request: ConfigEntryUpsertInput,
) -> anyhow::Result<()> {
    let store = state
        .store
        .context("missing config-center database url")?;
    store.upsert_entry(request).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ListEntriesQuery {
    pub namespace: Option<String>,
}

fn config_center_error_response(error: anyhow::Error) -> Response {
    ApiError::from(error).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn degraded_state_reports_disconnected_store() {
        let state = ConfigCenterApiState::degraded(None);
        assert!(state.store.is_none());
        assert!(state.database_url.is_none());
    }
}
