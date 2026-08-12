//! Studio HTTP 路由启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture, PluginType};

use super::{
    convention_routes::ConventionRoutesStarter, form_state_extractor::FormStateExtractorStarter,
    patch_agent::ProgramPatchAgentStarter, program_runtime::ProgramRuntimeStarter,
};
use crate::application_startup::ApplicationStartup;

/// 组装 Studio 编辑、发布、运行时记录和 SSE 路由。
pub struct StudioHttpRoutesStarter;

impl Plugin<ApplicationStartup> for StudioHttpRoutesStarter {
    fn order(&self) -> i32 {
        60
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![
            PluginType::of::<ProgramRuntimeStarter>(),
            PluginType::of::<ConventionRoutesStarter>(),
            PluginType::of::<ProgramPatchAgentStarter>(),
            PluginType::of::<FormStateExtractorStarter>(),
        ]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let runtime = target.program_runtime()?;
            let contracts = target.convention_contracts()?;
            let patch_agent = target.take_patch_agent()?;
            let form_state_extractor = target.take_form_state_extractor()?;
            let state = studio::studio_http::StudioState::new(
                runtime,
                patch_agent,
                form_state_extractor,
                contracts,
            );
            target.merge_router(studio::studio_http::router(state));
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<StudioHttpRoutesStarter>())]
pub fn studio_http_routes_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(StudioHttpRoutesStarter)
}
