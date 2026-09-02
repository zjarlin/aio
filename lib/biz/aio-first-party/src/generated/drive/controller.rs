use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::DriveService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn DriveService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        "5c696f9e-906c-6e17-9d3b-d9121726e83e"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetTasksController {
    service: Arc<dyn DriveService>,
}

impl ConventionEndpointProvider for GetTasksController {
    fn endpoint_id(&self) -> &'static str {
        "25912f41-e1d1-e655-df3d-0d2491d334a4"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_tasks(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostTaskController {
    service: Arc<dyn DriveService>,
}

impl ConventionEndpointProvider for PostTaskController {
    fn endpoint_id(&self) -> &'static str {
        "8818fbd1-7a34-8525-f5dd-f20ce396647a"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_task(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetTasksController>();
    builder.add::<PostTaskController>();
}
