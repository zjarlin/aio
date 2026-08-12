//! Studio ProgramRuntime 启动器。

use std::sync::Arc;

use anyhow::Context as _;
use az_plugin_core::{DynPlugin, Plugin, PluginFuture, PluginType, RecordStore};
use studio::{
    CompiledArtifactWriter, NativeContractCatalog, program_runtime::ProgramRuntime,
    program_store::ProgramStore,
};

use super::{
    capabilities::CapabilityCatalogStarter, database::SharedDatabaseStarter,
    native_plugins::NativePluginDiscoveryStarter,
};
use crate::{application_startup::ApplicationStartup, config::AppConfig};

/// 恢复数据库程序、同步原生契约并启动 PostgreSQL 监听。
pub struct ProgramRuntimeStarter {
    database_url: Option<String>,
}

impl Plugin<ApplicationStartup> for ProgramRuntimeStarter {
    fn order(&self) -> i32 {
        40
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![
            PluginType::of::<SharedDatabaseStarter>(),
            PluginType::of::<NativePluginDiscoveryStarter>(),
            PluginType::of::<CapabilityCatalogStarter>(),
        ]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let Some(database_url) = self.database_url.as_deref() else {
                target.set_program_runtime(None);
                return Ok(());
            };
            let database = target
                .shared_db()?
                .cloned()
                .context("已配置 PostgreSQL，但共享数据库启动器未建立连接")?;
            let native_contracts = NativeContractCatalog::from_contributions(
                target
                    .native_snapshot()?
                    .plugin_contributions
                    .iter()
                    .map(|record| (record.plugin_id.as_str(), &record.contributions)),
            )?;
            let capabilities = target.take_capabilities()?;
            let record_store =
                RecordStore::from_shared_db(database.shared_handle(), database.pg_pool());
            let store = ProgramStore::from_pool(database.pg_pool());
            let runtime = ProgramRuntime::new(
                store,
                record_store,
                capabilities,
                CompiledArtifactWriter::workspace_target(),
            );

            let _native_report = runtime
                .store()
                .reconcile_native_contracts(&native_contracts)
                .await
                .context("同步插件 API 元数据到 Studio 失败")?;
            runtime.restore_active_image().await?;
            if let Err(error) = runtime.publish_draft_if_changed("migration").await {
                if runtime.active_image().await.is_none() {
                    return Err(error).context("发布原生接口元数据 Revision 失败");
                }
                eprintln!("发布最新 Studio Draft 失败，继续使用活动 Revision: {error:#}");
            }
            runtime.spawn_postgres_listener(database_url).await?;
            target.set_program_runtime(Some(runtime));
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<ProgramRuntimeStarter>())]
pub fn program_runtime_starter(config: AppConfig) -> DynPlugin<ApplicationStartup> {
    Arc::new(ProgramRuntimeStarter {
        database_url: config.database_url,
    })
}
