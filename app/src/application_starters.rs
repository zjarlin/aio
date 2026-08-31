//! AIO 服务端全部应用启动器的显式聚合入口。

automod::dir!(pub "src/application_starters");

use az_plugin_core::{PluginGroup, PluginGroupBuilder};
use dill::CatalogBuilder;
use studio::BusinessModuleManager;

use self::{
    convention_routes::ConventionRoutesStarter,
    database::{DatabaseMigrationStarter, SharedDatabaseStarter},
    program_runtime::ProgramRuntimeStarter,
    static_web::StaticWebStarter,
    studio_routes::StudioHttpRoutesStarter,
};
use crate::application_startup::ApplicationStartup;

/// 按具体 Rust 类型向 Dill 注册全部启动器。
pub fn register(builder: &mut CatalogBuilder) {
    database::register(builder);
    program_runtime::register(builder);
    builder
        .add_value(BusinessModuleManager::repository())
        .add::<ConventionRoutesStarter>()
        .add::<StudioHttpRoutesStarter>()
        .add::<StaticWebStarter>();
}

/// AIO 服务端的默认插件组，声明唯一且确定的构建顺序。
pub struct AioPlugins;

impl PluginGroup<ApplicationStartup> for AioPlugins {
    fn build(self) -> PluginGroupBuilder<ApplicationStartup> {
        PluginGroupBuilder::default()
            .add::<DatabaseMigrationStarter>()
            .add::<SharedDatabaseStarter>()
            .add::<ProgramRuntimeStarter>()
            .add::<ConventionRoutesStarter>()
            .add::<StudioHttpRoutesStarter>()
            .add::<StaticWebStarter>()
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, collections::HashSet};

    use az_plugin_core::{App, PluginGroup as _};
    use dill::{AllOf, Catalog};

    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn dill_resolves_the_explicit_aio_plugin_group() -> anyhow::Result<()> {
        let config = AppConfig::for_test();
        let mut builder = Catalog::builder();
        builder
            .add_value(config)
            .add_value(studio::ProgramPatchAgent::default())
            .add_value(studio::FormStateExtractor::default());
        register(&mut builder);
        builder.validate()?;
        let catalog = builder.build();

        let starters = AioPlugins.build().resolve(&catalog)?;
        let actual = starters
            .iter()
            .map(|starter| starter.as_ref().type_id())
            .collect::<Vec<_>>();
        let expected = [
            TypeId::of::<DatabaseMigrationStarter>(),
            TypeId::of::<SharedDatabaseStarter>(),
            TypeId::of::<ProgramRuntimeStarter>(),
            TypeId::of::<ConventionRoutesStarter>(),
            TypeId::of::<StudioHttpRoutesStarter>(),
            TypeId::of::<StaticWebStarter>(),
        ];

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generated_controllers_have_unique_type_ids_and_valid_dependencies() -> anyhow::Result<()> {
        let config = AppConfig::for_test();
        let mut builder = Catalog::builder();
        builder
            .add_value(config)
            .add_value(studio::ProgramPatchAgent::default())
            .add_value(studio::FormStateExtractor::default());
        register(&mut builder);
        business::register(&mut builder);
        builder.validate()?;
        let catalog = builder.build();
        let controllers = catalog.get::<AllOf<dyn studio::ConventionEndpointProvider>>()?;
        let type_ids = controllers
            .iter()
            .map(|controller| controller.as_ref().type_id())
            .collect::<HashSet<_>>();
        assert_eq!(controllers.len(), business::ENDPOINT_COUNT);
        assert_eq!(type_ids.len(), controllers.len());
        Ok(())
    }

    #[tokio::test]
    async fn injected_startup_dependencies_build_without_database() -> anyhow::Result<()> {
        let config = AppConfig::for_test();
        let mut builder = Catalog::builder();
        builder
            .add_value(config)
            .add_value(studio::ProgramPatchAgent::default())
            .add_value(studio::FormStateExtractor::default());
        register(&mut builder);
        business::register(&mut builder);
        builder.validate()?;
        let catalog = builder.build();
        let plugins = AioPlugins.build().resolve(&catalog)?;
        let mut app = App::new(ApplicationStartup::default());

        app.add_plugins(plugins).await?;
        let _router = app.into_inner().into_router();
        Ok(())
    }
}
