use std::{fs, path::PathBuf};

use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::{db, error::AppResult};

#[derive(Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub pool: SqlitePool,
}

impl AppState {
    pub async fn new(app: &AppHandle) -> AppResult<Self> {
        let data_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("aio.sqlite");
        let pool = db::connect(db_path).await?;
        if let Err(error) = db::migrate_and_seed(&pool).await {
            eprintln!("setup migrate_and_seed failed: {error:?}");
            return Err(error);
        }

        Ok(Self { data_dir, pool })
    }
}
