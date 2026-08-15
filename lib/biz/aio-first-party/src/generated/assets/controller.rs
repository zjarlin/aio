use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::AssetsService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "Asset Hub Status"
    }

    fn endpoint_id(&self) -> &'static str {
        "c6961230-f0b2-25a6-e4cc-19fc2700490b"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetSkillsController {
    #[debug(skip)]
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetSkillsController {
    fn comment(&self) -> &'static str {
        "Scanned Skills"
    }

    fn endpoint_id(&self) -> &'static str {
        "b26fcd61-739d-dbb4-8cd4-690674c2b1a2"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_skills(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetAssetsController {
    #[debug(skip)]
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for GetAssetsController {
    fn comment(&self) -> &'static str {
        "Asset List"
    }

    fn endpoint_id(&self) -> &'static str {
        "a35d8703-ea30-3dd2-7e35-82045cf0a33e"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_assets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostAssetController {
    #[debug(skip)]
    service: Arc<dyn AssetsService>,
}

impl ConventionEndpointProvider for PostAssetController {
    fn comment(&self) -> &'static str {
        "Save Asset"
    }

    fn endpoint_id(&self) -> &'static str {
        "7fa4bf16-b191-b898-b60a-18cc8f99a6de"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_asset(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetSkillsController>();
    builder.add::<GetAssetsController>();
    builder.add::<PostAssetController>();
}
