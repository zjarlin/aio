//! AIO PostgreSQL 的 SQLx 迁移入口。

use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

const DATABASE_CONNECT_ATTEMPTS: u32 = 5;

/// 在 Toasty 建立业务句柄前执行全部 AIO schema 迁移。
pub async fn run(database_url: &str) -> Result<()> {
    let pool = connect_pool(database_url).await?;
    MIGRATOR
        .run(&pool)
        .await
        .context("执行 AIO SQLx 迁移失败")?;
    pool.close().await;
    Ok(())
}

/// 为短暂的局域网或数据库网络切换保留有限连接重试。
async fn connect_pool(database_url: &str) -> Result<sqlx::PgPool> {
    let mut attempt = 1;

    loop {
        match PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(_) if attempt < DATABASE_CONNECT_ATTEMPTS => {
                tokio::time::sleep(connection_retry_delay(attempt)).await;
                attempt += 1;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("SQLx 连续 {DATABASE_CONNECT_ATTEMPTS} 次连接 AIO PostgreSQL 失败")
                });
            }
        }
    }
}

fn connection_retry_delay(attempt: u32) -> Duration {
    Duration::from_secs(1_u64 << attempt.saturating_sub(1).min(2))
}

#[cfg(test)]
mod tests {
    use super::connection_retry_delay;
    use std::time::Duration;

    #[test]
    fn connection_retry_delay_is_bounded() {
        assert_eq!(connection_retry_delay(1), Duration::from_secs(1));
        assert_eq!(connection_retry_delay(2), Duration::from_secs(2));
        assert_eq!(connection_retry_delay(3), Duration::from_secs(4));
        assert_eq!(connection_retry_delay(4), Duration::from_secs(4));
    }
}
