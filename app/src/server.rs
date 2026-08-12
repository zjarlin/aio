use std::net::SocketAddr;

use anyhow::{Context as _, Result};
use az_plugin_core::{DynPlugin, discover_plugins, install_plugins};
use rudi::Context;

use crate::{application_startup::ApplicationStartup, config::AppConfig};

pub fn run() -> Result<()> {
    crate::application_starters::enable();

    let mut di = Context::auto_register();
    let config = AppConfig::load().context("加载 AIO 应用配置失败")?;
    di.insert_singleton(config.clone());
    let starters =
        discover_plugins::<ApplicationStartup>(&mut di).context("发现 AIO 应用启动器失败")?;
    let startup = ApplicationStartup::new(di);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建 AIO runtime 失败")?;
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind(address))
        .with_context(|| format!("绑定 AIO 监听地址失败: {address}"))?;

    runtime.block_on(serve(listener, startup, starters))
}

async fn serve(
    listener: tokio::net::TcpListener,
    mut startup: ApplicationStartup,
    starters: Vec<DynPlugin<ApplicationStartup>>,
) -> Result<()> {
    install_plugins(&mut startup, starters)
        .await
        .context("执行 AIO 应用启动器失败")?;
    let app = startup.into_router();
    let address = listener.local_addr().context("读取 AIO 监听地址失败")?;
    println!("AIO listening on http://{address}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
