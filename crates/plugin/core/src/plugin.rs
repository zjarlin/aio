#![forbid(unsafe_code)]

use crate::{
    AdminFieldKind, AdminMenuNodeKind, CatalogItemKind, CatalogSource, CatalogTagGroup,
    PluginActivation, PluginKind,
};
use anyhow::Context;
use serde::{Deserialize, Serialize};

pub type DynAdminPluginProvider = std::sync::Arc<dyn AdminPluginProvider>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDependency {
    pub id: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub activation: PluginActivation,
    pub priority: i32,
    pub dependencies: Vec<PluginDependency>,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub kind: PluginKind,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContributionSet {
    #[serde(default)]
    pub backend_apis: Vec<BackendApiContribution>,
    #[serde(default)]
    pub catalog_providers: Vec<CatalogProviderContribution>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminPluginContribution {
    pub menu: AdminMenuTree,
    #[serde(default)]
    pub resources: Vec<AdminResourceContract>,
    #[serde(default)]
    pub cli: Vec<AdminCliContribution>,
    pub native: ContributionSet,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminMenuTree {
    #[serde(default)]
    pub sections: Vec<AdminMenuSection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminMenuSection {
    pub domain_id: String,
    pub label: String,
    pub default_href: String,
    pub order: i32,
    #[serde(default)]
    pub menus: Vec<AdminMenuNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminMenuNode {
    pub id: String,
    pub kind: AdminMenuNodeKind,
    pub label: String,
    pub href: String,
    pub icon: String,
    pub order: i32,
    #[serde(default)]
    pub active_patterns: Vec<String>,
    #[serde(default)]
    pub permissions_any_of: Vec<String>,
    #[serde(default)]
    pub children: Vec<AdminMenuNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminResourceContract {
    pub id: String,
    pub label: String,
    pub description: String,
    pub route: String,
    pub table_name: String,
    #[serde(default)]
    pub permissions_any_of: Vec<String>,
    #[serde(default)]
    pub fields: Vec<AdminFieldContract>,
    #[serde(default)]
    pub operations: Vec<AdminOperationContract>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminFieldContract {
    pub name: String,
    pub label: String,
    pub kind: AdminFieldKind,
    pub required: bool,
    pub searchable: bool,
    pub table_visible: bool,
    pub form_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminOperationContract {
    pub id: String,
    pub label: String,
    pub method: String,
    pub path: String,
    pub cli: String,
    pub primary: bool,
    pub audit: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdminCliContribution {
    pub id: String,
    pub label: String,
    pub command: String,
    pub resource_id: Option<String>,
    pub operation_id: Option<String>,
}

impl ContributionSet {
    pub fn merge(&mut self, other: Self) {
        self.backend_apis.extend(other.backend_apis);
        self.catalog_providers.extend(other.catalog_providers);
    }
}

impl AdminPluginContribution {
    pub fn from_native(native: ContributionSet) -> Self {
        Self {
            menu: AdminMenuTree::default(),
            resources: Vec::new(),
            cli: Vec::new(),
            native,
        }
    }
}

pub fn merge_menu_tree(target: &mut AdminMenuTree, source: AdminMenuTree) {
    for section in source.sections {
        match target
            .sections
            .iter_mut()
            .find(|item| item.domain_id == section.domain_id)
        {
            Some(existing) => merge_menu_section(existing, section),
            None => target.sections.push(section),
        }
    }
    sort_menu_sections(&mut target.sections);
}

fn merge_menu_section(target: &mut AdminMenuSection, source: AdminMenuSection) {
    if target.default_href.is_empty() {
        target.default_href = source.default_href;
    }
    for node in source.menus {
        merge_menu_node(&mut target.menus, node);
    }
    sort_menu_nodes(&mut target.menus);
}

fn merge_menu_node(nodes: &mut Vec<AdminMenuNode>, source: AdminMenuNode) {
    match nodes.iter_mut().find(|node| node.id == source.id) {
        Some(existing) => {
            for child in source.children {
                merge_menu_node(&mut existing.children, child);
            }
            sort_menu_nodes(&mut existing.children);
        }
        None => nodes.push(source),
    }
}

pub fn sort_menu_sections(sections: &mut [AdminMenuSection]) {
    sections.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then(left.domain_id.cmp(&right.domain_id))
    });
    for section in sections {
        sort_menu_nodes(&mut section.menus);
    }
}

pub fn sort_menu_nodes(nodes: &mut [AdminMenuNode]) {
    nodes.sort_by(|left, right| left.order.cmp(&right.order).then(left.id.cmp(&right.id)));
    for node in nodes {
        sort_menu_nodes(&mut node.children);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackendApiContribution {
    pub id: String,
    pub method: String,
    pub path: String,
    pub label: String,
    pub description: String,
    pub order: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogProviderContribution {
    pub id: String,
    pub label: String,
    pub order: i32,
    pub items: Vec<CatalogItemContribution>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogItemContribution {
    pub id: String,
    pub name: String,
    pub description: String,
    pub section: String,
    pub icon: String,
    pub accent_class: String,
    pub kind: CatalogItemKind,
    pub source: CatalogSource,
    pub installed: bool,
    #[serde(default)]
    pub tags: Vec<CatalogTagContribution>,
    pub permissions: Vec<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogTagContribution {
    pub id: String,
    pub label: String,
    pub group: CatalogTagGroup,
}

#[derive(Clone, Debug)]
pub struct NativePluginContext {
    pub api_base_url: String,
    pub database_url: Option<String>,
    pub shared_db: Option<crate::Db>,
    pub config_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
}

impl Default for NativePluginContext {
    fn default() -> Self {
        Self {
            api_base_url: "http://127.0.0.1:0".to_string(),
            database_url: None,
            shared_db: None,
            config_dir: std::path::PathBuf::from("."),
            data_dir: std::path::PathBuf::from("."),
        }
    }
}

#[derive(Clone)]
pub struct NativePluginRuntime {
    pub router: axum::Router,
    pub startup: Option<fn(NativePluginContext) -> anyhow::Result<()>>,
}

impl Default for NativePluginRuntime {
    fn default() -> Self {
        Self {
            router: axum::Router::new(),
            startup: None,
        }
    }
}

pub trait NativePluginProvider: Send + Sync {
    fn descriptor(&self) -> PluginDescriptor;

    fn contributions(&self) -> anyhow::Result<ContributionSet>;

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime>;

    fn admin_menu(&self, contributions: &ContributionSet) -> AdminMenuTree {
        AdminPluginContribution::from_native(contributions.clone()).menu
    }

    fn admin_resources(&self) -> Vec<AdminResourceContract> {
        Vec::new()
    }

    fn admin_cli(&self) -> Vec<AdminCliContribution> {
        Vec::new()
    }
}

pub trait AdminPluginProvider: Send + Sync {
    fn admin_descriptor(&self) -> PluginDescriptor;

    fn admin_contribution(&self) -> anyhow::Result<AdminPluginContribution>;

    fn admin_runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime>;
}

impl<T> AdminPluginProvider for T
where
    T: NativePluginProvider,
{
    fn admin_descriptor(&self) -> PluginDescriptor {
        self.descriptor()
    }

    fn admin_contribution(&self) -> anyhow::Result<AdminPluginContribution> {
        let native = self.contributions()?;
        let menu = self.admin_menu(&native);
        Ok(AdminPluginContribution {
            menu,
            resources: self.admin_resources(),
            cli: self.admin_cli(),
            native,
        })
    }

    fn admin_runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        self.runtime(context)
    }
}

pub fn descriptor_to_json(descriptor: &PluginDescriptor) -> anyhow::Result<String> {
    serde_json::to_string(descriptor).context("plugin descriptor serialization failed")
}

pub fn contributions_to_json(contributions: &ContributionSet) -> anyhow::Result<String> {
    serde_json::to_string(contributions).context("plugin contributions serialization failed")
}

pub fn descriptor_from_json(value: &str) -> anyhow::Result<PluginDescriptor> {
    serde_json::from_str(value).context("plugin descriptor parse failed")
}

pub fn contributions_from_json(value: &str) -> anyhow::Result<ContributionSet> {
    serde_json::from_str(value).context("plugin contributions parse failed")
}
