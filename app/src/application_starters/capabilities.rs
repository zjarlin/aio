//! Studio Capability 聚合启动器。

use std::sync::Arc;

use az_plugin_core::{DynPlugin, Plugin, PluginFuture};
use studio::capability::{CapabilityCatalog, DynCapabilityProvider};

use crate::application_startup::ApplicationStartup;

/// 从 Rudi 聚合全部类型化 Capability Provider。
pub struct CapabilityCatalogStarter;

impl Plugin<ApplicationStartup> for CapabilityCatalogStarter {
    fn order(&self) -> i32 {
        30
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let providers = target.di_mut().resolve_by_type::<DynCapabilityProvider>();
            let catalog = CapabilityCatalog::new(providers)?;
            target.set_capabilities(catalog);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<CapabilityCatalogStarter>())]
pub fn capability_catalog_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(CapabilityCatalogStarter)
}
