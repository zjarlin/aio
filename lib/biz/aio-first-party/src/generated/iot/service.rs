use super::model::{EndpointFuture, EndpointRequest};

/// 物联网设备 领域服务契约。
pub(crate) trait IotService: Send + Sync {
    /// 新建设备
    fn post_devices(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 物联网状态
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 初始化物联网模板
    fn post_templates_default_apply(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 接收模拟遥测
    fn post_devices_device_code_fixture_telemetry(
        &self,
        request: EndpointRequest,
    ) -> EndpointFuture<'_>;
    /// 物联网页面操作
    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
