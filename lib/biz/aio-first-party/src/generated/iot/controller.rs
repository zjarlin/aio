use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::IotService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostDevicesController {
    #[debug(skip)]
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostDevicesController {
    fn comment(&self) -> &'static str {
        "新建设备"
    }

    fn endpoint_id(&self) -> &'static str {
        "7efe301d-7330-fb65-ca3c-b1d626374a7f"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_devices(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "物联网状态"
    }

    fn endpoint_id(&self) -> &'static str {
        "214dd5da-d598-c3e1-f03f-79bc97888824"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostTemplatesDefaultApplyController {
    #[debug(skip)]
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostTemplatesDefaultApplyController {
    fn comment(&self) -> &'static str {
        "初始化物联网模板"
    }

    fn endpoint_id(&self) -> &'static str {
        "7778cd07-5953-0ac7-1c98-a83f34916792"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_templates_default_apply(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostDevicesDeviceCodeFixtureTelemetryController {
    #[debug(skip)]
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostDevicesDeviceCodeFixtureTelemetryController {
    fn comment(&self) -> &'static str {
        "接收模拟遥测"
    }

    fn endpoint_id(&self) -> &'static str {
        "7659e936-1339-4447-7c6d-829aa40bf56d"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service
            .post_devices_device_code_fixture_telemetry(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostUiActionController {
    #[debug(skip)]
    service: Arc<dyn IotService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn comment(&self) -> &'static str {
        "物联网页面操作"
    }

    fn endpoint_id(&self) -> &'static str {
        "bb197142-aa68-d99b-08ff-22fbaf229b5b"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
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
