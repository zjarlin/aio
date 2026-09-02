// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::DriveService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn DriveService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct DriveServiceImpl;

impl DriveService for DriveServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Drive Center Status尚未实现")
        })
    }

    fn get_tasks(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Drive Task List尚未实现")
        })
    }

    fn post_task(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Enqueue Drive Task尚未实现")
        })
    }
}
