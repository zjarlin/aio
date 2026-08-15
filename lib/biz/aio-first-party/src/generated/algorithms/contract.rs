use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 算法中心 领域服务契约。
pub trait AlgorithmsService: Send + Sync {
    /// Algorithm Center Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Algorithm Components
    fn get_components(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Process Video
    fn post_process(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Upload Video
    fn post_upload(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// 算法页面操作
    fn post_ui_action(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
