#![forbid(unsafe_code)]

use anyhow::{Context as _, Result};
use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    middleware,
    response::Html,
    routing::get,
};
use az_aio_platform::{
    core::{config::AppConfig, db},
    plugin::host,
    system::{
        api_key_auth::{SystemApiKeyAuthState, optional_system_api_key_auth},
        store::SystemAdminStore,
    },
};
use az_remote_ui::ComponentIndex;
use rudi::Context;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;

mod migration;
mod remote_ui;
mod shell;

fn main() -> Result<()> {
    enable_plugin_providers();

    let mut di = Context::auto_register();
    let remote_ui_components =
        Arc::new(ComponentIndex::from_context(&mut di).context("收集 Remote UI 组件失败")?);
    let config = di.resolve::<az_aio_platform::core::config::ConfigCenterConfig>();

    let port = config.port();
    let database_url = config.database_url();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建 AIO web runtime 失败")?;
    let toasty_models = db::collect_toasty_models(&mut di);
    let shared_db = runtime
        .block_on(async {
            if let Some(database_url) = database_url.as_deref() {
                migration::run(database_url).await?;
            }
            db::install_shared_db_singleton(&mut di, database_url.as_deref(), toasty_models).await
        })
        .context("初始化 AIO PostgreSQL 失败")?;

    let native_context = az_aio_platform::plugin::contract::NativePluginContext {
        api_base_url: String::new(),
        config_dir: std::path::PathBuf::from("."),
        data_dir: std::path::PathBuf::from("."),
        database_url: database_url.clone(),
        shared_db: shared_db.clone(),
    };

    let snapshot = host::load_native_snapshot(native_context, &mut di);

    let remote_ui_store = shared_db
        .as_ref()
        .map(|db| az_engine::EngineStore::from_shared_db(db.shared_handle()));
    let remote_ui_runtime = remote_ui::RemoteUiRuntime::new(remote_ui_components, remote_ui_store);

    runtime.block_on(run_web_server(
        snapshot,
        port,
        database_url,
        shared_db,
        remote_ui_runtime,
    ))
}

async fn run_web_server(
    snapshot: az_aio_platform::plugin::host::HostSnapshot,
    port: u16,
    database_url: Option<String>,
    shared_db: Option<db::Db>,
    remote_ui_runtime: remote_ui::RemoteUiRuntime,
) -> Result<()> {
    let api_key_auth_state = if database_url
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        SystemApiKeyAuthState::from_store(shared_db.map(SystemAdminStore::from_shared))
    } else {
        SystemApiKeyAuthState::degraded()
    };
    let native_router = snapshot
        .native_router
        .clone()
        .layer(middleware::from_fn_with_state(
            api_key_auth_state,
            optional_system_api_key_auth,
        ));
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let client_dist_dir = client_dist_dir();

    let page_snapshot = snapshot.clone();
    let page_router = Router::new()
        .route("/", get(root_page))
        .route("/gateway", get(root_page))
        .route("/remote-ui", get(remote_ui::page))
        .route("/api/client/bootstrap", get(client_bootstrap))
        .route(
            "/api/remote-ui/pages/{page_key}/stream",
            get(remote_ui::page_stream),
        )
        .route(
            "/api/remote-ui/components",
            get(remote_ui::component_catalog),
        )
        .route("/health", get(health))
        .nest_service("/assets", ServeDir::new(&assets_dir))
        .nest_service("/client", ServeDir::new(&client_dist_dir))
        .nest_service("/wasm", ServeDir::new(client_dist_dir.join("wasm")))
        .layer(Extension(remote_ui_runtime))
        .with_state(page_snapshot);

    let app = page_router.merge(native_router.with_state(()));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("AIO web workbench listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn root_page(
    State(snapshot): State<az_aio_platform::plugin::host::HostSnapshot>,
    Query(query): Query<HashMap<String, String>>,
) -> Html<String> {
    let active_route = query
        .get("route")
        .cloned()
        .unwrap_or_else(|| "/system/account/api-keys".to_string());
    let route_for_error = active_route.clone();
    let html = tokio::task::spawn_blocking(move || {
        shell::render_workbench_page(&snapshot, &active_route, "")
    })
    .await
    .unwrap_or_else(|error| shell::render_ssr_error_page(&route_for_error, &error.to_string()));
    Html(html)
}

async fn client_bootstrap(
    State(snapshot): State<az_aio_platform::plugin::host::HostSnapshot>,
) -> Json<az_aio_platform::plugin::contract::ClientBootstrapPayload> {
    Json(host::client_bootstrap_payload(
        &snapshot,
        default_route(&snapshot),
        "",
    ))
}

fn default_route(snapshot: &az_aio_platform::plugin::host::HostSnapshot) -> String {
    snapshot
        .pages
        .first()
        .map(|page| page.route.clone())
        .unwrap_or_else(|| "/system/account/api-keys".to_string())
}

fn client_dist_dir() -> PathBuf {
    if let Ok(path) = std::env::var("AZ_AIO_CLIENT_DIST") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest_dir.join("../client/dist"),
        manifest_dir.join("../target/dx/az-aio-client/release/web/public"),
        manifest_dir.join("../target/dx/az-aio-client/debug/web/public"),
    ]
    .into_iter()
    .find(|path| path.join("wasm/az-aio-client.js").exists())
    .unwrap_or_else(|| manifest_dir.join("../client/dist"))
}

