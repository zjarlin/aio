//! Studio 程序补丁 Agent 启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture};
use studio::ProgramPatchAgent;

use crate::application_startup::ApplicationStartup;

/// 从环境配置建立 ProgramPatchAgent。
pub struct ProgramPatchAgentStarter;

impl Plugin<ApplicationStartup> for ProgramPatchAgentStarter {
    fn order(&self) -> i32 {
        40
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            target.set_patch_agent(ProgramPatchAgent::from_env()?);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<ProgramPatchAgentStarter>())]
pub fn program_patch_agent_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(ProgramPatchAgentStarter)
}
