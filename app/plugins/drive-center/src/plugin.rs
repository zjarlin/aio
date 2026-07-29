use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::{
        routes::{DriveCenterApiState, drive_center_router},
        store::{DriveCenterStore, build_drive_center_context_with_db},
    },
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct DriveCenterPlugin;

impl NativePluginProvider for DriveCenterPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let store = context.shared_db.clone().map(|shared_db| {
            let mut plugin_context = build_drive_center_context_with_db(shared_db.clone());
            plugin_context.resolve::<DriveCenterStore>()
        });
        let state = DriveCenterApiState::from_store(context.database_url.clone(), store);
        Ok(NativePluginRuntime {
            router: drive_center_router(state),
            startup: None,
        })
    }
}

#[Singleton(name = "drive-center")]
pub fn drive_center_plugin() -> DynAdminPluginProvider {
    Arc::new(DriveCenterPlugin)
}


#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = DriveCenterPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "drive-center");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/drive-center/status")
        );
    }
}
