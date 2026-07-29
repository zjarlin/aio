use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use axum::{
    Json, Router,
    extract::{Query, State},
    middleware,
    response::Redirect,
    routing::get,
};
use az_plugin_core::{
    Db, RecordStore,
    database::{collect_toasty_models, install_shared_db_singleton},
    http::{ApiResponse, ok_json},
    plugin::NativePluginContext,
};
use rudi::Context;
use studio::{
    ComponentCatalog, ComponentIndex, ProgramPatchAgent, PublishedApplication, WorkbenchBootstrap,
    capability::{CapabilityCatalog, DynCapabilityProvider},
    program_runtime::ProgramRuntime,
    program_store::ProgramStore,
};
use system_admin::{
    api_key_auth::{SystemApiKeyAuthState, optional_system_api_key_auth},
    store::SystemAdminStore,
};
use tower_http::services::ServeDir;

use crate::{
    config::AppConfig,
    migration,
    plugin_host::{self, HostSnapshot},
};

pub fn run() -> Result<()> {
    enable_plugin_providers();

    let mut di = Context::auto_register();
    let components =
        Arc::new(ComponentIndex::from_context(&mut di).context("收集 Studio 组件失败")?);
    let program_components = components.program_catalog();
    di.insert_singleton(components);

    let config = AppConfig::from_env().context("加载 AIO 应用配置失败")?;
    let port = config.port;
    let database_url = config.database_url;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建 AIO runtime 失败")?;
    let models = collect_toasty_models(&mut di);
    let shared_db = runtime
        .block_on(async {
            if let Some(database_url) = database_url.as_deref() {
                migration::run(database_url).await?;
            }
            install_shared_db_singleton(&mut di, database_url.as_deref(), models).await
        })
        .context("初始化 AIO PostgreSQL 失败")?;

    let native_context = NativePluginContext {
        api_base_url: String::new(),
        config_dir: PathBuf::from("."),
        data_dir: PathBuf::from("."),
        database_url: database_url.clone(),
        shared_db: shared_db.clone(),
    };
    let snapshot = {
        let _runtime_guard = runtime.enter();
        plugin_host::load_native_snapshot(native_context, &mut di)
    };
    let capabilities = CapabilityCatalog::new(di.resolve_by_type::<DynCapabilityProvider>())?;

    runtime.block_on(run_server(
        snapshot,
        port,
        database_url,
        shared_db,
        program_components,
        capabilities,
    ))
}

async fn run_server(
    snapshot: HostSnapshot,
    port: u16,
    database_url: Option<String>,
    shared_db: Option<Db>,
    components: ComponentCatalog,
    capabilities: CapabilityCatalog,
) -> Result<()> {
    let record_store = shared_db
        .as_ref()
        .map(|database| RecordStore::from_shared_db(database.shared_handle(), database.pg_pool()));
    let program_runtime = match (database_url.as_deref(), record_store) {
        (Some(database_url), Some(record_store)) => {
            let store = ProgramStore::connect(database_url).await?;
            let runtime = ProgramRuntime::new(store, record_store, components, capabilities);
            runtime.restore_active_images().await?;
            runtime.publish_unactivated_applications().await?;
            runtime.spawn_postgres_listener(database_url).await?;
            Some(runtime)
        }
        _ => None,
    };
    let page_state = PageState {
        program_runtime: program_runtime.clone(),
    };
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
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = manifest_dir.join("assets");
    let web_dist_dir = web_dist_dir();

    let page_router = Router::new()
        .route("/", get(root_page))
        .route("/gateway", get(root_page))
        .route("/api/bootstrap", get(bootstrap))
        .route("/health", get(health))
        .nest_service("/assets", ServeDir::new(assets_dir))
        .nest_service("/app", ServeDir::new(web_dist_dir))
        .with_state(page_state)
        .merge(studio::studio_http::router(
            studio::studio_http::StudioState::new(program_runtime, ProgramPatchAgent::from_env()?),
        ));

    let app = page_router.merge(native_router.with_state(()));
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    println!("AIO listening on http://{address}");
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn root_page(Query(query): Query<HashMap<String, String>>) -> Redirect {
    let route = query
        .get("route")
        .cloned()
        .unwrap_or_else(|| "/studio".to_owned());
    Redirect::temporary(&format!("/app/?route={}", urlencoding::encode(&route)))
}

async fn bootstrap(State(state): State<PageState>) -> Json<ApiResponse<WorkbenchBootstrap>> {
    ok_json(state.workbench_bootstrap().await)
}

#[derive(Clone)]
struct PageState {
    program_runtime: Option<ProgramRuntime>,
}

impl PageState {
    async fn workbench_bootstrap(&self) -> WorkbenchBootstrap {
        let mut bootstrap = WorkbenchBootstrap::default();
        let Some(runtime) = &self.program_runtime else {
            return bootstrap;
        };
        bootstrap.applications = runtime
            .active_images()
            .await
            .into_iter()
            .map(|(application_id, runtime_image)| {
                let image = runtime_image.image();
                PublishedApplication {
                    application_id,
                    program_id: image.application_id,
                    name: image.name.clone(),
                    title: image.title.clone(),
                    revision_id: image.revision_id.clone(),
                    content_hash: image.content_hash.clone(),
                    menus: image.menus.clone(),
                    routes: image.routes.clone(),
                }
            })
            .collect();
        bootstrap.default_route = bootstrap
            .applications
            .iter()
            .flat_map(|application| &application.routes)
            .map(|route| route.path.clone())
            .next()
            .unwrap_or_else(|| "/studio".to_owned());
        bootstrap
    }
}

fn web_dist_dir() -> PathBuf {
    if let Ok(path) = std::env::var("AZ_AIO_WEB_DIST") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest_dir.join("dist"),
        manifest_dir.join("../target/dx/az-aio-app/release/web/public"),
        manifest_dir.join("../target/dx/az-aio-app/debug/web/public"),
    ]
    .into_iter()
    .find(|path| path.join("wasm/az-aio-app.js").exists())
    .unwrap_or_else(|| manifest_dir.join("dist"))
}

async fn health() -> &'static str {
    "ok"
}

fn enable_plugin_providers() {
    studio::enable();
    system_admin::enable();
    algorithm_center::enable();
    asset_hub::enable();
    config_center::enable();
    drive_center::enable();
    edge_gateway::enable();
    iot_center::enable();
    software_center::enable();
    ssh_plugin::enable();
    az_linux::enable();
}
