use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::routes::{LinuxApiState, linux_router},
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct LinuxPlugin;

impl NativePluginProvider for LinuxPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let install_base_url = if context.api_base_url.trim().is_empty() {
            "http://<aio-host>:<port>".to_string()
        } else {
            context.api_base_url.clone()
        };
        let state = LinuxApiState::new(install_base_url);
        Ok(NativePluginRuntime {
            router: linux_router(state),
            startup: None,
        })
    }
}

#[Singleton(name = "linux")]
pub fn linux_plugin() -> DynAdminPluginProvider {
    Arc::new(LinuxPlugin)
}

#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_client_first_linux_contract() -> anyhow::Result<()> {
        let plugin = LinuxPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions()?;

        assert_eq!(descriptor.id, "linux");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(contributions.backend_apis.iter().any(|api| {
            api.method == "GET" && api.path == "/api/linux/bootstrap-script"
        }));
        Ok(())
    }
}
