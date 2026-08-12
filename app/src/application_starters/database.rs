//! PostgreSQL 迁移与共享连接启动器。

use std::sync::Arc;

use az_plugin_core::{
    DynPlugin, Plugin, PluginFuture, PluginType, database::install_shared_db_singleton,
};

use crate::{application_startup::ApplicationStartup, config::AppConfig, migration};

/// 执行 AIO SQLx 迁移。
pub struct DatabaseMigrationStarter {
    database_url: Option<String>,
    migrations_enabled: bool,
}

impl Plugin<ApplicationStartup> for DatabaseMigrationStarter {
    fn order(&self) -> i32 {
        10
    }

    fn install<'a>(&'a self, _target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            if !self.migrations_enabled {
                return Ok(());
            }
            let Some(database_url) = self.database_url.as_deref() else {
                return Ok(());
            };
            migration::run(database_url).await
        })
    }
}

/// 汇总 Toasty 模型并注册共享 PostgreSQL 连接。
pub struct SharedDatabaseStarter {
    database_url: Option<String>,
}

impl Plugin<ApplicationStartup> for SharedDatabaseStarter {
    fn order(&self) -> i32 {
        20
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![PluginType::of::<DatabaseMigrationStarter>()]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let models = target.take_models()?.into_model_set();
            let shared_db =
                install_shared_db_singleton(target.di_mut(), self.database_url.as_deref(), models)
                    .await?;
            target.set_shared_db(shared_db);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<DatabaseMigrationStarter>())]
pub fn database_migration_starter(config: AppConfig) -> DynPlugin<ApplicationStartup> {
    Arc::new(DatabaseMigrationStarter {
        database_url: config.database_url,
        migrations_enabled: config.database_migrations_enabled,
    })
}

#[rudi::Singleton(name = std::any::type_name::<SharedDatabaseStarter>())]
pub fn shared_database_starter(config: AppConfig) -> DynPlugin<ApplicationStartup> {
    Arc::new(SharedDatabaseStarter {
        database_url: config.database_url,
    })
}

#[cfg(test)]
mod tests {
    use az_plugin_core::Plugin;

    use super::*;

    #[tokio::test]
    async fn disabled_migration_does_not_connect_database() -> anyhow::Result<()> {
        let starter = DatabaseMigrationStarter {
            database_url: Some("postgresql://127.0.0.1/unused".to_owned()),
            migrations_enabled: false,
        };
        let mut startup = ApplicationStartup::new(rudi::Context::create(Vec::new()));

        starter.install(&mut startup).await
    }
}
