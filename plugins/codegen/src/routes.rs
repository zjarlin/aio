//! nature revision REST API 与 SSR 表单入口。

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use az_aio_platform::core::api_error::{ApiError, ApiJson, ApiResponse, ok_json};
use serde::Deserialize;

use crate::{
    contract::{
        AcceptedNatureRevision, CreateNatureRevisionRequest, PROJECT_REVISIONS_PATH, REVISION_PATH,
        REVISION_PUBLISH_PATH, UI_ACTION_PATH,
    },
    service::NatureService,
};

/// nature API 共享状态。
#[derive(Clone)]
pub struct NatureApiState {
    service: Arc<NatureService>,
}

impl NatureApiState {
    pub fn new(service: NatureService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

pub fn nature_router(state: NatureApiState) -> Router {
    Router::new()
        .route(PROJECT_REVISIONS_PATH, post(create_revision_handler))
        .route(REVISION_PATH, get(get_revision_handler))
        .route(REVISION_PUBLISH_PATH, post(publish_revision_handler))
        .route(UI_ACTION_PATH, post(ui_action_handler))
        .with_state(state)
}

async fn create_revision_handler(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(project_id): Path<String>,
    State(state): State<NatureApiState>,
    ApiJson(request): ApiJson<CreateNatureRevisionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AcceptedNatureRevision>>), ApiError> {
    ensure_local_client(peer).map_err(ApiError::from)?;
    let revision = state
        .service
        .store()
        .create_revision(&project_id, &request.source_text)
        .await
        .map_err(ApiError::from)?;
    let revision_id = revision.id;
    spawn_generation(Arc::clone(&state.service), revision_id.clone());
    Ok((
        StatusCode::ACCEPTED,
        Json(ApiResponse {
            code: StatusCode::ACCEPTED.as_u16(),
            msg: "accepted".to_string(),
            data: Some(AcceptedNatureRevision {
                revision_id,
                status: "queued".to_string(),
            }),
        }),
    ))
}

async fn get_revision_handler(
    Path(revision_id): Path<String>,
    State(state): State<NatureApiState>,
) -> Result<Json<ApiResponse<crate::contract::NatureRevisionView>>, ApiError> {
    state
        .service
        .store()
        .revision_view(&revision_id)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn publish_revision_handler(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(revision_id): Path<String>,
    State(state): State<NatureApiState>,
) -> Result<Json<ApiResponse<crate::contract::PublishedNatureRevision>>, ApiError> {
    ensure_local_client(peer).map_err(ApiError::from)?;
    state
        .service
        .publish_revision(&revision_id, az_aio_nature_generated::ARTIFACT_HASH)
        .await
        .map(ok_json)
        .map_err(ApiError::from)
}

async fn ui_action_handler(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<NatureApiState>,
    Form(form): Form<NatureRevisionForm>,
) -> Response {
    let result = match ensure_local_client(peer) {
        Ok(()) => {
            state
                .service
                .store()
                .create_revision(&form.project_id, &form.source_text)
                .await
        }
        Err(error) => Err(error),
    };
    let route = match result {
        Ok(revision) => {
            let revision_id = revision.id;
            spawn_generation(Arc::clone(&state.service), revision_id.clone());
            format!("/nature?revision={}", urlencoding::encode(&revision_id))
        }
        Err(error) => format!("/nature?error={}", urlencoding::encode(&error.to_string())),
    };
    let redirect = format!("/?route={}", urlencoding::encode(&route));
    Redirect::to(&redirect).into_response()
}

fn spawn_generation(service: Arc<NatureService>, revision_id: String) {
    tokio::spawn(async move {
        if let Err(error) = service.generate_revision(revision_id).await {
            tracing::error!(error = %error, "nature 生成任务失败");
        }
    });
}

fn ensure_local_client(peer: SocketAddr) -> Result<()> {
    if peer.ip().is_loopback() {
        return Ok(());
    }
    Err(anyhow!("forbidden: 只有本机请求可以触发源码生成或发布"))
}

#[derive(Debug, Deserialize)]
struct NatureRevisionForm {
    project_id: String,
    source_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_peer_cannot_trigger_repository_writes() {
        let peer = SocketAddr::from(([192, 168, 1, 20], 18080));
        assert!(ensure_local_client(peer).is_err());
    }

    #[test]
    fn loopback_peer_can_submit_generation() {
        let peer = SocketAddr::from(([127, 0, 0, 1], 18080));
        assert!(ensure_local_client(peer).is_ok());
    }
}
