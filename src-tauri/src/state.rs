use std::{fs, path::PathBuf};

use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::{
    capability_broker::CapabilityBroker,
    db,
    error::AppResult,
    event_bus::{EventBus, EventBusPublishInput},
    extension_host::ExtensionHostRuntime,
    permission_core::PermissionCore,
};

#[derive(Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub capability_broker: CapabilityBroker,
    pub event_bus: EventBus,
    pub extension_host: ExtensionHostRuntime,
    pub permission_core: PermissionCore,
    pub pool: SqlitePool,
}

impl AppState {
    pub async fn new(app: &AppHandle) -> AppResult<Self> {
        let data_dir = app.path().app_data_dir()?;
        Self::from_data_dir(data_dir).await
    }

    pub(crate) async fn from_data_dir(data_dir: PathBuf) -> AppResult<Self> {
        fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("aio.sqlite");
        let pool = db::connect(db_path).await?;
        if let Err(error) = db::migrate_and_seed(&pool).await {
            eprintln!("setup migrate_and_seed failed: {error:?}");
            return Err(error);
        }

        let event_bus = EventBus::default();
        event_bus.publish(EventBusPublishInput {
            event_type: "platform.started".to_string(),
            payload: serde_json::json!({
                "dataDir": data_dir.to_string_lossy(),
            }),
            source: "platform.runtime".to_string(),
            target: None,
            parent_trace_id: None,
            permissions: None,
        })?;

        Ok(Self {
            data_dir,
            capability_broker: CapabilityBroker::default(),
            event_bus,
            extension_host: ExtensionHostRuntime::default(),
            permission_core: PermissionCore::default(),
            pool,
        })
    }
}
