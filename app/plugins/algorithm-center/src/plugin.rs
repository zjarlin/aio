use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use az_algorithm::di::{create_algorithm_context, resolve_algorithm_catalog};
use rudi::Singleton;

use crate::{
    backend::routes::{AlgorithmCenterApiState, algorithm_center_router},
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct AlgorithmCenterPlugin;

impl NativePluginProvider for AlgorithmCenterPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, _context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let mut algorithm_context = create_algorithm_context();
        let catalog = resolve_algorithm_catalog(&mut algorithm_context)?;
        Ok(NativePluginRuntime {
            router: algorithm_center_router(AlgorithmCenterApiState { catalog }),
        })
    }
}

#[Singleton(name = "algorithm-center")]
pub fn algorithm_center_plugin() -> DynAdminPluginProvider {
    Arc::new(AlgorithmCenterPlugin)
}

#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = AlgorithmCenterPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "algorithm-center");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/algorithm-center/status")
        );
    }
}
