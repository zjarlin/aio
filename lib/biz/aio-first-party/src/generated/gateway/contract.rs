use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 边缘网关 领域服务契约。
pub trait GatewayService: Send + Sync {
    /// Edge Gateway Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Gateway Example Plan
    fn get_example(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Run Gateway Plan
    fn post_run(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Callable Edge Assets
    fn get_assets(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Weather Current Asset
    fn post_assets_weather_current(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_>;
    /// Edge Asset Usage
    fn get_assets_usage(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Managed API Routes
    fn get_routes(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Save Managed API Route
    fn post_route(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// 网关路由页面操作
    fn post_ui_route(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Gateway Flows
    fn get_flows(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Save Gateway Flow
    fn post_flow(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
