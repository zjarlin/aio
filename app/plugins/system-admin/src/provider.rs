//! 系统后台聚合 Provider。
//!
//! Provider 只暴露后端 API 与启动生命周期；业务页面、菜单和交互由数据库中的
//! ProgramDefinition 提供。

use std::sync::Arc;

use az_plugin_core::{
    PluginActivation, PluginKind,
    plugin::{
        AdminMenuTree, AdminPluginContribution, AdminPluginProvider, BackendApiContribution,
        ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginRuntime,
        PluginDescriptor,
    },
};
use rudi::Singleton;

use crate::{
    catalog::{SYSTEM_DOMAIN_ID, SYSTEM_DOMAIN_LABEL, system_dashboard_view},
    routes::{SystemAdminApiState, system_admin_router},
    store::SystemAdminStore,
};

#[derive(Clone, Debug, Default)]
pub struct AdminProvider;

impl AdminProvider {
    pub fn contributions(&self) -> ContributionSet {
        let mut backend_apis = system_dashboard_view()
            .pages
            .into_iter()
            .flat_map(|page| {
                page.operations
                    .into_iter()
                    .map(move |operation| BackendApiContribution {
                        id: operation.id.to_owned(),
                        method: operation.method.to_owned(),
                        path: operation.path.to_owned(),
                        label: operation.label.to_owned(),
                        description: format!("{} / {}", page.label, operation.cli),
                        order: 1_000 + page.order,
                    })
            })
            .collect::<Vec<_>>();
        backend_apis.extend([
            backend_api(
                "system.api.status",
                "GET",
                "/api/system/status",
                "系统后台状态",
                "返回系统后台 API 与 PostgreSQL 状态。",
                900,
            ),
            backend_api(
                "system.api.dashboard",
                "GET",
                "/api/system/dashboard",
                "系统后台总览",
                "返回系统后台 API 能力摘要。",
                901,
            ),
            backend_api(
                "system.api.navigation",
                "GET",
                "/api/system/navigation",
                "系统上下文数据",
                "返回可供 ProgramGraph 数据源消费的系统上下文。",
                902,
            ),
            backend_api(
                "system.api.store.pages",
                "GET",
                "/api/system/store/pages",
                "系统资源快照",
                "返回系统后端资源快照。",
                903,
            ),
            backend_api(
                "system.api.store.operations",
                "GET",
                "/api/system/store/operations",
                "系统操作审计",
                "返回系统后端操作审计记录。",
                904,
            ),
            backend_api(
                "system.api.store.records",
                "GET",
                "/api/system/store/records",
                "系统数据快照",
                "按 o/s 分页返回系统数据记录。",
                905,
            ),
        ]);
        ContributionSet {
            backend_apis,
            ..ContributionSet::default()
        }
    }

    pub fn contribution_bundle(&self) -> AdminPluginContribution {
        AdminPluginContribution {
            menu: AdminMenuTree::default(),
            resources: Vec::new(),
            cli: Vec::new(),
            native: self.contributions(),
        }
    }
}

impl AdminPluginProvider for AdminProvider {
    fn admin_descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: SYSTEM_DOMAIN_ID.to_owned(),
            name: SYSTEM_DOMAIN_LABEL.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: "系统核心后端能力插件。".to_owned(),
            activation: PluginActivation::Eager,
            priority: 900,
            dependencies: Vec::new(),
            capabilities: vec!["axum-api".to_owned(), "postgresql-required".to_owned()],
            permissions: vec!["system-admin".to_owned()],
            kind: PluginKind::Native,
        }
    }

    fn admin_contribution(&self) -> anyhow::Result<AdminPluginContribution> {
        Ok(self.contribution_bundle())
    }

    fn admin_runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let state = system_admin_api_state(context.database_url, context.shared_db);
        Ok(NativePluginRuntime {
            router: system_admin_router(state),
            startup: None,
        })
    }
}

#[Singleton(name = "system-admin")]
pub fn system_admin_provider() -> DynAdminPluginProvider {
    Arc::new(AdminProvider)
}

pub fn system_contributions() -> ContributionSet {
    AdminProvider.contributions()
}

fn system_admin_api_state(
    database_url: Option<String>,
    shared_db: Option<az_plugin_core::Db>,
) -> SystemAdminApiState {
    if database_url.as_ref().is_none_or(|value| value.trim().is_empty()) {
        return SystemAdminApiState::degraded(database_url);
    }
    let store = shared_db.map(SystemAdminStore::from_shared);
    if let Some(store) = &store {
        sync_system_catalog(store.clone());
    }
    SystemAdminApiState::from_store(database_url, store)
}

fn sync_system_catalog(store: SystemAdminStore) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("system-admin Toasty startup degraded: {error:#}");
            return;
        }
    };
    if let Err(error) = runtime.block_on(store.sync_catalog_snapshot()) {
        eprintln!("system-admin Toasty startup degraded: {error:#}");
    }
}

fn backend_api(
    id: &str,
    method: &str,
    path: &str,
    label: &str,
    description: &str,
    order: i32,
) -> BackendApiContribution {
    BackendApiContribution {
        id: id.to_owned(),
        method: method.to_owned(),
        path: path.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_exports_backend_apis_without_ui_contracts() {
        let contributions = AdminProvider.contributions();
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == "/api/system/status")
        );
        let bundle = AdminProvider.contribution_bundle();
        assert!(bundle.menu.sections.is_empty());
        assert!(bundle.resources.is_empty());
    }
}
