// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::GatewayService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn GatewayService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct GatewayServiceImpl;

impl GatewayService for GatewayServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Edge Gateway Status尚未实现")
        })
    }

    fn get_example(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Gateway Example Plan尚未实现")
        })
    }

    fn post_run(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Run Gateway Plan尚未实现")
        })
    }

    fn get_assets(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Callable Edge Assets尚未实现")
        })
    }

    fn post_assets_weather_current(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Weather Current Asset尚未实现")
        })
    }

    fn get_assets_usage(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Edge Asset Usage尚未实现")
        })
    }

    fn get_routes(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Managed API Routes尚未实现")
        })
    }

    fn post_route(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Managed API Route尚未实现")
        })
    }

    fn post_ui_route(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("网关路由页面操作尚未实现")
        })
    }

    fn get_flows(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Gateway Flows尚未实现")
        })
    }

    fn post_flow(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Gateway Flow尚未实现")
        })
    }
}
