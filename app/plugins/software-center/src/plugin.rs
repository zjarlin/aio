use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::{
        routes::{SoftwareCenterApiState, software_center_router},
        store::{SoftwareCenterStore, build_software_center_context_with_db},
    },
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct SoftwareCenterPlugin;

impl NativePluginProvider for SoftwareCenterPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let store = context.shared_db.clone().map(|shared_db| {
            let mut plugin_context = build_software_center_context_with_db(shared_db.clone());
            plugin_context.resolve::<SoftwareCenterStore>()
        });
        let state = SoftwareCenterApiState::from_store(context.database_url.clone(), store);
        Ok(NativePluginRuntime {
            router: software_center_router(state),
        })
    }
}

#[Singleton(name = "software-center")]
pub fn software_center_plugin() -> DynAdminPluginProvider {
    Arc::new(SoftwareCenterPlugin)
}


#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = SoftwareCenterPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "software-center");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/software-center/status")
        );
    }
}
