// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::LinuxService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn LinuxService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct LinuxServiceImpl;

impl LinuxService for LinuxServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Linux Status尚未实现")
        })
    }

    fn get_profiles(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Linux Profiles尚未实现")
        })
    }

    fn get_setup_catalog(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Setup Catalog尚未实现")
        })
    }

    fn post_bootstrap_plan(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Bootstrap Plan尚未实现")
        })
    }

    fn get_bootstrap_script(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Bootstrap Script尚未实现")
        })
    }
}
