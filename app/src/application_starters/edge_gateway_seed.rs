//! 边缘网关内置数据启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture, PluginType};

use super::database::SharedDatabaseStarter;
use crate::application_startup::ApplicationStartup;

/// 初始化边缘网关内置天气令牌和路由。
pub struct EdgeGatewaySeedStarter;

impl Plugin<ApplicationStartup> for EdgeGatewaySeedStarter {
    fn order(&self) -> i32 {
        30
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![PluginType::of::<SharedDatabaseStarter>()]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let shared_db = target.shared_db()?.cloned();
            if let Err(error) = edge_gateway::plugin::seed_builtin_data(shared_db).await {
                eprintln!("edge-gateway Toasty startup degraded: {error:#}");
            }
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<EdgeGatewaySeedStarter>())]
pub fn edge_gateway_seed_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(EdgeGatewaySeedStarter)
}
