//! Studio 约定接口 Provider 索引启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture};
use studio::ConventionEndpointIndex;

use crate::application_startup::ApplicationStartup;

/// 从 Rudi 聚合全部约定接口 Provider。
pub struct ConventionEndpointIndexStarter;

impl Plugin<ApplicationStartup> for ConventionEndpointIndexStarter {
    fn order(&self) -> i32 {
        30
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let endpoints = ConventionEndpointIndex::from_context(target.di_mut())?;
            target.set_convention_endpoints(endpoints);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<ConventionEndpointIndexStarter>())]
pub fn convention_endpoint_index_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(ConventionEndpointIndexStarter)
}
