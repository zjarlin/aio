use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::AssetsService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        "c6961230-f0b2-25a6-e4cc-19fc2700490b"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetSkillsController {
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetSkillsController {
    fn endpoint_id(&self) -> &'static str {
        "b26fcd61-739d-dbb4-8cd4-690674c2b1a2"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_skills(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetAssetsController {
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetAssetsController {
    fn endpoint_id(&self) -> &'static str {
        "a35d8703-ea30-3dd2-7e35-82045cf0a33e"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_assets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostAssetController {
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for PostAssetController {
    fn endpoint_id(&self) -> &'static str {
        "7fa4bf16-b191-b898-b60a-18cc8f99a6de"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_asset(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetSkillsController>();
    builder.add::<GetAssetsController>();
    builder.add::<PostAssetController>();
}
