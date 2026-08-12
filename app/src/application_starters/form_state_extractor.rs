//! Studio AI 表单状态提取器启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture};
use studio::FormStateExtractor;

use crate::application_startup::ApplicationStartup;

/// 从环境配置建立 FormStateExtractor。
pub struct FormStateExtractorStarter;

impl Plugin<ApplicationStartup> for FormStateExtractorStarter {
    fn order(&self) -> i32 {
        40
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            target.set_form_state_extractor(FormStateExtractor::from_env()?);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<FormStateExtractorStarter>())]
pub fn form_state_extractor_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(FormStateExtractorStarter)
}
