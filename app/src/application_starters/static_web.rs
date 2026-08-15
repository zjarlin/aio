//! AIO 健康检查、跳转和静态 Web 路由启动器。

use std::{path::PathBuf, sync::Arc};

use axum::{Router, response::Redirect, routing::get};
use az_plugin_core::{Plugin, PluginFuture};
use tower_http::services::{ServeDir, ServeFile};

use crate::{application_startup::ApplicationStartup, config::AppConfig};

/// 安装应用入口、健康检查和 Dioxus 静态产物路由。
#[dill::component]
#[dill::interface(dyn Plugin<ApplicationStartup>)]
#[dill::scope(dill::Singleton)]
pub(super) struct StaticWebStarter {
    config: Arc<AppConfig>,
}

impl Plugin<ApplicationStartup> for StaticWebStarter {
    fn build<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let assets_dir = manifest_dir.join("assets");
            let web_dist_dir = resolve_web_dist_dir(self.config.web_dist_dir.as_ref());
            let web_index = web_dist_dir.join("index.html");
            let web_application =
                ServeDir::new(web_dist_dir).fallback(ServeFile::new(web_index));
            let router = Router::new()
                .route("/", get(root_page))
                .route("/gateway", get(gateway_page))
                .route("/health", get(health))
                .nest_service("/assets", ServeDir::new(assets_dir))
                .nest_service("/app", web_application);
            target.merge_router(router);
            Ok(())
        })
    }
}

async fn root_page() -> Redirect {
    Redirect::temporary("/app/studio")
}

async fn gateway_page() -> Redirect {
    Redirect::temporary("/app/gateway")
}

async fn health() -> &'static str {
    "ok"
}

fn resolve_web_dist_dir(configured: Option<&PathBuf>) -> PathBuf {
    if let Some(path) = configured {
        return path.clone();
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest_dir.join("dist"),
        manifest_dir.join("../target/dx/az-aio-app/release/web/public"),
        manifest_dir.join("../target/dx/az-aio-app/debug/web/public"),
    ]
    .into_iter()
    .find(|path| path.join("index.html").is_file() && path.join("assets").is_dir())
    .unwrap_or_else(|| manifest_dir.join("dist"))
}
