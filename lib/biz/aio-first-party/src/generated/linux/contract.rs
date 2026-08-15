use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// Linux 领域服务契约。
pub trait LinuxService: Send + Sync {
    /// Linux Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Linux Profiles
    fn get_profiles(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Setup Catalog
    fn get_setup_catalog(&self, request: ConventionEndpointRequest)
    -> ConventionEndpointFuture<'_>;
    /// Bootstrap Plan
    fn post_bootstrap_plan(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_>;
    /// Bootstrap Script
    fn get_bootstrap_script(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_>;
}
