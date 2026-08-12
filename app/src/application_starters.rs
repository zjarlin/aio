//! AIO 服务端全部应用启动器的编译期聚合入口。

automod::dir!(pub "src/application_starters");

rudi::enable! {
    crate::contracts::enable();
    studio::enable();
    algorithm_center::enable();
    asset_hub::enable();
    config_center::enable();
    drive_center::enable();
    edge_gateway::enable();
    iot_center::enable();
    software_center::enable();
    ssh_plugin::enable();
    az_linux::enable();
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use az_plugin_core::{discover_plugins, plugin_key};

    use super::{
        api_error_middleware::ApiErrorMiddlewareStarter,
        capabilities::CapabilityCatalogStarter,
        convention_endpoints::ConventionEndpointIndexStarter,
        convention_routes::ConventionRoutesStarter,
        database::{DatabaseMigrationStarter, SharedDatabaseStarter},
        edge_gateway_seed::EdgeGatewaySeedStarter,
        form_state_extractor::FormStateExtractorStarter,
        native_plugins::NativePluginDiscoveryStarter,
        patch_agent::ProgramPatchAgentStarter,
        program_runtime::ProgramRuntimeStarter,
        static_web::StaticWebStarter,
        studio_routes::StudioHttpRoutesStarter,
    };
    use crate::{application_startup::ApplicationStartup, config::AppConfig};

    #[test]
    fn rudi_discovers_every_server_feature_as_app_starter() -> anyhow::Result<()> {
        super::enable();
        let mut context = rudi::Context::auto_register();
        context.insert_singleton(AppConfig::for_test());

        let starters = discover_plugins::<ApplicationStartup>(&mut context)?;
        let actual = starters
            .into_iter()
            .map(|starter| starter.key())
            .collect::<BTreeSet<_>>();
        let expected = BTreeSet::from([
            plugin_key::<DatabaseMigrationStarter>(),
            plugin_key::<SharedDatabaseStarter>(),
            plugin_key::<EdgeGatewaySeedStarter>(),
            plugin_key::<NativePluginDiscoveryStarter>(),
            plugin_key::<CapabilityCatalogStarter>(),
            plugin_key::<ConventionEndpointIndexStarter>(),
            plugin_key::<ProgramRuntimeStarter>(),
            plugin_key::<ConventionRoutesStarter>(),
            plugin_key::<ProgramPatchAgentStarter>(),
            plugin_key::<FormStateExtractorStarter>(),
            plugin_key::<StudioHttpRoutesStarter>(),
            plugin_key::<ApiErrorMiddlewareStarter>(),
            plugin_key::<StaticWebStarter>(),
        ]);

        assert_eq!(actual, expected);
        Ok(())
    }
}
