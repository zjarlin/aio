use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::{
        routes::{ConfigCenterApiState, config_center_router},
        store::{ConfigCenterStore, build_config_center_context_with_db},
    },
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct ConfigCenterPlugin;

impl NativePluginProvider for ConfigCenterPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let store = context.shared_db.clone().map(|shared_db| {
            let mut plugin_context = build_config_center_context_with_db(shared_db.clone());
            plugin_context.resolve::<ConfigCenterStore>()
        });
        let state = ConfigCenterApiState::from_store(context.database_url.clone(), store);
        Ok(NativePluginRuntime {
            router: config_center_router(state),
            startup: None,
        })
    }
}

#[Singleton(name = "config-center")]
pub fn config_center_plugin() -> DynAdminPluginProvider {
    Arc::new(ConfigCenterPlugin)
}


#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = ConfigCenterPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "config-center");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/config-center/status")
        );
    }
}
