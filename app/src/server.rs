use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context as _, Result};
use axum::{Json, Router, extract::State, response::Redirect, routing::get};
use az_plugin_core::{
    Db, RecordStore,
    database::{collect_toasty_models, install_shared_db_singleton},
    http::{ApiResponse, ok_json},
    plugin::NativePluginContext,
};
use rudi::Context;
use studio::{
    AdminWorkbenchState, CompiledArtifactWriter, ConventionContractManager,
    ConventionEndpointIndex, NativeContractCatalog, ProgramPatchAgent, PublishedProgram,
    WorkbenchBootstrap,
    capability::{CapabilityCatalog, DynCapabilityProvider},
    program_runtime::ProgramRuntime,
    program_store::ProgramStore,
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    config::AppConfig,
    migration,
    plugin_host::{self, HostSnapshot},
};

pub fn run() -> Result<()> {
    enable_plugin_providers();

    let mut di = Context::auto_register();
    let config = AppConfig::load().context("加载 AIO 应用配置失败")?;
    let port = config.port;
    let database_url = config.database_url;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建 AIO runtime 失败")?;
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind(address))
        .with_context(|| format!("绑定 AIO 监听地址失败: {address}"))?;
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
    let admin = studio::resolve_admin_workbench_state(&mut di);
    let capabilities = CapabilityCatalog::new(di.resolve_by_type::<DynCapabilityProvider>())?;
    let convention_endpoints = ConventionEndpointIndex::from_context(&mut di)?;

    runtime.block_on(run_server(
        snapshot,
        listener,
        database_url,
        shared_db,
        capabilities,
        admin,
        convention_endpoints,
    ))
}

async fn run_server(
    snapshot: HostSnapshot,
    listener: tokio::net::TcpListener,
    database_url: Option<String>,
    shared_db: Option<Db>,
    capabilities: CapabilityCatalog,
    admin: Option<AdminWorkbenchState>,
    convention_endpoints: ConventionEndpointIndex,
) -> Result<()> {
    let native_contracts = NativeContractCatalog::from_contributions(
        snapshot
            .plugin_contributions
            .iter()
            .map(|record| (record.plugin_id.as_str(), &record.contributions)),
    )?;
    let record_store = shared_db
        .as_ref()
        .map(|database| RecordStore::from_shared_db(database.shared_handle(), database.pg_pool()));
    let program_runtime = match (database_url.as_deref(), shared_db.as_ref(), record_store) {
        (Some(database_url), Some(database), Some(record_store)) => {
            let store = ProgramStore::from_pool(database.pg_pool());
            let runtime = ProgramRuntime::new(
                store,
                record_store,
                capabilities,
                CompiledArtifactWriter::workspace_target(),
            );
            let _native_report = runtime
                .store()
                .reconcile_native_contracts(&native_contracts)
                .await
                .context("同步插件 API 元数据到 Studio 失败")?;
            runtime.restore_active_image().await?;
            if let Err(error) = runtime.publish_draft_if_changed("migration").await {
                if runtime.active_image().await.is_none() {
                    return Err(error).context("发布原生接口元数据 Revision 失败");
                }
                eprintln!("发布最新 Studio Draft 失败，继续使用活动 Revision: {error:#}");
            }
            runtime.spawn_postgres_listener(database_url).await?;
            Some(runtime)
        }
        _ => None,
    };
    let page_state = PageState {
        program_runtime: program_runtime.clone(),
        admin,
    };
    let convention_contracts = ConventionContractManager::workspace_app();
    let convention_router = match &program_runtime {
        Some(runtime) => {
            let draft = runtime.store().draft().await?;
            convention_contracts
                .reconcile(&draft.definition)
                .context("同步 Studio 约定接口文件失败")?;
            match runtime.active_image().await {
                Some(image) => convention_endpoints.router(image.image())?,
                None => Router::new(),
            }
        }
        None => Router::new(),
    };
    let native_router = snapshot.native_router.clone();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = manifest_dir.join("assets");
    let web_dist_dir = web_dist_dir();
    let web_index = web_dist_dir.join("index.html");
    let web_assets_router = Router::<PageState>::new()
        .route_service("/", ServeFile::new(web_index.clone()))
        .route_service("/{*asset}", ServeDir::new(web_dist_dir.join("assets")));

    let page_router = Router::new()
        .route("/", get(root_page))
        .route("/gateway", get(gateway_page))
        .route("/api/bootstrap", get(bootstrap))
        .route("/health", get(health))
        .nest_service("/assets", ServeDir::new(assets_dir))
        .nest("/app/assets", web_assets_router)
        .route_service("/app", ServeFile::new(web_index.clone()))
        .route_service("/app/{*route}", ServeFile::new(web_index))
        .with_state(page_state)
        .merge(studio::studio_http::router(
            studio::studio_http::StudioState::new(
                program_runtime,
                ProgramPatchAgent::from_env()?,
                studio::FormStateExtractor::from_env()?,
                convention_contracts,
            ),
        ));

    let app = page_router
        .merge(native_router.with_state(()))
        .merge(convention_router);
    let address = listener.local_addr().context("读取 AIO 监听地址失败")?;
    println!("AIO listening on http://{address}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn root_page() -> Redirect {
    Redirect::temporary("/app/studio")
}

async fn gateway_page() -> Redirect {
    Redirect::temporary("/app/gateway")
}

async fn bootstrap(State(state): State<PageState>) -> Json<ApiResponse<WorkbenchBootstrap>> {
    ok_json(state.workbench_bootstrap().await)
}

#[derive(Clone)]
struct PageState {
    program_runtime: Option<ProgramRuntime>,
    admin: Option<AdminWorkbenchState>,
}

impl PageState {
    async fn workbench_bootstrap(&self) -> WorkbenchBootstrap {
        let mut bootstrap = WorkbenchBootstrap::default();
        bootstrap.admin = self.admin.clone();
        let Some(runtime) = &self.program_runtime else {
            return bootstrap;
        };
        bootstrap.program = runtime.active_image().await.map(|runtime_image| {
            let image = runtime_image.image();
            PublishedProgram {
                id: image.program_id,
                name: image.name.clone(),
                title: image.title.clone(),
                revision_id: image.revision_id.clone(),
                content_hash: image.content_hash.clone(),
                menus: image.menus.clone(),
                routes: image.routes.clone(),
            }
        });
        bootstrap.default_route = bootstrap
            .program
            .as_ref()
            .and_then(PublishedProgram::default_route)
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
    .find(|path| path.join("index.html").is_file() && path.join("assets").is_dir())
    .unwrap_or_else(|| manifest_dir.join("dist"))
}

async fn health() -> &'static str {
    "ok"
}

fn enable_plugin_providers() {
    crate::contracts::enable();
    studio::enable();
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
