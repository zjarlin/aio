// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::SoftwareService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn SoftwareService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct SoftwareServiceImpl;

impl SoftwareService for SoftwareServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Software Center Status尚未实现")
        })
    }

    fn get_installers(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Installer Scan尚未实现")
        })
    }

    fn get_packages(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Software Packages尚未实现")
        })
    }

    fn post_organize(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Organize Installers尚未实现")
        })
    }

    fn post_package(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Software Package尚未实现")
        })
    }
}
