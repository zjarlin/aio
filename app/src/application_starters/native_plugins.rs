//! 原生业务插件发现启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture, PluginType, plugin::NativePluginContext};

use super::database::SharedDatabaseStarter;
use crate::{application_startup::ApplicationStartup, config::AppConfig, plugin_host};

/// 从 Rudi 收集并初始化全部原生业务插件。
pub struct NativePluginDiscoveryStarter {
    api_base_url: String,
    database_url: Option<String>,
    config_dir: std::path::PathBuf,
    data_dir: std::path::PathBuf,
}

impl Plugin<ApplicationStartup> for NativePluginDiscoveryStarter {
    fn order(&self) -> i32 {
        30
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![PluginType::of::<SharedDatabaseStarter>()]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let shared_db = target.shared_db()?.cloned();
            let context = NativePluginContext {
                api_base_url: self.api_base_url.clone(),
                database_url: self.database_url.clone(),
                shared_db,
                config_dir: self.config_dir.clone(),
                data_dir: self.data_dir.clone(),
            };
            let snapshot = plugin_host::load_native_snapshot(context, target.di_mut());
            target.set_native_snapshot(snapshot);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<NativePluginDiscoveryStarter>())]
pub fn native_plugin_discovery_starter(config: AppConfig) -> DynPlugin<ApplicationStartup> {
    Arc::new(NativePluginDiscoveryStarter {
        api_base_url: config.api_base_url(),
        database_url: config.database_url,
        config_dir: config.config_dir,
        data_dir: config.data_dir,
    })
}
