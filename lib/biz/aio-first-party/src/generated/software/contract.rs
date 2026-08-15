use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 软件中心 领域服务契约。
pub trait SoftwareService: Send + Sync {
    /// Software Center Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Installer Scan
    fn get_installers(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Software Packages
    fn get_packages(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Organize Installers
    fn post_organize(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Save Software Package
    fn post_package(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
