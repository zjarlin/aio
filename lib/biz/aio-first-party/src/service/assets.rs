use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::assets::contract::AssetsService;

#[dill::component]
#[dill::interface(dyn AssetsService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct AssetsServiceImpl;

impl AssetsService for AssetsServiceImpl {
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Asset Hub Status尚未实现")
        })
    }

    fn get_skills(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Scanned Skills尚未实现")
        })
    }

    fn get_assets(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Asset List尚未实现")
        })
    }

    fn post_asset(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Asset尚未实现")
        })
    }
}
