//! PostgreSQL 迁移与共享连接启动器。

use std::sync::{Arc, OnceLock};

use anyhow::{Context as _, Result, ensure};
use az_plugin_core::{
    Db, Plugin, PluginFuture, ToastyModelProvider,
    database::{collect_toasty_models, connect_shared_db},
};
use dill::CatalogBuilder;

use crate::{application_startup::ApplicationStartup, config::AppConfig, migration};

#[derive(Clone, Debug)]
enum DatabaseState {
    Disabled,
    Ready(Db),
}

/// Dill 管理的共享数据库生命周期。
#[derive(Debug, Default)]
pub(super) struct SharedDatabase {
    state: OnceLock<DatabaseState>,
}

impl SharedDatabase {
    fn initialize(&self, database: Option<Db>) -> Result<()> {
        let state = database.map_or(DatabaseState::Disabled, DatabaseState::Ready);
        ensure!(self.state.set(state).is_ok(), "共享数据库被重复初始化");
        Ok(())
    }

    pub(super) fn current(&self) -> Result<Option<Db>> {
        let state = self.state.get().context("共享数据库启动器尚未执行")?;
        match state {
            DatabaseState::Disabled => Ok(None),
            DatabaseState::Ready(database) => Ok(Some(database.clone())),
        }
    }
}

/// 执行 AIO SQLx 迁移。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct DatabaseMigrationStarter {
    config: Arc<AppConfig>,
}

impl Plugin<ApplicationStartup> for DatabaseMigrationStarter {
    fn build<'a>(&'a self, _target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            if !self.config.database_migrations_enabled {
                return Ok(());
            }
            let Some(database_url) = self.config.database_url.as_deref() else {
                return Ok(());
            };
            migration::run(database_url).await
        })
    }
}

/// 汇总 Toasty 模型并注册共享 PostgreSQL 连接。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct SharedDatabaseStarter {
    config: Arc<AppConfig>,
    shared_database: Arc<SharedDatabase>,
    model_providers: Vec<Arc<dyn ToastyModelProvider>>,
}

impl Plugin<ApplicationStartup> for SharedDatabaseStarter {
    fn build<'a>(&'a self, _target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let models = collect_toasty_models(&self.model_providers);
            let database = connect_shared_db(self.config.database_url.as_deref(), models).await?;
            self.shared_database.initialize(database)
        })
    }
}

pub(super) fn register(builder: &mut CatalogBuilder) {
    builder
        .add_value(SharedDatabase::default())
        .add::<DatabaseMigrationStarter>()
        .add::<SharedDatabaseStarter>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_migration_does_not_connect_database() -> anyhow::Result<()> {
        let config = AppConfig {
            port: 0,
            database_url: Some("postgresql://127.0.0.1/unused".to_owned()),
            database_migrations_enabled: false,
            web_dist_dir: None,
        };
        let starter = DatabaseMigrationStarter::new(Arc::new(config));
        let mut startup = ApplicationStartup::default();

        starter.build(&mut startup).await
    }

    #[test]
    fn disabled_database_is_an_initialized_dill_resource() -> anyhow::Result<()> {
        let database = SharedDatabase::default();
        database.initialize(None)?;

        assert!(database.current()?.is_none());
        Ok(())
    }
}
