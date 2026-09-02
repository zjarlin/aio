//! Studio ProgramRuntime 启动器。

use std::sync::{Arc, OnceLock};

use anyhow::Context as _;
use az_plugin_core::{Plugin, PluginFuture, RecordStore};
use dill::CatalogBuilder;
use studio::{
    CompiledArtifactWriter,
    capability::{CapabilityCatalog, CapabilityProvider},
    program_runtime::ProgramRuntime,
    program_store::ProgramStore,
};

use super::database::SharedDatabase;
use crate::{application_startup::ApplicationStartup, config::AppConfig};

/// Dill 管理的 Studio ProgramRuntime 生命周期。
#[derive(Default)]
pub(super) struct SharedProgramRuntime {
    state: OnceLock<Option<Arc<ProgramRuntime>>>,
}

impl SharedProgramRuntime {
    fn initialize(&self, runtime: Option<ProgramRuntime>) -> anyhow::Result<()> {
        let runtime = runtime.map(Arc::new);
        anyhow::ensure!(
            self.state.set(runtime).is_ok(),
            "Studio ProgramRuntime 被重复初始化"
        );
        Ok(())
    }

    pub(super) fn current(&self) -> anyhow::Result<Option<Arc<ProgramRuntime>>> {
        self.state
            .get()
            .context("Studio ProgramRuntime 启动器尚未执行")
            .cloned()
    }
}

/// 恢复数据库程序并启动 PostgreSQL 监听。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct ProgramRuntimeStarter {
    config: Arc<AppConfig>,
    shared_database: Arc<SharedDatabase>,
    shared_runtime: Arc<SharedProgramRuntime>,
    capability_providers: Vec<Arc<dyn CapabilityProvider>>,
    compiled_artifacts: Arc<CompiledArtifactWriter>,
}

impl Plugin<ApplicationStartup> for ProgramRuntimeStarter {
    fn build<'a>(&'a self, _target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let Some(database) = self.shared_database.current()? else {
                self.shared_runtime.initialize(None)?;
                return Ok(());
            };
            let database_url = self
                .config
                .database_url
                .as_deref()
                .context("共享数据库已连接但 PostgreSQL 配置缺失")?;
            let capabilities = CapabilityCatalog::new(self.capability_providers.clone())?;
            let record_store =
                RecordStore::from_shared_db(database.shared_handle(), database.pg_pool());
            let store = ProgramStore::from_pool(database.pg_pool());
            let runtime = ProgramRuntime::new(
                store,
                record_store,
                capabilities,
                self.compiled_artifacts.as_ref().clone(),
            );

            runtime.restore_active_image().await?;
            if let Err(error) = runtime.publish_draft_if_changed("migration").await {
                if runtime.image().is_none() {
                    return Err(error).context("发布最新 Studio Draft 失败");
                }
                eprintln!("发布最新 Studio Draft 失败，继续使用活动 Revision: {error:#}");
            }
            runtime.spawn_postgres_listener(database_url);
            self.shared_runtime.initialize(Some(runtime))
        })
    }
}

pub(super) fn register(builder: &mut CatalogBuilder) {
    builder
        .add_value(SharedProgramRuntime::default())
        .add_value(CompiledArtifactWriter::workspace_target())
        .add::<ProgramRuntimeStarter>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_runtime_is_an_initialized_dill_resource() -> anyhow::Result<()> {
        let runtime = SharedProgramRuntime::default();
        runtime.initialize(None)?;

        assert!(runtime.current()?.is_none());
        Ok(())
    }

    #[test]
    fn runtime_resource_rejects_duplicate_initialization() -> anyhow::Result<()> {
        let runtime = SharedProgramRuntime::default();
        runtime.initialize(None)?;

        assert!(runtime.initialize(None).is_err());
        Ok(())
    }
}
