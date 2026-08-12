use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::{
        routes::{AssetHubApiState, asset_hub_router},
        store::{AssetHubStore, build_asset_hub_context_with_db},
    },
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct AssetHubPlugin;

impl NativePluginProvider for AssetHubPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let store = context.shared_db.clone().map(|shared_db| {
            let mut plugin_context = build_asset_hub_context_with_db(shared_db.clone());
            plugin_context.resolve::<AssetHubStore>()
        });
        let state = AssetHubApiState::from_store(context.database_url.clone(), store);
        Ok(NativePluginRuntime {
            router: asset_hub_router(state),
        })
    }
}

#[Singleton(name = "asset-hub")]
pub fn asset_hub_plugin() -> DynAdminPluginProvider {
    Arc::new(AssetHubPlugin)
}


#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = AssetHubPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "asset-hub");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/asset-hub/status")
        );
    }
}
