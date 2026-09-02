use super::model::{EndpointFuture, EndpointRequest};

/// SSH 目标 领域服务契约。
pub(crate) trait SshService: Send + Sync {
    /// 保存 SSH 目标
    fn post_targets(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// SSH 运维状态
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 初始化 SSH 低代码模板
    fn post_templates_default_apply(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 保存 SSH 命令
    fn post_commands(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 采集目标监测项
    fn post_collect(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// 执行指定 SSH 命令
    fn post_execute(&self, request: EndpointRequest) -> EndpointFuture<'_>;
    /// SSH 页面操作
    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_>;
}
