use std::{
    collections::{BTreeSet, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use az_plugin_core::{CatalogItemKind, CatalogSource, PluginState};
#[cfg(test)]
use az_plugin_core::{PluginActivation, PluginKind};
use az_plugin_core::{
    http::with_global_api_error_layer,
    plugin::{
        AdminCliContribution, AdminMenuTree, AdminResourceContract, BackendApiContribution,
        CatalogItemContribution, ContributionSet, DynAdminPluginProvider, NativePluginContext,
        PluginDescriptor, merge_menu_tree,
    },
};
use serde::{Deserialize, Serialize};

const PLUGIN_STATE_FILE: &str = "plugin-state.json";

pub fn load_native_snapshot(context: NativePluginContext, di: &mut rudi::Context) -> HostSnapshot {
    let enablement = load_plugin_enablement();
    NativePluginHost::from_context(context, di).load_snapshot_with_enablement(&enablement)
}

pub struct NativePluginHost {
    plugins: Vec<DynAdminPluginProvider>,
    context: NativePluginContext,
}

impl NativePluginHost {
    #[cfg(test)]
    pub fn new(context: NativePluginContext) -> Self {
        Self {
            plugins: Vec::new(),
            context,
        }
    }

    pub fn from_context(mut context: NativePluginContext, di: &mut rudi::Context) -> Self {
        if context.shared_db.is_none() {
            context.shared_db = di.resolve_option::<az_plugin_core::Db>();
        }
        let mut plugins = di.resolve_by_type::<DynAdminPluginProvider>();
        plugins.sort_by(|left, right| left.admin_descriptor().id.cmp(&right.admin_descriptor().id));
        Self { plugins, context }
    }

    #[cfg(test)]
    pub fn with_plugin(mut self, plugin: DynAdminPluginProvider) -> Self {
        self.plugins.push(plugin);
        self
    }

    #[cfg(test)]
    pub fn load_snapshot(self) -> HostSnapshot {
        self.load_snapshot_with_enablement(&PluginEnablementStore::default())
    }

    pub fn load_snapshot_with_enablement(self, enablement: &PluginEnablementStore) -> HostSnapshot {
        let mut snapshot = HostSnapshot::default();
        let mut seen_ids = HashSet::new();
        let mut seen_routes = HashSet::new();

        for plugin in self.plugins {
            let descriptor = plugin.admin_descriptor();
            if !seen_ids.insert(descriptor.id.clone()) {
                snapshot.plugins.push(failed_record(
                    descriptor.clone(),
                    format!("duplicate plugin ID: {}", descriptor.id),
                ));
                continue;
            }
            if !enablement.plugin_enabled(&descriptor.id) {
                snapshot.plugins.push(disabled_record(descriptor));
                continue;
            }

            let admin_contribution = match plugin.admin_contribution() {
                Ok(contribution) => headless_admin_contribution(contribution),
                Err(error) => {
                    snapshot.plugins.push(failed_record(
                        descriptor.clone(),
                        format!(
                            "plugin `{}` failed during contributions: {}",
                            descriptor.id, error
                        ),
                    ));
                    continue;
                }
            };
            let contributions = admin_contribution.native.clone();

            let runtime = match plugin.admin_runtime(self.context.clone()) {
                Ok(r) => r,
                Err(error) => {
                    merge_snapshot_admin_contribution(&mut snapshot, admin_contribution);
                    merge_snapshot_contributions(&mut snapshot, contributions.clone());
                    snapshot
                        .plugin_contributions
                        .push(PluginContributionRecord {
                            plugin_id: descriptor.id.clone(),
                            contributions: contributions.clone(),
                        });
                    snapshot.plugins.push(failed_record(
                        descriptor.clone(),
                        format!(
                            "plugin `{}` failed during runtime: {}",
                            descriptor.id, error
                        ),
                    ));
                    continue;
                }
            };

            if let Some(startup) = runtime.startup
                && let Err(error) = startup(self.context.clone())
            {
                snapshot.plugins.push(failed_record(
                    descriptor.clone(),
                    format!(
                        "plugin `{}` failed during startup: {}",
                        descriptor.id, error
                    ),
                ));
                continue;
            }

            if let Some((method, path)) =
                first_duplicate_backend_route(&contributions.backend_apis, &mut seen_routes)
            {
                snapshot.plugins.push(failed_record(
                    descriptor.clone(),
                    format!("backend route duplicated: {method} {path}"),
                ));
                continue;
            }

            merge_snapshot_admin_contribution(&mut snapshot, admin_contribution);
            snapshot
                .plugin_contributions
                .push(PluginContributionRecord {
                    plugin_id: descriptor.id.clone(),
                    contributions: contributions.clone(),
                });
            merge_snapshot_contributions(&mut snapshot, contributions);
            snapshot.native_router = snapshot.native_router.merge(runtime.router);
            snapshot.plugins.push(PluginRuntimeRecord {
                descriptor,
                state: PluginState::Active,
                error: None,
            });
        }

        sort_snapshot(&mut snapshot);
        snapshot.native_router = with_global_api_error_layer(snapshot.native_router);
        snapshot
    }
}

pub fn load_plugin_enablement() -> PluginEnablementStore {
    read_plugin_enablement_store(&plugin_enablement_store_path()).unwrap_or_default()
}

pub fn plugin_enablement_store_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        })
        .join("addzero")
        .join("az-aio")
        .join(PLUGIN_STATE_FILE)
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginEnablementStore {
    #[serde(default)]
    pub disabled_plugin_ids: BTreeSet<String>,
}

