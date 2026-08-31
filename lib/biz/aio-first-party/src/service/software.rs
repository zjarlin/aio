// AIO 根据元数据生成的 Service 骨架；内容修改后自动转为人工所有。
use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::software::contract::SoftwareService;

#[dill::component]
#[dill::interface(dyn SoftwareService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct SoftwareServiceImpl;

impl SoftwareService for SoftwareServiceImpl {
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Software Center Status尚未实现")
        })
    }

    fn get_installers(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Installer Scan尚未实现")
        })
    }

    fn get_packages(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Software Packages尚未实现")
        })
    }

    fn post_organize(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Organize Installers尚未实现")
        })
    }

    fn post_package(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Software Package尚未实现")
        })
    }
}
