use super::model::{EndpointFuture, EndpointRequest};

/// 网盘中心 领域服务契约。
pub(crate) trait DriveService: Send + Sync {
    /// Drive Center Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Drive Task List
    fn get_tasks(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Enqueue Drive Task
    fn post_task(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
