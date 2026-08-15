use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::linux::contract::LinuxService;

#[dill::component]
#[dill::interface(dyn LinuxService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct LinuxServiceImpl;

impl LinuxService for LinuxServiceImpl {
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Linux Status尚未实现")
        })
    }

    fn get_profiles(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Linux Profiles尚未实现")
        })
    }

    fn get_setup_catalog(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Setup Catalog尚未实现")
        })
    }

    fn post_bootstrap_plan(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Bootstrap Plan尚未实现")
        })
    }

    fn get_bootstrap_script(
        &self,
        request: ConventionEndpointRequest,
    ) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Bootstrap Script尚未实现")
        })
    }
}
