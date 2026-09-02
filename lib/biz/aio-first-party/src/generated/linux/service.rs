use super::model::{EndpointFuture, EndpointRequest};

/// Linux 领域服务契约。
pub(crate) trait LinuxService: Send + Sync {
    /// Linux Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Linux Profiles
    fn get_profiles(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Setup Catalog
    fn get_setup_catalog(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Bootstrap Plan
    fn post_bootstrap_plan(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Bootstrap Script
    fn get_bootstrap_script(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
