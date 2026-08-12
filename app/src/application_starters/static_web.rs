//! AIO 健康检查、跳转和静态 Web 路由启动器。

use std::{path::PathBuf, sync::Arc};

use axum::{Router, response::Redirect, routing::get};
use az_plugin_core::{DynPlugin, Plugin, PluginFuture};
use tower_http::services::{ServeDir, ServeFile};

use crate::{application_startup::ApplicationStartup, config::AppConfig};

/// 安装应用入口、健康检查和 Dioxus 静态产物路由。
pub struct StaticWebStarter {
    web_dist_dir: Option<PathBuf>,
}

impl Plugin<ApplicationStartup> for StaticWebStarter {
    fn order(&self) -> i32 {
        60
    }

    fn install<'a>(&'a self, target: &'a mut ApplicationStartup) -> PluginFuture<'a> {
        Box::pin(async move {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let assets_dir = manifest_dir.join("assets");
            let web_dist_dir = resolve_web_dist_dir(self.web_dist_dir.as_ref());
            let web_index = web_dist_dir.join("index.html");
            let web_assets_router = Router::new()
                .route_service("/", ServeFile::new(web_index.clone()))
                .route_service("/{*asset}", ServeDir::new(web_dist_dir.join("assets")));
            let router = Router::new()
                .route("/", get(root_page))
                .route("/gateway", get(gateway_page))
                .route("/health", get(health))
                .nest_service("/assets", ServeDir::new(assets_dir))
                .nest("/app/assets", web_assets_router)
                .route_service("/app", ServeFile::new(web_index.clone()))
                .route_service("/app/{*route}", ServeFile::new(web_index));
            target.merge_router(router);
            Ok(())
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<StaticWebStarter>())]
pub fn static_web_starter(config: AppConfig) -> DynPlugin<ApplicationStartup> {
    Arc::new(StaticWebStarter {
        web_dist_dir: config.web_dist_dir,
    })
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
