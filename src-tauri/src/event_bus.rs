use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
};

#[allow(dead_code)]
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventBusPublishInput {
    pub event_type: String,
    pub payload: Value,
    pub source: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    #[serde(default)]
    pub permissions: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct EventBusRequestInput {
    pub event_type: String,
    pub payload: Value,
    pub source: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    #[serde(default)]
    pub permissions: Option<Value>,
    #[serde(default = "default_request_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventBusSnapshotRequest {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformEventRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub kind: String,
    pub source: String,
    #[serde(default)]
    pub target: Option<String>,
    pub payload: Value,
    pub trace_id: String,
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    pub timestamp: i64,
    #[serde(default)]
    pub permissions: Option<Value>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EventRequestContext {
    pub cancelled: watch::Receiver<bool>,
    pub request_id: String,
    pub trace_id: String,
}

type EventHandlerFuture = Pin<Box<dyn Future<Output = AppResult<Value>> + Send>>;
type EventHandler =
    Arc<dyn Fn(PlatformEventRecord, EventRequestContext) -> EventHandlerFuture + Send + Sync>;

#[derive(Default)]
#[allow(dead_code)]
struct EventBusState {
    history: Vec<PlatformEventRecord>,
    handlers: HashMap<String, EventHandler>,
    pending: HashMap<String, watch::Sender<bool>>,
}

#[derive(Clone)]
pub struct EventBus {
    state: Arc<Mutex<EventBusState>>,
    topic: broadcast::Sender<PlatformEventRecord>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (topic, _) = broadcast::channel(256);
        Self {
            state: Arc::new(Mutex::new(EventBusState::default())),
            topic,
        }
    }

    pub fn publish(&self, input: EventBusPublishInput) -> AppResult<PlatformEventRecord> {
        let event_type = normalize_event_type(&input.event_type)?;
        let source = normalize_event_source(&input.source)?;
        let record = PlatformEventRecord {
            id: Uuid::new_v4().to_string(),
            event_type,
            kind: "publish".to_string(),
            source,
            target: input.target,
            payload: input.payload,
            trace_id: Uuid::new_v4().to_string(),
            parent_trace_id: input.parent_trace_id,
            correlation_id: None,
            timestamp: now_millis(),
            permissions: input.permissions,
        };
        self.record(record.clone())?;
        Ok(record)
    }

    #[allow(dead_code)]
    pub fn register_handler<F, Fut>(&self, event_type: impl Into<String>, handler: F)
    where
        F: Fn(PlatformEventRecord, EventRequestContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AppResult<Value>> + Send + 'static,
    {
        let event_type = event_type.into();
        let handler = Arc::new(move |event, context| {
            let future = handler(event, context);
            Box::pin(future) as EventHandlerFuture
        });
        if let Ok(mut state) = self.state.lock() {
            state.handlers.insert(event_type, handler);
        }
    }

    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<PlatformEventRecord> {
        self.topic.subscribe()
    }

    pub fn snapshot(&self, filter: Option<&str>, limit: Option<usize>) -> Vec<PlatformEventRecord> {
        let mut records = self
            .state
            .lock()
            .map(|state| {
                state
                    .history
                    .iter()
                    .filter(|record| {
                        filter
                            .map(|value| record.event_type == value)
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let Some(limit) = limit {
            let keep = limit.min(records.len());
            records.drain(0..records.len() - keep);
        }
        records
    }

    #[allow(dead_code)]
    pub async fn request(&self, input: EventBusRequestInput) -> AppResult<Value> {
        let event_type = normalize_event_type(&input.event_type)?;
        let source = normalize_event_source(&input.source)?;
        let request_id = input
            .request_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let trace_id = Uuid::new_v4().to_string();
        let timeout = Duration::from_millis(input.timeout_ms.max(1));
        let request_record = PlatformEventRecord {
            id: request_id.clone(),
            event_type: event_type.clone(),
            kind: "request".to_string(),
            source: source.clone(),
            target: input.target.clone(),
            payload: input.payload.clone(),
            trace_id: trace_id.clone(),
            parent_trace_id: input.parent_trace_id.clone(),
            correlation_id: Some(request_id.clone()),
            timestamp: now_millis(),
            permissions: input.permissions.clone(),
        };

        let handler = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AppError::Conflict("event bus lock poisoned".to_string()))?;
            let Some(handler) = state.handlers.get(&event_type).cloned() else {
                return Err(AppError::NotFound);
            };
            let (cancel_tx, cancel_rx) = watch::channel(false);
            if state
                .pending
                .insert(request_id.clone(), cancel_tx)
                .is_some()
            {
                return Err(AppError::Conflict(format!(
                    "event request {} already exists",
                    request_id
                )));
            }
            drop(state);
            (handler, cancel_rx)
        };

        self.record(request_record.clone())?;
        let (handler, mut cancel_rx) = handler;
        let context = EventRequestContext {
            cancelled: cancel_rx.clone(),
            request_id: request_id.clone(),
            trace_id: trace_id.clone(),
        };

        let result = tokio::time::timeout(timeout, async {
            tokio::select! {
                _ = cancel_rx.changed() => Err(AppError::Conflict(format!("event request {} was cancelled", request_id))),
                reply = handler(request_record.clone(), context) => reply,
            }
        })
        .await;

        self.remove_pending(&request_id)?;

        match result {
            Ok(Ok(value)) => {
                let reply_record = PlatformEventRecord {
                    id: Uuid::new_v4().to_string(),
                    event_type,
                    kind: "reply".to_string(),
                    source: input
                        .target
                        .clone()
                        .unwrap_or_else(|| "event-bus".to_string()),
                    target: Some(source),
                    payload: value.clone(),
                    trace_id,
                    parent_trace_id: input.parent_trace_id,
                    correlation_id: Some(request_id),
                    timestamp: now_millis(),
                    permissions: input.permissions,
                };
                self.record(reply_record)?;
                Ok(value)
            }
            Ok(Err(error)) => {
                let error_record = PlatformEventRecord {
                    id: Uuid::new_v4().to_string(),
                    event_type,
                    kind: "error".to_string(),
                    source: input.target.unwrap_or_else(|| "event-bus".to_string()),
                    target: Some(source),
                    payload: json!({ "error": error.to_string() }),
                    trace_id,
                    parent_trace_id: input.parent_trace_id,
                    correlation_id: Some(request_id),
                    timestamp: now_millis(),
                    permissions: input.permissions,
                };
                self.record(error_record)?;
                Err(error)
            }
            Err(_) => Err(AppError::Conflict(format!(
                "event request {} timed out after {:?}",
                request_id, timeout
            ))),
        }
    }

    #[allow(dead_code)]
    pub fn cancel_request(&self, request_id: &str) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|mut state| state.pending.remove(request_id))
            .map(|sender| sender.send(true).is_ok())
            .unwrap_or(false)
    }

    fn record(&self, record: PlatformEventRecord) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Conflict("event bus lock poisoned".to_string()))?;
        state.history.push(record.clone());
        let _ = self.topic.send(record);
        Ok(())
    }

    #[allow(dead_code)]
    fn remove_pending(&self, request_id: &str) -> AppResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Conflict("event bus lock poisoned".to_string()))?;
        state.pending.remove(request_id);
        Ok(())
    }
}

