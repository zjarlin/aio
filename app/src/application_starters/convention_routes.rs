//! Studio 约定接口契约与路由启动器。

use std::sync::Arc;

use anyhow::Context as _;
use axum::Router;
use az_plugin_core::{DynPlugin, Plugin, PluginFuture, PluginType};
use studio::ConventionContractManager;

use super::{
    convention_endpoints::ConventionEndpointIndexStarter, program_runtime::ProgramRuntimeStarter,
};
use crate::application_startup::ApplicationStartup;

/// 同步约定文件并根据活动程序建立约定接口路由。
pub struct ConventionRoutesStarter;

impl Plugin<ApplicationStartup> for ConventionRoutesStarter {
    fn order(&self) -> i32 {
        50
    }

    fn dependencies(&self) -> Vec<PluginType<ApplicationStartup>> {
        vec![
            PluginType::of::<ProgramRuntimeStarter>(),
            PluginType::of::<ConventionEndpointIndexStarter>(),
        ]
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let contracts = ConventionContractManager::workspace_app();
            let endpoints = target.take_convention_endpoints()?;
            let router = match target.program_runtime()? {
                Some(runtime) => {
                    let draft = runtime.store().draft().await?;
                    contracts
                        .reconcile(&draft.definition)
                        .context("同步 Studio 约定接口文件失败")?;
                    match runtime.active_image().await {
                        Some(image) => endpoints.router(image.image())?,
                        None => Router::new(),
                    }
                }
                None => Router::new(),
            };
            target.set_convention_contracts(contracts);
            target.merge_router(router);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<ConventionRoutesStarter>())]
pub fn convention_routes_starter() -> DynPlugin<ApplicationStartup> {
    Arc::new(ConventionRoutesStarter)
}
