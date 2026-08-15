use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::drive::contract::DriveService;

#[dill::component]
#[dill::interface(dyn DriveService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct DriveServiceImpl;

impl DriveService for DriveServiceImpl {
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Drive Center Status尚未实现")
        })
    }

    fn get_tasks(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Drive Task List尚未实现")
        })
    }

    fn post_task(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Enqueue Drive Task尚未实现")
        })
    }
}