impl PluginEnablementStore {
    pub fn plugin_enabled(&self, plugin_id: &str) -> bool {
        !self.disabled_plugin_ids.contains(plugin_id)
    }
}

#[derive(Clone, Default)]
pub struct HostSnapshot {
    pub admin_menu_tree: AdminMenuTree,
    pub admin_resources: Vec<AdminResourceContract>,
    pub admin_cli: Vec<AdminCliContribution>,
    pub backend_apis: Vec<BackendApiContribution>,
    pub catalog_items: Vec<CatalogItemContribution>,
    pub plugin_contributions: Vec<PluginContributionRecord>,
    pub plugins: Vec<PluginRuntimeRecord>,
    pub native_router: axum::Router,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginRuntimeRecord {
    pub descriptor: PluginDescriptor,
    pub state: PluginState,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginContributionRecord {
    pub plugin_id: String,
    pub contributions: ContributionSet,
}

// ── internal helpers ────────────────────────────────────────────

fn first_duplicate_backend_route(
    apis: &[BackendApiContribution],
    seen_routes: &mut HashSet<(String, String)>,
) -> Option<(String, String)> {
    for api in apis {
        let key = (api.method.clone(), api.path.clone());
        if !seen_routes.insert(key.clone()) {
            return Some(key);
        }
    }
    None
}

fn headless_admin_contribution(
    mut contribution: az_plugin_core::plugin::AdminPluginContribution,
) -> az_plugin_core::plugin::AdminPluginContribution {
    contribution.menu = AdminMenuTree::default();
    contribution.resources.clear();
    contribution
}

fn merge_snapshot_contributions(snapshot: &mut HostSnapshot, contributions: ContributionSet) {
    snapshot.backend_apis.extend(contributions.backend_apis);
    for provider in contributions.catalog_providers {
        snapshot.catalog_items.extend(provider.items);
    }
}

fn merge_snapshot_admin_contribution(
    snapshot: &mut HostSnapshot,
    contribution: az_plugin_core::plugin::AdminPluginContribution,
) {
    merge_menu_tree(&mut snapshot.admin_menu_tree, contribution.menu);
    snapshot.admin_resources.extend(contribution.resources);
    snapshot.admin_cli.extend(contribution.cli);
}

fn sort_snapshot(snapshot: &mut HostSnapshot) {
    let mut contributions = ContributionSet {
        backend_apis: std::mem::take(&mut snapshot.backend_apis),
        catalog_providers: Vec::new(),
    };
    sort_contributions(&mut contributions);
    snapshot.backend_apis = contributions.backend_apis;
    snapshot
        .admin_resources
        .sort_by(|left, right| left.route.cmp(&right.route).then(left.id.cmp(&right.id)));
    snapshot.admin_cli.sort_by(|left, right| {
        left.command
            .cmp(&right.command)
            .then(left.id.cmp(&right.id))
    });
    snapshot
        .catalog_items
        .extend(plugin_catalog_items(&snapshot.plugins));
    snapshot.catalog_items.sort_by(|left, right| {
        left.kind
            .label()
            .cmp(right.kind.label())
            .then(left.section.cmp(&right.section))
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
    snapshot
        .plugin_contributions
        .sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    snapshot
        .plugins
        .sort_by(|left, right| left.descriptor.id.cmp(&right.descriptor.id));
}

fn failed_record(descriptor: PluginDescriptor, error: String) -> PluginRuntimeRecord {
    PluginRuntimeRecord {
        descriptor,
        state: PluginState::Failed,
        error: Some(error),
    }
}

fn disabled_record(descriptor: PluginDescriptor) -> PluginRuntimeRecord {
    PluginRuntimeRecord {
        descriptor,
        state: PluginState::Disabled,
        error: None,
    }
}

fn sort_contributions(contributions: &mut ContributionSet) {
    contributions.backend_apis.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.method.cmp(&right.method))
            .then(left.order.cmp(&right.order))
            .then(left.id.cmp(&right.id))
    });
    contributions
        .catalog_providers
        .sort_by(|left, right| left.order.cmp(&right.order).then(left.id.cmp(&right.id)));
}

fn plugin_catalog_items(records: &[PluginRuntimeRecord]) -> Vec<CatalogItemContribution> {
    records
        .iter()
        .map(|record| CatalogItemContribution {
            id: record.descriptor.id.clone(),
            name: record.descriptor.name.clone(),
            description: record
                .error
                .clone()
                .unwrap_or_else(|| record.descriptor.description.clone()),
            section: "插件".to_string(),
            icon: "◇".to_string(),
            accent_class: match record.state {
                PluginState::Failed => "plugin-icon--git",
                _ => "plugin-icon--automation",
            }
            .to_string(),
            kind: CatalogItemKind::Plugin,
            source: CatalogSource::Bundled,
            installed: record.state == PluginState::Active || record.state == PluginState::Loaded,
            tags: Vec::new(),
            permissions: record.descriptor.permissions.clone(),
            path: None,
        })
        .collect()
}

#[cfg(test)]
pub fn descriptor(
    id: &str,
    name: &str,
    description: &str,
    priority: i32,
    dependencies: Vec<az_plugin_core::plugin::PluginDependency>,
    capabilities: Vec<&str>,
) -> PluginDescriptor {
    PluginDescriptor {
        id: id.to_string(),
        name: name.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: description.to_string(),
        activation: PluginActivation::Eager,
        priority,
        dependencies,
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
        permissions: Vec::new(),
        kind: PluginKind::Native,
    }
}

// ── plugin enablement persistence ─────────────────────────────────

fn read_plugin_enablement_store(path: &Path) -> io::Result<PluginEnablementStore> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid plugin state file: {error}"),
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(PluginEnablementStore::default())
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use az_plugin_core::AdminMenuNodeKind;

