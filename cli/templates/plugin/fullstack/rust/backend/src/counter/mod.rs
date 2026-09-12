mod service;
mod service_impl;

use anyhow::{Context as _, Result, ensure};
use az_plugin_manifest::PluginRequest;
use fullstack_model::IncrementRequest;
use service::CounterService;
use service_impl::Counter;

pub(super) fn handle(request: &str) -> Result<String> {
    let PluginRequest::ServiceRequest {
        method,
        path,
        body,
        tenant_id,
        ..
    } = serde_json::from_str(request).context("请求格式无效")?
    else {
        anyhow::bail!("只接受服务请求")
    };
    ensure!(
        method == "POST" && path == "/counter/increment",
        "未声明的操作"
    );
    let request: IncrementRequest = serde_json::from_str(&body).context("计数请求格式无效")?;
    let catalog = dill::CatalogBuilder::new()
        .add_value(Counter)
        .bind::<dyn CounterService, Counter>()
        .build();
    let result = catalog
        .get_one::<dyn CounterService>()?
        .increment(request, tenant_id)?;
    serde_json::to_string(&result).context("序列化计数响应失败")
}
