use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 网盘中心 领域服务契约。
pub trait DriveService: Send + Sync {
    /// Drive Center Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Drive Task List
    fn get_tasks(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Enqueue Drive Task
    fn post_task(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
