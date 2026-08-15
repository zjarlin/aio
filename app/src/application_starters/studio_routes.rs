//! Studio HTTP 路由启动器。

use std::sync::Arc;

use az_plugin_core::{Plugin, PluginFuture};
use studio::{BusinessModuleManager, FormStateExtractor, ProgramPatchAgent};

use super::program_runtime::SharedProgramRuntime;
use crate::application_startup::ApplicationStartup;

/// 组装 Studio 编辑、发布、运行时记录和 SSE 路由。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct StudioHttpRoutesStarter {
    shared_runtime: Arc<SharedProgramRuntime>,
    business_modules: Arc<BusinessModuleManager>,
    patch_agent: Arc<ProgramPatchAgent>,
    form_state_extractor: Arc<FormStateExtractor>,
}

impl Plugin<ApplicationStartup> for StudioHttpRoutesStarter {
    fn build<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let state = studio::studio_http::StudioState::new(
                self.shared_runtime.current()?,
                Arc::clone(&self.patch_agent),
                Arc::clone(&self.form_state_extractor),
                Arc::clone(&self.business_modules),
            );
            target.merge_router(studio::studio_http::router(state));
            Ok(())
        })
    }
}