    use az_plugin_core::plugin::{
        AdminMenuNode, AdminMenuSection, AdminPluginContribution, AdminPluginProvider,
        BackendApiContribution, NativePluginRuntime,
    };

    use super::*;

    #[derive(Clone)]
    struct TestProvider {
        id: &'static str,
        route: &'static str,
        api_path: &'static str,
    }

    impl AdminPluginProvider for TestProvider {
        fn admin_descriptor(&self) -> PluginDescriptor {
            descriptor(self.id, self.id, "测试插件", 10, Vec::new(), vec!["test"])
        }

        fn admin_contribution(&self) -> anyhow::Result<AdminPluginContribution> {
            Ok(AdminPluginContribution {
                menu: AdminMenuTree {
                    sections: vec![AdminMenuSection {
                        domain_id: "test".to_string(),
                        label: "测试".to_string(),
                        default_href: self.route.to_string(),
                        order: 10,
                        menus: vec![AdminMenuNode {
                            id: format!("{}.nav", self.id),
                            kind: AdminMenuNodeKind::Page,
                            label: self.id.to_string(),
                            href: self.route.to_string(),
                            icon: "T".to_string(),
                            order: 10,
                            active_patterns: vec![self.route.to_string()],
                            permissions_any_of: Vec::new(),
                            children: Vec::new(),
                        }],
                    }],
                },
                resources: Vec::new(),
                cli: Vec::new(),
                native: ContributionSet {
                    backend_apis: vec![BackendApiContribution {
                        id: format!("{}.api", self.id),
                        method: "GET".to_string(),
                        path: self.api_path.to_string(),
                        label: "测试 API".to_string(),
                        description: "测试 API".to_string(),
                        order: 10,
                    }],
                    ..ContributionSet::default()
                },
            })
        }

