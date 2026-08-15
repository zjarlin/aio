use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::LinuxService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn LinuxService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "Linux Status"
    }

    fn endpoint_id(&self) -> &'static str {
        "df69e1c1-8687-b7ea-05f8-9048ea1ffbdc"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetProfilesController {
    #[debug(skip)]
    service: Arc<dyn LinuxService>,
}

impl ConventionEndpointProvider for GetProfilesController {
    fn comment(&self) -> &'static str {
        "Linux Profiles"
    }

    fn endpoint_id(&self) -> &'static str {
        "627b86a2-22ec-0663-4a03-2d73a1619b31"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_profiles(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetSetupCatalogController {
    #[debug(skip)]
    service: Arc<dyn LinuxService>,
}

impl ConventionEndpointProvider for GetSetupCatalogController {
    fn comment(&self) -> &'static str {
        "Setup Catalog"
    }

    fn endpoint_id(&self) -> &'static str {
        "7e960357-1d77-07df-c7ee-04dd00a99aa9"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_setup_catalog(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostBootstrapPlanController {
    #[debug(skip)]
    service: Arc<dyn LinuxService>,
}

impl ConventionEndpointProvider for PostBootstrapPlanController {
    fn comment(&self) -> &'static str {
        "Bootstrap Plan"
    }

    fn endpoint_id(&self) -> &'static str {
        "8da2e6e5-d447-3dc1-be0d-5415ae113725"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_bootstrap_plan(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetBootstrapScriptController {
    #[debug(skip)]
    service: Arc<dyn LinuxService>,
}

impl ConventionEndpointProvider for GetBootstrapScriptController {
    fn comment(&self) -> &'static str {
        "Bootstrap Script"
    }

    fn endpoint_id(&self) -> &'static str {
        "e0712a36-4e78-d1a8-e666-ea45dff2fd02"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_bootstrap_script(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetProfilesController>();
    builder.add::<GetSetupCatalogController>();
    builder.add::<PostBootstrapPlanController>();
    builder.add::<GetBootstrapScriptController>();
}