fn normalize_event_type(event_type: &str) -> AppResult<String> {
    let event_type = event_type.trim();
    if event_type.is_empty() {
        return Err(AppError::BadRequest(
            "event.type must not be empty".to_string(),
        ));
    }
    Ok(event_type.to_string())
}

fn normalize_event_source(source: &str) -> AppResult<String> {
    let source = source.trim();
    if source.is_empty() {
        return Err(AppError::BadRequest(
            "event.source must not be empty".to_string(),
        ));
    }
    Ok(source.to_string())
}

#[allow(dead_code)]
fn default_request_timeout_ms() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_should_record_events_and_notify_subscribers() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();

        let record = bus
            .publish(EventBusPublishInput {
                event_type: "workspace.opened".to_string(),
                payload: json!({ "workspace": "/tmp/workspace" }),
                source: "platform.runtime".to_string(),
                target: None,
                parent_trace_id: None,
                permissions: None,
            })
            .expect("publish");

        assert_eq!(record.kind, "publish");
        assert_eq!(bus.snapshot(None, None).len(), 1);

        let received =
            tauri::async_runtime::block_on(async move { receiver.recv().await.expect("event") });
        assert_eq!(received.event_type, "workspace.opened");
    }

    #[test]
    fn request_should_reply_and_record_events() {
        let bus = EventBus::new();
        bus.register_handler("settings.get", |_event, _context| async move {
            Ok(json!({ "theme": "day" }))
        });

        let reply = tauri::async_runtime::block_on(async {
            bus.request(EventBusRequestInput {
                event_type: "settings.get".to_string(),
                payload: json!({ "key": "theme" }),
                source: "plugin.test".to_string(),
                request_id: None,
                target: Some("platform.runtime".to_string()),
                parent_trace_id: None,
                permissions: None,
                timeout_ms: 500,
            })
            .await
        })
        .expect("request");

        assert_eq!(reply["theme"], "day");
        assert_eq!(bus.snapshot(Some("settings.get"), None).len(), 2);
    }

    #[test]
    fn request_should_timeout_when_handler_is_slow() {
        let bus = EventBus::new();
        bus.register_handler("slow.reply", |_event, _context| async move {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Ok(json!({ "ok": true }))
        });

        let result = tauri::async_runtime::block_on(async {
            bus.request(EventBusRequestInput {
                event_type: "slow.reply".to_string(),
                payload: json!({}),
                source: "plugin.test".to_string(),
                request_id: None,
                target: None,
                parent_trace_id: None,
                permissions: None,
                timeout_ms: 1,
            })
            .await
        });

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[test]
    fn request_should_support_external_cancellation() {
        let bus = EventBus::new();
        bus.register_handler("slow.cancel", |_event, _context| async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(json!({ "ok": true }))
        });

        let result = tauri::async_runtime::block_on(async {
            let request_bus = bus.clone();
            let request = tauri::async_runtime::spawn(async move {
                request_bus
                    .request(EventBusRequestInput {
                        event_type: "slow.cancel".to_string(),
                        payload: json!({}),
                        source: "plugin.test".to_string(),
                        request_id: Some("cancel-me".to_string()),
                        target: None,
                        parent_trace_id: None,
                        permissions: None,
                        timeout_ms: 500,
                    })
                    .await
            });

            tokio::time::sleep(Duration::from_millis(1)).await;
            assert!(bus.cancel_request("cancel-me"));
            request.await.expect("request task")
        });

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }
}
