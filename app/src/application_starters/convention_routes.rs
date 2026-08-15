//! Studio 约定接口契约与路由启动器。

use std::sync::Arc;

use anyhow::Context as _;
use axum::Router;
use az_plugin_core::{Plugin, PluginFuture};
use studio::{BusinessModuleManager, ConventionEndpointIndex, ConventionEndpointProvider};

use super::program_runtime::SharedProgramRuntime;
use crate::application_startup::ApplicationStartup;

/// 同步业务模块并根据活动程序建立约定接口路由。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct ConventionRoutesStarter {
    shared_runtime: Arc<SharedProgramRuntime>,
    endpoint_providers: Vec<Arc<dyn ConventionEndpointProvider>>,
    business_modules: Arc<BusinessModuleManager>,
}

impl Plugin<ApplicationStartup> for ConventionRoutesStarter {
    fn build<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let endpoints = ConventionEndpointIndex::new(self.endpoint_providers.clone())?;
            let router = match self.shared_runtime.current()? {
                Some(runtime) => {
                    let draft = runtime.store().draft().await?;
                    self.business_modules
                        .reconcile(&draft.definition)
                        .context("同步 Studio 业务 Service 与 Controller 失败")?;
                    match runtime.active_image().await {
                        Some(image) => endpoints.router(image.image())?,
                        None => Router::new(),
                    }
                }
                None => Router::new(),
            };
            target.merge_router(router);
            Ok(())
        })
    }
}
