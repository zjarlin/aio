use super::model::{EndpointFuture, EndpointRequest};

/// 资产中心 领域服务契约。
pub(crate) trait AssetsService: Send + Sync {
    /// Asset Hub Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Scanned Skills
    fn get_skills(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Asset List
    fn get_assets(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Save Asset
    fn post_asset(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
