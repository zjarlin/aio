use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use az_plugin_core::http::{ApiError, ApiJson, ApiResponse, ok_json};
use az_asset_hub_contract::{AssetSummary, AssetUpsertInput, ScannedSkillSummary};
use serde::{Deserialize, Serialize};

use crate::backend::{
    model::TABLE_NAME_PREFIX,
    skill_scanner::scan_skill_assets,
    store::AssetHubStore,
};

#[derive(Clone)]
pub struct AssetHubApiState {
    database_url: Option<String>,
    store: Option<AssetHubStore>,
}

impl AssetHubApiState {
    pub fn degraded(database_url: Option<String>) -> Self {
        Self {
            database_url,
            store: None,
        }
    }

    pub fn from_store(database_url: Option<String>, store: Option<AssetHubStore>) -> Self {
        Self {
            database_url,
            store,
        }
    }

    pub fn status(&self) -> AssetHubStatusResponse {
        AssetHubStatusResponse {
            ok: true,
            database_configured: self
                .database_url
                .as_ref()
                .is_some_and(|value| !value.is_empty()),
            store_connected: self.store.is_some(),
            table_prefix: TABLE_NAME_PREFIX.to_string(),
        }
    }

    pub fn store(&self) -> Option<AssetHubStore> {
        self.store.clone()
    }
}

pub fn asset_hub_router(state: AssetHubApiState) -> Router {
    Router::new()
        .route("/api/asset-hub/status", get(status_handler))
        .route("/api/asset-hub/skills", get(scan_skills_handler))
        .route("/api/asset-hub/assets", get(list_assets_handler))
        .route("/api/asset-hub/asset", post(upsert_asset_handler))
        .with_state(state)
}

async fn status_handler(State(state): State<AssetHubApiState>) -> Json<AssetHubStatusResponse> {
    Json(state.status())
}

async fn scan_skills_handler() -> Result<Json<ApiResponse<Vec<ScannedSkillSummary>>>, Response> {
    scan_skill_assets()
        .map(ok_json)
        .map_err(asset_hub_error_response)
}

async fn list_assets_handler(
    State(state): State<AssetHubApiState>,
) -> Result<Json<ApiResponse<Vec<AssetSummary>>>, Response> {
    let store = state
        .store
        .ok_or_else(|| anyhow!("missing asset-hub database url"))
        .map_err(asset_hub_error_response)?;
    store
        .list_assets()
        .await
        .map(ok_json)
        .map_err(asset_hub_error_response)
}

async fn upsert_asset_handler(
    State(state): State<AssetHubApiState>,
    ApiJson(request): ApiJson<AssetUpsertInput>,
) -> Result<Json<ApiResponse<AssetSummary>>, Response> {
    let store = state
        .store
        .ok_or_else(|| anyhow!("missing asset-hub database url"))
        .map_err(asset_hub_error_response)?;
    store
        .upsert_asset(request)
        .await
        .map(ok_json)
        .map_err(asset_hub_error_response)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetHubStatusResponse {
    pub ok: bool,
    pub database_configured: bool,
    pub store_connected: bool,
    pub table_prefix: String,
}

fn asset_hub_error_response(error: anyhow::Error) -> Response {
    ApiError::from(error).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn degraded_state_reports_disconnected_store() {
        let state = AssetHubApiState::degraded(None);
        assert!(state.store.is_none());
        assert!(state.database_url.is_none());
    }
}
