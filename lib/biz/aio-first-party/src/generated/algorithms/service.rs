use super::model::{EndpointFuture, EndpointRequest};

/// 算法中心 领域服务契约。
pub(crate) trait AlgorithmsService: Send + Sync {
    /// Algorithm Center Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Algorithm Components
    fn get_components(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Process Video
    fn post_process(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Upload Video
    fn post_upload(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 算法页面操作
    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
