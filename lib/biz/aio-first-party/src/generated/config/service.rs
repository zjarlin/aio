use super::model::{EndpointFuture, EndpointRequest};

/// 配置中心 领域服务契约。
pub(crate) trait ConfigService: Send + Sync {
    /// Config Center Status
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Dotfiles Status
    fn get_dotfiles(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Pairing Identity
    fn get_pairing(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Config Entries
    fn get_entries(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// Save Config Entry
    fn post_entry(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 配置页面操作
    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
