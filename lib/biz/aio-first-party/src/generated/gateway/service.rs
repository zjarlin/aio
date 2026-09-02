use super::model::{EndpointFuture, EndpointRequest};

/// 边缘网关 领域服务契约。
pub(crate) trait GatewayService: Send + Sync {
    /// Edge Gateway Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Gateway Example Plan
    fn get_example(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Run Gateway Plan
    fn post_run(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Callable Edge Assets
    fn get_assets(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Weather Current Asset
    fn post_assets_weather_current(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Edge Asset Usage
    fn get_assets_usage(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Managed API Routes
    fn get_routes(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Save Managed API Route
    fn post_route(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 网关路由页面操作
    fn post_ui_route(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Gateway Flows
    fn get_flows(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Save Gateway Flow
    fn post_flow(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
