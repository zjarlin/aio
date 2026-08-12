//! 原生 API 统一错误与超时中间件启动器。

use std::sync::Arc;

use az_plugin_core::{
    DynPlugin, Plugin, PluginFuture, PluginType, http::with_global_api_error_layer,
};

use super::native_plugins::NativePluginDiscoveryStarter;
use crate::application_startup::ApplicationStartup;

/// 给全部原生插件路由安装统一错误与超时层。
pub struct ApiErrorMiddlewareStarter;

impl Plugin<ApplicationStartup> for ApiErrorMiddlewareStarter {
    fn order(&self) -> i32 {
        60
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![PluginType::of::<NativePluginDiscoveryStarter>()]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let native_router = target.native_snapshot()?.native_router.clone();
            let router = with_global_api_error_layer(native_router).with_state(());
            target.merge_router(router);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<ApiErrorMiddlewareStarter>())]
pub fn api_error_middleware_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(ApiErrorMiddlewareStarter)
}
