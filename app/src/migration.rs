//! AIO PostgreSQL 的 SQLx 迁移入口。

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// 在 Toasty 建立业务句柄前执行全部 AIO schema 迁移。
pub async fn run(database_url: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("SQLx 连接 AIO PostgreSQL 失败")?;
    MIGRATOR
        .run(&pool)
        .await
        .context("执行 AIO SQLx 迁移失败")?;
    pool.close().await;
    Ok(())
}
