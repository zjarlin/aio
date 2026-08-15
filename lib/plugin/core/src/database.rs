//! 共享数据库底座。
//!
//! 为所有 插件提供统一的 `toasty::Db` 连接管理与工具函数，
//! 消除每个插件各自重复的 `Arc<Mutex<Db>>`、URL 校验、UUID 和时间戳逻辑。

use std::{any::Any, sync::Arc};

use anyhow::{Context, anyhow};
use toasty::ModelSet;
use tokio::sync::Mutex;

/// 由 Dill 按具体 Rust 类型聚合的 Toasty 模型贡献者。
pub trait ToastyModelProvider: Any + Send + Sync {
    fn models(&self) -> ModelSet;

    /// 可选的人类可读说明，只用于日志和诊断。
    fn comment(&self) -> &'static str {
        ""
    }
}

/// 共享数据库柄。
///
/// 持有 `toasty::Db` 的共享引用，所有插件复用同一连接池。
/// 平台启动时统一注册所有 Toasty 模型，插件 store 只持有该共享句柄。
#[derive(Clone)]
pub struct Db {
    db: Arc<Mutex<toasty::Db>>,
    pool: sqlx::PgPool,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Db").finish_non_exhaustive()
    }
}

impl Db {
    /// 从已配置好的 `toasty::Db` 创建共享包装。
    ///
    /// `toasty::Db` 的构造由平台启动入口统一完成，
    /// `Db` 只负责提供共享的 `Arc<Mutex<>>` 访问。
    pub fn new(db: toasty::Db, pool: sqlx::PgPool) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            pool,
        }
    }

    /// 使用已完成 SQLx 迁移的 PostgreSQL 和模型集合建立连接。
    pub async fn connect_with_models(database_url: &str, models: ModelSet) -> anyhow::Result<Self> {
        let database_url = verify_database_url(database_url)?;
        let db = toasty::Db::builder()
            .models(models)
            .connect(database_url)
            .await
            .context("连接 AIO PostgreSQL 失败")?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("连接 AIO SQLx PostgreSQL 失败")?;
        Ok(Self::new(db, pool))
    }

    /// 获取内部 `toasty::Db` 的独占锁守卫。
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, toasty::Db> {
        self.db.lock().await
    }

    /// 为已经持有执行器包装的 store 返回共享 Toasty 句柄。
    pub fn shared_handle(&self) -> Arc<Mutex<toasty::Db>> {
        Arc::clone(&self.db)
    }

    pub fn pg_pool(&self) -> sqlx::PgPool {
        self.pool.clone()
    }
}

/// 使用全部模型建立共享 PostgreSQL 连接。
pub async fn connect_shared_db(
    database_url: Option<&str>,
    models: ModelSet,
) -> anyhow::Result<Option<Db>> {
    let Some(database_url) = database_url.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let db = Db::connect_with_models(database_url, models).await?;
    Ok(Some(db))
}

/// 把 Dill 注入的全部 Toasty 模型贡献合并为一个模型集合。
pub fn collect_toasty_models(providers: &[Arc<dyn ToastyModelProvider>]) -> ModelSet {
    let mut models = crate::records::engine_models();
    for provider in providers {
        for model in provider.models() {
            models.add(model);
        }
    }
    models
}

/// 校验并规范化数据库连接串。
pub fn verify_database_url(value: &str) -> anyhow::Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("数据库连接串未配置"));
    }
    if !value.starts_with("postgresql://") && !value.starts_with("postgres://") {
        return Err(anyhow!(
            "正式持久化只接受 PostgreSQL 连接串，请改用 postgresql://...，当前值: {value}"
        ));
    }
    Ok(value)
}

/// 生成 UUID v4 字符串。
pub fn new_uuid_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 生成秒级 Unix 时间戳字符串。
pub fn timestamp_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// 生成毫秒级 Unix 时间戳。
pub fn timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_model_collection_contains_engine_models() {
        let engine_model_ids = crate::records::engine_models()
            .iter()
            .map(|model| model.id())
            .collect::<Vec<_>>();
        let models = collect_toasty_models(&[]);

        assert!(
            engine_model_ids
                .into_iter()
                .all(|model_id| models.contains(model_id))
        );
    }

    #[test]
    fn rejects_empty_url() {
        assert!(verify_database_url("").is_err());
        assert!(verify_database_url("   ").is_err());
    }

    #[test]
    fn timestamp_ms_uses_millisecond_precision() {
        // 管理 API 统一返回毫秒时间点，避免前端再猜测时间单位。
        assert!(timestamp_ms() > 1_000_000_000_000);
    }

    #[test]
    fn accepts_valid_url() {
        assert_eq!(
            verify_database_url("postgresql://localhost/test").unwrap(),
            "postgresql://localhost/test"
        );
        assert_eq!(
            verify_database_url("postgres://localhost/test").unwrap(),
            "postgres://localhost/test"
        );
    }

    #[test]
    fn rejects_sqlite_url() {
        let error = verify_database_url("sqlite:az-aio.db?mode=rwc").unwrap_err();
        // 防止正式 admin 数据误落到本地 SQLite，统一走 Toasty PG。
        assert!(error.to_string().contains("PostgreSQL"));

        let error = verify_database_url("sqlite:sync.db?mode=rwc").unwrap_err();
        // sync.db 也不再作为正式持久化落点。
        assert!(error.to_string().contains("PostgreSQL"));
    }

    #[test]
    fn uuid_has_expected_length() {
        assert_eq!(new_uuid_id().len(), 36);
    }

    #[test]
    fn timestamp_is_non_empty() {
        assert!(!timestamp_secs().is_empty());
    }
}
