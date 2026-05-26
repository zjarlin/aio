use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
    event_bus::EventBusPublishInput,
    permission_core::current_platform,
};

const RUNTIME_EVENT_SCHEMA: &str = "schemas/events/platform-runtime-lifecycle.v1.schema.json";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeStartInput {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeStopInput {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeReloadInput {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeWorkspaceInput {
    pub workspace: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionInput {
    pub session_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeLifecycleRecord {
    pub action: String,
    pub status: String,
    pub workspace: Option<String>,
    pub session_id: Option<String>,
    pub mode: String,
    pub data_dir: String,
    pub platform_id: String,
    pub reload_count: u32,
    pub reason: String,
    pub timestamp: i64,
}

impl AppRuntimeLifecycleRecord {
    pub fn event_input(&self) -> EventBusPublishInput {
        EventBusPublishInput {
            event_type: "platform.runtime.lifecycle".to_string(),
            payload: json!({
                "action": self.action,
                "status": self.status,
                "workspace": self.workspace,
                "sessionId": self.session_id,
                "mode": self.mode,
                "dataDir": self.data_dir,
                "platformId": self.platform_id,
                "reloadCount": self.reload_count,
                "reason": self.reason,
            }),
            source: "platform.runtime".to_string(),
            target: None,
            parent_trace_id: None,
            permissions: None,
            schema: Some(RUNTIME_EVENT_SCHEMA.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSnapshot {
    pub schema_version: String,
    pub status: String,
    pub workspace: Option<String>,
    pub session_id: Option<String>,
    pub mode: String,
    pub data_dir: String,
    pub platform_id: String,
    pub reload_count: u32,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub updated_at: i64,
    pub lifecycle: Vec<AppRuntimeLifecycleRecord>,
}

#[derive(Debug, Clone)]
struct AppRuntimeState {
    status: String,
    workspace: Option<String>,
    session_id: Option<String>,
    mode: String,
    data_dir: String,
    platform_id: String,
    reload_count: u32,
    started_at: Option<i64>,
    stopped_at: Option<i64>,
    updated_at: i64,
    lifecycle: Vec<AppRuntimeLifecycleRecord>,
}

#[derive(Clone)]
pub struct AppRuntime {
    state: Arc<Mutex<AppRuntimeState>>,
}

impl AppRuntime {
    pub fn new(data_dir: PathBuf) -> Self {
        let now = now_millis();
        Self {
            state: Arc::new(Mutex::new(AppRuntimeState {
                status: "initialized".to_string(),
                workspace: None,
                session_id: None,
                mode: "desktop".to_string(),
                data_dir: data_dir.to_string_lossy().into_owned(),
                platform_id: current_platform().to_string(),
                reload_count: 0,
                started_at: None,
                stopped_at: None,
                updated_at: now,
                lifecycle: Vec::new(),
            })),
        }
    }

    pub fn start(&self, input: AppRuntimeStartInput) -> AppResult<AppRuntimeLifecycleRecord> {
        let mut state = self.lock_state()?;
        let now = now_millis();
        state.status = "running".to_string();
        if let Some(workspace) = non_empty(input.workspace) {
            state.workspace = Some(workspace);
        }
        if let Some(session_id) = non_empty(input.session_id) {
            state.session_id = Some(session_id);
        }
        if let Some(mode) = non_empty(input.mode) {
            state.mode = mode;
        }
        state.started_at = Some(state.started_at.unwrap_or(now));
        state.stopped_at = None;
        state.updated_at = now;
        let record = lifecycle_record(&state, "start", input.reason, now);
        state.lifecycle.push(record.clone());
        Ok(record)
    }

    pub fn stop(&self, input: AppRuntimeStopInput) -> AppResult<AppRuntimeLifecycleRecord> {
        let mut state = self.lock_state()?;
        let now = now_millis();
        state.status = "stopped".to_string();
        state.stopped_at = Some(now);
        state.updated_at = now;
        let record = lifecycle_record(&state, "stop", input.reason, now);
        state.lifecycle.push(record.clone());
        Ok(record)
    }

    pub fn reload(&self, input: AppRuntimeReloadInput) -> AppResult<AppRuntimeLifecycleRecord> {
        let mut state = self.lock_state()?;
        let now = now_millis();
        state.status = "running".to_string();
        state.reload_count = state.reload_count.saturating_add(1);
        if let Some(workspace) = non_empty(input.workspace) {
            state.workspace = Some(workspace);
        }
        if let Some(session_id) = non_empty(input.session_id) {
            state.session_id = Some(session_id);
        }
        if state.started_at.is_none() {
            state.started_at = Some(now);
        }
        state.stopped_at = None;
        state.updated_at = now;
        let record = lifecycle_record(&state, "reload", input.reason, now);
        state.lifecycle.push(record.clone());
        Ok(record)
    }

    pub fn set_workspace(
        &self,
        input: AppRuntimeWorkspaceInput,
    ) -> AppResult<AppRuntimeLifecycleRecord> {
        let workspace = required_non_empty("workspace", input.workspace)?;
        let mut state = self.lock_state()?;
        let now = now_millis();
        state.workspace = Some(workspace);
        state.updated_at = now;
        let record = lifecycle_record(&state, "workspace", input.reason, now);
        state.lifecycle.push(record.clone());
        Ok(record)
    }

    pub fn set_session(
        &self,
        input: AppRuntimeSessionInput,
    ) -> AppResult<AppRuntimeLifecycleRecord> {
        let session_id = required_non_empty("sessionId", input.session_id)?;
        let mut state = self.lock_state()?;
        let now = now_millis();
        state.session_id = Some(session_id);
        state.updated_at = now;
        let record = lifecycle_record(&state, "session", input.reason, now);
        state.lifecycle.push(record.clone());
        Ok(record)
    }

    pub fn snapshot(&self) -> AppRuntimeSnapshot {
        self.state
            .lock()
            .map(|state| AppRuntimeSnapshot {
                schema_version: "app-runtime/v1".to_string(),
                status: state.status.clone(),
                workspace: state.workspace.clone(),
                session_id: state.session_id.clone(),
                mode: state.mode.clone(),
                data_dir: state.data_dir.clone(),
                platform_id: state.platform_id.clone(),
                reload_count: state.reload_count,
                started_at: state.started_at,
                stopped_at: state.stopped_at,
                updated_at: state.updated_at,
                lifecycle: state.lifecycle.clone(),
            })
            .unwrap_or_else(|_| AppRuntimeSnapshot {
                schema_version: "app-runtime/v1".to_string(),
                status: "unavailable".to_string(),
                workspace: None,
                session_id: None,
                mode: String::new(),
                data_dir: String::new(),
                platform_id: current_platform().to_string(),
                reload_count: 0,
                started_at: None,
                stopped_at: None,
                updated_at: now_millis(),
                lifecycle: Vec::new(),
            })
    }

    fn lock_state(&self) -> AppResult<std::sync::MutexGuard<'_, AppRuntimeState>> {
        self.state
            .lock()
            .map_err(|_| AppError::Conflict("app runtime lock poisoned".to_string()))
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

fn lifecycle_record(
    state: &AppRuntimeState,
    action: &str,
    reason: Option<String>,
    timestamp: i64,
) -> AppRuntimeLifecycleRecord {
    AppRuntimeLifecycleRecord {
        action: action.to_string(),
        status: state.status.clone(),
        workspace: state.workspace.clone(),
        session_id: state.session_id.clone(),
        mode: state.mode.clone(),
        data_dir: state.data_dir.clone(),
        platform_id: state.platform_id.clone(),
        reload_count: state.reload_count,
        reason: reason.unwrap_or_default(),
        timestamp,
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn required_non_empty(field: &str, value: String) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field} must not be empty")));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn runtime_should_track_lifecycle_workspace_session_and_reload() {
        let data_dir = tempdir().expect("data dir");
        let runtime = AppRuntime::new(data_dir.path().to_path_buf());

        let start = runtime
            .start(AppRuntimeStartInput {
                workspace: Some("/workspace/aio".to_string()),
                session_id: Some("session-1".to_string()),
                mode: Some("desktop".to_string()),
                reason: Some("test start".to_string()),
            })
            .expect("start");
        assert_eq!(start.action, "start");
        assert_eq!(start.status, "running");

        runtime
            .set_workspace(AppRuntimeWorkspaceInput {
                workspace: "/workspace/next".to_string(),
                reason: None,
            })
            .expect("workspace");
        runtime
            .set_session(AppRuntimeSessionInput {
                session_id: "session-2".to_string(),
                reason: None,
            })
            .expect("session");
        runtime
            .reload(AppRuntimeReloadInput {
                reason: Some("registry hot path".to_string()),
                ..Default::default()
            })
            .expect("reload");

        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.status, "running");
        assert_eq!(snapshot.workspace.as_deref(), Some("/workspace/next"));
        assert_eq!(snapshot.session_id.as_deref(), Some("session-2"));
        assert_eq!(snapshot.reload_count, 1);
        assert_eq!(snapshot.lifecycle.len(), 4);
        assert_eq!(
            snapshot
                .lifecycle
                .last()
                .unwrap()
                .event_input()
                .schema
                .as_deref(),
            Some(RUNTIME_EVENT_SCHEMA)
        );
    }

    #[test]
    fn runtime_should_reject_empty_workspace_and_session() {
        let runtime = AppRuntime::default();

        assert!(matches!(
            runtime.set_workspace(AppRuntimeWorkspaceInput {
                workspace: " ".to_string(),
                reason: None,
            }),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            runtime.set_session(AppRuntimeSessionInput {
                session_id: String::new(),
                reason: None,
            }),
            Err(AppError::BadRequest(_))
        ));
    }
}
