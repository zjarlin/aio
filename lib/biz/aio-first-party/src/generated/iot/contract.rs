use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 物联网设备 领域服务契约。
pub trait IotService: Send + Sync {
    /// 新建设备
    fn post_devices(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// 物联网状态
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// 初始化物联网模板
    fn post_templates_default_apply(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_>;
    /// 接收模拟遥测
    fn post_devices_device_code_fixture_telemetry(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_>;
    /// 物联网页面操作
    fn post_ui_action(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
