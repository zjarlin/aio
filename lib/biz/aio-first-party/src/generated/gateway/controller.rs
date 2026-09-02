use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::GatewayService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("a68f4b3a-8281-8451-4028-b3ca3d7589a9")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetExampleController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetExampleController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("ccea1686-7e08-0c60-b282-b85d7ad7c1f0")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_example(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostRunController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostRunController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("090def59-8a00-6a9b-5471-e92bfe354f97")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_run(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetAssetsController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetAssetsController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("28d32576-88ce-6523-43b1-1eaeaec4e4e9")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_assets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostAssetsWeatherCurrentController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostAssetsWeatherCurrentController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("62efd665-5b51-ef03-3e16-c68cb5203f7e")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_assets_weather_current(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetAssetsUsageController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetAssetsUsageController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("2474cc5d-2bfd-08cd-a14b-eb75dba407a9")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_assets_usage(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetRoutesController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetRoutesController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("217352b3-dbc7-8453-ae0d-1e81d26304f6")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_routes(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostRouteController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostRouteController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("39660ac3-aa6b-7987-e5d3-dde7af3c0754")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_route(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostUiRouteController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostUiRouteController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("f7c99d0c-bf58-d6a5-824d-e8a6e9c1541a")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_ui_route(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetFlowsController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetFlowsController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("24a16a7d-063a-ad95-91b8-117468e18e1a")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_flows(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostFlowController {
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostFlowController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("6c795e5f-1fd9-6adc-d760-52865b677ec3")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_flow(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetExampleController>();
    builder.add::<PostRunController>();
    builder.add::<GetAssetsController>();
    builder.add::<PostAssetsWeatherCurrentController>();
    builder.add::<GetAssetsUsageController>();
    builder.add::<GetRoutesController>();
    builder.add::<PostRouteController>();
    builder.add::<PostUiRouteController>();
    builder.add::<GetFlowsController>();
    builder.add::<PostFlowController>();
}