async fn health() -> &'static str {
    "ok"
}

fn enable_plugin_providers() {
    az_aio_platform::enable();
    algorithm_center::enable();
    asset_hub::enable();
    codegen::enable();
    config_center::enable();
    drive_center::enable();
    edge_gateway::enable();
    iot_center::enable();
    lowcode::enable();
    software_center::enable();
    ssh_plugin::enable();
    az_linux::enable();
}

#[cfg(test)]
mod tests {
    use az_aio_platform::plugin::contract::{DynAdminPluginProvider, NativePluginContext};

    use super::*;

    #[test]
    fn rudi_collects_all_admin_plugin_providers() {
        enable_plugin_providers();

        let mut di = Context::auto_register();
        let mut plugin_ids = di
            .resolve_by_type::<DynAdminPluginProvider>()
            .into_iter()
            .map(|plugin| plugin.admin_descriptor().id)
            .collect::<Vec<_>>();
        plugin_ids.sort();

        assert_eq!(
            plugin_ids,
            [
                "admin-scenes",
                "algorithm-center",
                "asset-hub",
                "codegen",
                "config-center",
                "drive-center",
                "edge-gateway",
                "iot-center",
                "linux",
                "lowcode",
                "software-center",
                "ssh",
                "system",
            ]
        );
    }

    #[test]
    fn rudi_menu_reserves_admin_knowledge_base_and_gateway_scenes() {
        enable_plugin_providers();

        let mut di = Context::auto_register();
        let snapshot = host::load_native_snapshot(NativePluginContext::default(), &mut di);
        let labels = snapshot
            .admin_menu_tree
            .sections
            .iter()
            .map(|section| section.label.as_str())
            .collect::<Vec<_>>();
        let knowledge = snapshot
            .admin_menu_tree
            .sections
            .iter()
            .find(|section| section.label == "知识库");
        let gateway = snapshot
            .admin_menu_tree
            .sections
            .iter()
            .find(|section| section.label == "智能网关");
        let system = snapshot
            .admin_menu_tree
            .sections
            .iter()
            .find(|section| section.label == "管理后台");
        let server_operations = snapshot
            .admin_menu_tree
            .sections
            .iter()
            .find(|section| section.label == "服务器运维");

        assert!(labels.contains(&"管理后台"));
        assert!(labels.contains(&"知识库"));
        assert!(labels.contains(&"智能网关"));
        assert!(labels.contains(&"服务器运维"));
        assert_eq!(
            knowledge.map(|section| section.default_href.as_str()),
            Some("/assets")
        );
        assert!(
            knowledge
                .map(|section| section
                    .menus
                    .iter()
                    .any(|node| node.href == "/assets" && node.label == "资产中心"))
                .unwrap_or(false)
        );
        // The migrated pages must keep SSR renderer IDs aligned with client route entries.
        assert!(
            system
                .map(|section| section
                    .menus
                    .iter()
                    .any(|node| menu_node_contains_href(node, "/config")))
                .unwrap_or(false)
        );
        assert!(
            gateway
                .map(|section| section.menus.iter().any(|node| node.label == "算法中心"))
                .unwrap_or(false)
        );
        assert_eq!(
            server_operations.map(|section| section.default_href.as_str()),
            Some("/ssh?view=overview")
        );
        assert!(
            server_operations
                .map(|section| section
                    .menus
                    .iter()
                    .any(|node| menu_node_contains_href(node, "/ssh")))
                .unwrap_or(false)
        );
    }

    #[test]
    fn rudi_collects_all_toasty_models() {
        enable_plugin_providers();

        let mut di = Context::auto_register();
        let models = db::collect_toasty_models(&mut di);
        let mut model_names = models
            .iter()
            .map(|model| model.name().upper_camel_case())
            .collect::<Vec<_>>();
        model_names.sort();

        assert_eq!(
            model_names,
            [
                "AssetRecord",
                "ConfigEntry",
                "DataRecord",
                "DictionaryItemRecord",
                "DictionaryTypeRecord",
                "DriveTask",
                "EdgeApiTokenRecord",
                "EdgeUsageRecordRow",
                "GatewayFlow",
                "GatewayRouteDefinition",
                "HookDefinition",
                "MetaField",
                "MetaModel",
                "OperationDefinition",
                "OperationRevision",
                "OperationRun",
                "PageRecord",
                "SoftwarePackageRecord",
                "SystemApiKeyRecord",
                "SystemDataRecord",
                "SystemOperationRecord",
                "SystemPageRecord",
            ]
        );
    }

    #[test]
    fn migrated_plugins_expose_matching_client_pages() {
        enable_plugin_providers();

        let mut di = Context::auto_register();
        let snapshot = host::load_native_snapshot(NativePluginContext::default(), &mut di);
        let client_routes = snapshot
            .client_pages
            .iter()
            .map(|page| (page.route.as_str(), page.renderer_id.as_str()))
            .collect::<Vec<_>>();

        assert!(
            [
                ("/assets", "asset-hub.page"),
                ("/drive", "drive-center.page"),
                ("/software", "software-center.page"),
            ]
            .iter()
            .all(|route| client_routes.contains(route))
        );
    }

    fn menu_node_contains_href(
        node: &az_aio_platform::plugin::contract::AdminMenuNode,
        href: &str,
    ) -> bool {
        node.href == href
            || node
                .children
                .iter()
                .any(|child| menu_node_contains_href(child, href))
    }
}
