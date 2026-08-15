use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

/// 配置中心 领域服务契约。
pub trait ConfigService: Send + Sync {
    /// Config Center Status
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Dotfiles Status
    fn get_dotfiles(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Pairing Identity
    fn get_pairing(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Config Entries
    fn get_entries(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// Save Config Entry
    fn post_entry(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
    /// 配置页面操作
    fn post_ui_action(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_>;
}
