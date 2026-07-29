//! SSH 服务器运维插件的 SSR 状态。

use std::sync::OnceLock;

use anyhow::{Context, anyhow};

use crate::service::SshService;

static SERVICE: OnceLock<SshService> = OnceLock::new();

/// 安装插件级 SSH 服务。
pub fn install_service(service: SshService) {
    let _ = SERVICE.set(service);
}

/// 读取插件级 SSH 服务。
pub fn service() -> anyhow::Result<SshService> {
    SERVICE
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("SSH 服务器运维服务尚未初始化"))
}

/// 在 SSR 同步渲染路径执行 SSH 异步查询。
pub fn run_ssh_future<T, F>(future: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("创建 SSH 运维 SSR runtime 失败")?;
    runtime.block_on(future)
}
