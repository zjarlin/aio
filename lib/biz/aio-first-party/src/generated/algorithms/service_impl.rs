// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::AlgorithmsService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn AlgorithmsService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct AlgorithmsServiceImpl;

impl AlgorithmsService for AlgorithmsServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Algorithm Center Status尚未实现")
        })
    }

    fn get_components(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Algorithm Components尚未实现")
        })
    }

    fn post_process(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Process Video尚未实现")
        })
    }

    fn post_upload(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Upload Video尚未实现")
        })
    }

    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("算法页面操作尚未实现")
        })
    }
}
