use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::SoftwareService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn SoftwareService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        "71ffa22a-0124-8037-c9b5-7a720422137f"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetInstallersController {
    service: Arc<dyn SoftwareService>,
}

impl ConventionEndpointProvider for GetInstallersController {
    fn endpoint_id(&self) -> &'static str {
        "3b8b34b9-5518-fcc6-9182-ec0af247b2b5"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_installers(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetPackagesController {
    service: Arc<dyn SoftwareService>,
}

impl ConventionEndpointProvider for GetPackagesController {
    fn endpoint_id(&self) -> &'static str {
        "6ed53d73-7729-420d-092d-4f48b03a968c"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_packages(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostOrganizeController {
    service: Arc<dyn SoftwareService>,
}

impl ConventionEndpointProvider for PostOrganizeController {
    fn endpoint_id(&self) -> &'static str {
        "9ac74b1e-0466-f39a-f5d7-cad2d7d69351"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_organize(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostPackageController {
    service: Arc<dyn SoftwareService>,
}

impl ConventionEndpointProvider for PostPackageController {
    fn endpoint_id(&self) -> &'static str {
        "bc6b003c-2fc4-0e60-3865-65764e52830b"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_package(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetInstallersController>();
    builder.add::<GetPackagesController>();
    builder.add::<PostOrganizeController>();
    builder.add::<PostPackageController>();
}
