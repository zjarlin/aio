use super::model::{EndpointFuture, EndpointRequest};

/// 软件中心 领域服务契约。
pub(crate) trait SoftwareService: Send + Sync {
    /// Software Center Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Installer Scan
    fn get_installers(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Software Packages
    fn get_packages(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Organize Installers
    fn post_organize(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Save Software Package
    fn post_package(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
