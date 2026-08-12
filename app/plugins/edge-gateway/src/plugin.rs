use std::sync::Arc;

use az_plugin_core::plugin::{
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, PluginDescriptor,
};
use rudi::Singleton;

use crate::{
    backend::{
        routes::{EdgeGatewayApiState, edge_gateway_router},
        store::{EdgeGatewayStore, build_edge_gateway_context_with_db},
    },
    descriptor::{contributions, descriptor},
};

#[derive(Default)]
pub struct EdgeGatewayPlugin;

impl NativePluginProvider for EdgeGatewayPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        descriptor()
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(contributions())
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let store = edge_gateway_store(context.shared_db.clone());
        let state = EdgeGatewayApiState::from_store(context.database_url.clone(), store);
        Ok(NativePluginRuntime {
            router: edge_gateway_router(state),
        })
    }
}

#[Singleton(name = "edge-gateway")]
pub fn edge_gateway_plugin() -> DynAdminPluginProvider {
    Arc::new(EdgeGatewayPlugin)
}


fn edge_gateway_store(shared_db: Option<az_plugin_core::Db>) -> Option<EdgeGatewayStore> {
    shared_db.map(|shared_db| {
        let mut plugin_context = build_edge_gateway_context_with_db(shared_db);
        plugin_context.resolve::<EdgeGatewayStore>()
    })
}

/// 初始化边缘网关内置天气令牌和路由。
pub async fn seed_builtin_data(shared_db: Option<az_plugin_core::Db>) -> anyhow::Result<()> {
    if let Some(store) = edge_gateway_store(shared_db) {
        store.ensure_demo_weather_token().await?;
        store.ensure_builtin_weather_route().await?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use az_plugin_core::PluginKind;

    use super::*;

    #[test]
    fn descriptor_exposes_native_runtime_contract() {
        let plugin = EdgeGatewayPlugin;
        let descriptor = plugin.descriptor();
        let contributions = plugin.contributions().unwrap();
        assert_eq!(descriptor.id, "edge-gateway");
        assert_eq!(descriptor.kind, PluginKind::Native);
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/edge-gateway/status")
        );
    }
}
