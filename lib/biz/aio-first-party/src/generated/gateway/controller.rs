use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::GatewayService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "Edge Gateway Status"
    }

    fn endpoint_id(&self) -> &'static str {
        "a68f4b3a-8281-8451-4028-b3ca3d7589a9"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetExampleController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetExampleController {
    fn comment(&self) -> &'static str {
        "Gateway Example Plan"
    }

    fn endpoint_id(&self) -> &'static str {
        "ccea1686-7e08-0c60-b282-b85d7ad7c1f0"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_example(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostRunController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostRunController {
    fn comment(&self) -> &'static str {
        "Run Gateway Plan"
    }

    fn endpoint_id(&self) -> &'static str {
        "090def59-8a00-6a9b-5471-e92bfe354f97"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_run(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetAssetsController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetAssetsController {
    fn comment(&self) -> &'static str {
        "Callable Edge Assets"
    }

    fn endpoint_id(&self) -> &'static str {
        "28d32576-88ce-6523-43b1-1eaeaec4e4e9"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_assets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostAssetsWeatherCurrentController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostAssetsWeatherCurrentController {
    fn comment(&self) -> &'static str {
        "Weather Current Asset"
    }

    fn endpoint_id(&self) -> &'static str {
        "62efd665-5b51-ef03-3e16-c68cb5203f7e"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_assets_weather_current(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetAssetsUsageController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetAssetsUsageController {
    fn comment(&self) -> &'static str {
        "Edge Asset Usage"
    }

    fn endpoint_id(&self) -> &'static str {
        "2474cc5d-2bfd-08cd-a14b-eb75dba407a9"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_assets_usage(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetRoutesController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetRoutesController {
    fn comment(&self) -> &'static str {
        "Managed API Routes"
    }

    fn endpoint_id(&self) -> &'static str {
        "217352b3-dbc7-8453-ae0d-1e81d26304f6"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_routes(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostRouteController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostRouteController {
    fn comment(&self) -> &'static str {
        "Save Managed API Route"
    }

    fn endpoint_id(&self) -> &'static str {
        "39660ac3-aa6b-7987-e5d3-dde7af3c0754"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_route(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostUiRouteController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostUiRouteController {
    fn comment(&self) -> &'static str {
        "网关路由页面操作"
    }

    fn endpoint_id(&self) -> &'static str {
        "f7c99d0c-bf58-d6a5-824d-e8a6e9c1541a"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_ui_route(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetFlowsController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for GetFlowsController {
    fn comment(&self) -> &'static str {
        "Gateway Flows"
    }

    fn endpoint_id(&self) -> &'static str {
        "24a16a7d-063a-ad95-91b8-117468e18e1a"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_flows(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostFlowController {
    #[debug(skip)]
    service: Arc<dyn GatewayService>,
}

impl ConventionEndpointProvider for PostFlowController {
    fn comment(&self) -> &'static str {
        "Save Gateway Flow"
    }

    fn endpoint_id(&self) -> &'static str {
        "6c795e5f-1fd9-6adc-d760-52865b677ec3"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
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
