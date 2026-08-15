use std::net::SocketAddr;

use anyhow::{Context as _, Result};
use az_plugin_core::{App, PluginGroup};
use dill::Catalog;
use studio::{FormStateExtractor, ProgramPatchAgent};

use crate::{
    application_starters::AioPlugins, application_startup::ApplicationStartup, config::AppConfig,
};

pub fn run(register_business: fn(&mut dill::CatalogBuilder)) -> Result<()> {
    let config = AppConfig::load().context("加载 AIO 应用配置失败")?;
    let mut builder = Catalog::builder();
    builder
        .add_value(config.clone())
        .add_value(ProgramPatchAgent::from_env()?)
        .add_value(FormStateExtractor::from_env()?);
    crate::application_starters::register(&mut builder);
    register_business(&mut builder);
    builder.validate().context("校验 AIO Dill 依赖图失败")?;
    let catalog = builder.build();
    let plugins = AioPlugins
        .build()
        .resolve(&catalog)
        .context("解析 AIO 默认插件组失败")?;
    let app = App::new(ApplicationStartup::default());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建 AIO runtime 失败")?;
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind(address))
        .with_context(|| format!("绑定 AIO 监听地址失败: {address}"))?;

    runtime.block_on(serve(listener, app, plugins))
}

async fn serve(
    listener: tokio::net::TcpListener,
    mut app: App<ApplicationStartup>,
    plugins: Vec<az_plugin_core::DynPlugin<ApplicationStartup>>,
) -> Result<()> {
    app.add_plugins(plugins)
        .await
        .context("构建 AIO 默认插件组失败")?;
    let router = app.into_inner().into_router();
    let address = listener.local_addr().context("读取 AIO 监听地址失败")?;
    println!("AIO listening on http://{address}");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
