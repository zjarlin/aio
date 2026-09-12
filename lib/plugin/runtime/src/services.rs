use anyhow::Result;
use async_trait::async_trait;
use az_plugin_contract::InvocationScope;

use crate::bindings::aio::plugin::transport::{Request, Response};

#[async_trait]
pub trait HostServices: Send + Sync {
    async fn authorize(&self, scope: &InvocationScope, permission: &str) -> Result<bool>;
    async fn manage(&self, scope: &InvocationScope, request: Request) -> Result<Response>;
}
