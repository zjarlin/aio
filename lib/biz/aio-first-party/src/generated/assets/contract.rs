use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 资产中心 领域服务契约。
pub trait AssetsService: Send + Sync {
    /// Asset Hub Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Scanned Skills
    fn get_skills(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Asset List
    fn get_assets(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Save Asset
    fn post_asset(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
