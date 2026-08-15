use anyhow::bail;
use studio::{ConventionEndpointFuture, ConventionEndpointRequest};

use crate::generated::config::contract::ConfigService;

#[dill::component]
#[dill::interface(dyn ConfigService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct ConfigServiceImpl;

impl ConfigService for ConfigServiceImpl {
    fn get_status(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Config Center Status尚未实现")
        })
    }

    fn get_dotfiles(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Dotfiles Status尚未实现")
        })
    }

    fn get_pairing(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Pairing Identity尚未实现")
        })
    }

    fn get_entries(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Config Entries尚未实现")
        })
    }

    fn post_entry(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Config Entry尚未实现")
        })
    }

    fn post_ui_action(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("配置页面操作尚未实现")
        })
    }
}
