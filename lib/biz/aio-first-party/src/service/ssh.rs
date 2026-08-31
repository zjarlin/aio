// AIO 根据元数据生成的 Service 骨架；内容修改后自动转为人工所有。
use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::ssh::contract::SshService;

#[dill::component]
#[dill::interface(dyn SshService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct SshServiceImpl;

impl SshService for SshServiceImpl {
    fn post_targets(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("保存 SSH 目标尚未实现")
        })
    }

    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("SSH 运维状态尚未实现")
        })
    }

    fn post_templates_default_apply(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("初始化 SSH 低代码模板尚未实现")
        })
    }

    fn post_commands(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("保存 SSH 命令尚未实现")
        })
    }

    fn post_collect(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("采集目标监测项尚未实现")
        })
    }

    fn post_execute(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("执行指定 SSH 命令尚未实现")
        })
    }

    fn post_ui_action(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("SSH 页面操作尚未实现")
        })
    }
}