        fn admin_runtime(
            &self,
            _context: NativePluginContext,
        ) -> anyhow::Result<NativePluginRuntime> {
            Ok(NativePluginRuntime::default())
        }
    }

    #[test]
    fn empty_host_does_not_create_business_menu() {
        let snapshot = NativePluginHost::new(NativePluginContext::default()).load_snapshot();

        assert!(snapshot.admin_menu_tree.sections.is_empty());
    }

    #[test]
    fn host_discards_business_menu_and_keeps_backend_api() {
        let provider = Arc::new(TestProvider {
            id: "demo",
            route: "/demo",
            api_path: "/api/demo",
        });
        let snapshot = NativePluginHost::new(NativePluginContext::default())
            .with_plugin(provider)
            .load_snapshot();

        assert!(snapshot.admin_menu_tree.sections.is_empty());
        assert_eq!(snapshot.backend_apis[0].path, "/api/demo");
    }

    #[test]
    fn duplicate_backend_route_marks_second_plugin_failed() {
        let first = Arc::new(TestProvider {
            id: "first",
            route: "/first",
            api_path: "/api/shared",
        });
        let second = Arc::new(TestProvider {
            id: "second",
            route: "/second",
            api_path: "/api/shared",
        });

        let snapshot = NativePluginHost::new(NativePluginContext::default())
            .with_plugin(first)
            .with_plugin(second)
            .load_snapshot();
        let failed = snapshot
            .plugins
            .iter()
            .filter(|record| record.state == PluginState::Failed)
            .count();

        assert_eq!(failed, 1);
        assert_eq!(snapshot.backend_apis.len(), 1);
    }

    #[test]
    fn disabled_plugins_are_not_loaded() {
        let enabled = Arc::new(TestProvider {
            id: "enabled",
            route: "/enabled",
            api_path: "/api/enabled",
        });
        let disabled = Arc::new(TestProvider {
            id: "disabled",
            route: "/disabled",
            api_path: "/api/disabled",
        });
        let enablement = PluginEnablementStore {
            disabled_plugin_ids: ["disabled".to_string()].into_iter().collect(),
        };

        let snapshot = NativePluginHost::new(NativePluginContext::default())
            .with_plugin(enabled)
            .with_plugin(disabled)
            .load_snapshot_with_enablement(&enablement);
        let plugin_ids = snapshot
            .plugins
            .iter()
            .filter(|record| record.state == PluginState::Active)
            .map(|record| record.descriptor.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(plugin_ids, ["enabled"]);
    }
}
