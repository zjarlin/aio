use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::IotService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostDevicesController {
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostDevicesController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("7efe301d-7330-fb65-ca3c-b1d626374a7f")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_devices(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("214dd5da-d598-c3e1-f03f-79bc97888824")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostTemplatesDefaultApplyController {
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostTemplatesDefaultApplyController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("7778cd07-5953-0ac7-1c98-a83f34916792")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_templates_default_apply(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostDevicesDeviceCodeFixtureTelemetryController {
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostDevicesDeviceCodeFixtureTelemetryController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("7659e936-1339-4447-7c6d-829aa40bf56d")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service
            .post_devices_device_code_fixture_telemetry(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostUiActionController {
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("bb197142-aa68-d99b-08ff-22fbaf229b5b")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_ui_action(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<PostDevicesController>();
    builder.add::<GetStatusController>();
    builder.add::<PostTemplatesDefaultApplyController>();
    builder.add::<PostDevicesDeviceCodeFixtureTelemetryController>();
    builder.add::<PostUiActionController>();
}
