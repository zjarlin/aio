use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::AlgorithmsService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn AlgorithmsService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        "bc442bc5-6d94-fb86-3ccc-735e7b505580"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetComponentsController {
    service: Arc<dyn AlgorithmsService>,
}

impl ConventionEndpointProvider for GetComponentsController {
    fn endpoint_id(&self) -> &'static str {
        "301e08ca-166c-a89e-c8fb-186f3a9bf906"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_components(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostProcessController {
    service: Arc<dyn AlgorithmsService>,
}

impl ConventionEndpointProvider for PostProcessController {
    fn endpoint_id(&self) -> &'static str {
        "af3c3c5e-0786-ed7b-5b7d-27e15a3ff438"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_process(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostUploadController {
    service: Arc<dyn AlgorithmsService>,
}

impl ConventionEndpointProvider for PostUploadController {
    fn endpoint_id(&self) -> &'static str {
        "ba4355e4-ec9c-b7da-0e86-b041f693fc40"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_upload(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostUiActionController {
    service: Arc<dyn AlgorithmsService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn endpoint_id(&self) -> &'static str {
        "0e2b0418-6fb3-1df6-9e86-422642f2fa66"
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_ui_action(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetComponentsController>();
    builder.add::<PostProcessController>();
    builder.add::<PostUploadController>();
    builder.add::<PostUiActionController>();
}
