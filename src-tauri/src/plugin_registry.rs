use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
    auth::require_session,
    error::{AppError, AppResult},
    permission_core::PermissionCore,
    plugin_store::PluginRegistryStore,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMatrix {
    #[serde(default)]
    pub supported: Vec<String>,
    #[serde(default)]
    pub degraded: Vec<String>,
    #[serde(default)]
    pub unsupported: Vec<String>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityFormula {
    pub id: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolContribution {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingContribution {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub schema: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContribution {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub schema: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyMatchContribution {
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub source_kinds: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub target_contains: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyContribution {
    pub id: String,
    pub title: String,
    #[serde(default = "deny_policy_effect")]
    pub effect: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub matches: PolicyMatchContribution,
    #[serde(default)]
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuContribution {
    #[serde(default)]
    pub permission_id: String,
    #[serde(default)]
    pub parent_permission_id: Option<String>,
    pub code: String,
    #[serde(alias = "name")]
    pub title: String,
    #[serde(default = "menu_permission_type")]
    pub permission_type: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub ordinary_user: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionContribution {
    #[serde(default)]
    pub permission_id: String,
    #[serde(default)]
    pub parent_permission_id: Option<String>,
    pub code: String,
    #[serde(alias = "name")]
    pub title: String,
    #[serde(default = "button_permission_type")]
    pub permission_type: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub ordinary_user: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewContribution {
    pub id: String,
    #[serde(default)]
    pub slot: String,
    pub schema: String,
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub asset_item_kind: String,
    #[serde(default)]
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPointContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub multiplicity: String,
    #[serde(default)]
    pub activation: String,
    #[serde(default)]
    pub allowed_contribution_kinds: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionBlock {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub tools: Vec<ToolContribution>,
    #[serde(default)]
    pub menus: Vec<MenuContribution>,
    #[serde(default)]
    pub permissions: Vec<PermissionContribution>,
    #[serde(default)]
    pub views: Vec<ViewContribution>,
    #[serde(default)]
    pub settings: Vec<SettingContribution>,
    #[serde(default)]
    pub resources: Vec<ResourceContribution>,
    #[serde(default)]
    pub policies: Vec<PolicyContribution>,
    #[serde(default)]
    pub extension_points: Vec<ExtensionPointContribution>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParent {
    pub plugin_id: String,
    pub mount: String,
    #[serde(default)]
    pub compatible_parent_range: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub browser: String,
    #[serde(default)]
    pub worker: String,
    #[serde(default)]
    pub remote: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventBlock {
    #[serde(default)]
    pub publishes: Vec<String>,
    #[serde(default)]
    pub subscribes: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAiHints {
    #[serde(default)]
    pub editable_by_ai: bool,
    #[serde(default)]
    pub primary_edit_file: String,
    #[serde(default)]
    pub preferred_files: Vec<String>,
    #[serde(default)]
    pub do_not_split_below: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub primary_output: String,
    #[serde(default)]
    pub repair_strategy: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildPolicy {
    #[serde(default)]
    pub allow_third_party_children: bool,
    #[serde(default)]
    pub capability_mode: String,
    #[serde(default)]
    pub event_namespace: String,
    #[serde(default)]
    pub requires_platform_approval_for_new_capabilities: bool,
    #[serde(default)]
    pub allowed_child_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProvides {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFormula {
    pub schema_version: String,
    pub id: String,
    pub kind: String,
    pub display_name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub entry: Option<PluginEntry>,
    #[serde(default)]
    pub parent: Option<PluginParent>,
    #[serde(default)]
    pub activation: Vec<String>,
    #[serde(default)]
    pub platforms: Option<PlatformMatrix>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityFormula>,
    #[serde(default)]
    pub contributes: ContributionBlock,
    #[serde(default)]
    pub events: EventBlock,
    #[serde(default)]
    pub expectations: Vec<String>,
    #[serde(default)]
    pub ai: PluginAiHints,
    #[serde(default)]
    pub child_policy: Option<ChildPolicy>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCapsule {
    pub schema_version: String,
    pub id: String,
    pub kind: String,
    pub display_name: String,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub mutable: bool,
    #[serde(default)]
    pub replaceable: bool,
    #[serde(default)]
    pub managed_by: String,
    #[serde(default)]
    pub platforms: Option<PlatformMatrix>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityFormula>,
    #[serde(default)]
    pub provides: Option<SystemProvides>,
    #[serde(default)]
    pub child_policy: Option<ChildPolicy>,
    #[serde(default)]
    pub contributes: ContributionBlock,
    #[serde(default)]
    pub ai: PluginAiHints,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPluginRecord {
    pub id: String,
    pub kind: String,
    pub display_name: String,
    pub version: String,
    pub intent: String,
    pub parent_plugin_id: Option<String>,
    pub parent_mount: Option<String>,
    pub platform_supported: Vec<String>,
    pub platform_degraded: Vec<String>,
    pub commands: Vec<String>,
    pub tools: Vec<String>,
    pub menus: Vec<String>,
    pub views: Vec<String>,
    pub extension_points: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPermissionSeedRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub permission_type: String,
    pub path: String,
    pub component: String,
    pub icon: String,
    pub sort_order: i64,
    pub ordinary_user: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCommandRecord {
    pub id: String,
    pub title: String,
    pub category: String,
    pub when: String,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryToolRecord {
    pub id: String,
    pub title: String,
    pub category: String,
    pub input: String,
    pub output: String,
    pub when: String,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryViewRecord {
    pub id: String,
    pub schema: String,
    pub contract: String,
    pub slot: String,
    pub path: String,
    pub when: String,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCapabilityRecord {
    pub id: String,
    pub scope: String,
    pub allow: Vec<String>,
    pub reason: String,
    pub platforms: Vec<String>,
    pub optional: bool,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEventRecord {
    pub event: String,
    pub direction: String,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPolicyRecord {
    pub id: String,
    pub title: String,
    pub effect: String,
    pub reason: String,
    pub priority: i64,
    pub source_ids: Vec<String>,
    pub source_kinds: Vec<String>,
    pub capabilities: Vec<String>,
    pub scopes: Vec<String>,
    pub target_contains: Vec<String>,
    pub platforms: Vec<String>,
    pub when: String,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryExtensionPointRecord {
    pub id: String,
    pub title: String,
    pub contract: String,
    pub multiplicity: String,
    pub activation: String,
    pub allowed_contribution_kinds: Vec<String>,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPermissionRecord {
    pub permission_id: String,
    pub parent_permission_id: Option<String>,
    pub code: String,
    pub title: String,
    pub permission_type: String,
    pub path: String,
    pub component: String,
    pub icon: String,
    pub sort_order: i64,
    pub ordinary_user: bool,
    pub source_id: String,
    pub source_kind: String,
    pub parent_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryExtensionTree {
    pub nodes: Vec<RegistryExtensionTreeNode>,
    pub mounts: Vec<RegistryExtensionTreeMount>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryExtensionTreeNode {
    pub plugin_id: String,
    pub display_name: String,
    pub kind: String,
    pub version: String,
    pub parent_plugin_id: Option<String>,
    pub parent_mount: Option<String>,
    pub parent_chain: Vec<String>,
    pub depth: usize,
    pub child_count: usize,
    pub commands: Vec<String>,
    pub views: Vec<String>,
    pub menus: Vec<String>,
    pub capabilities: Vec<String>,
    pub extension_points: Vec<String>,
    pub effective_capabilities: Vec<String>,
    pub capability_escalations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryExtensionTreeMount {
    pub parent_plugin_id: String,
    pub extension_point_id: String,
    pub child_plugin_id: String,
    pub compatible_parent_range: String,
    pub parent_chain: Vec<String>,
    pub commands: Vec<String>,
    pub views: Vec<String>,
    pub menus: Vec<String>,
    pub capabilities: Vec<String>,
    pub effective_capabilities: Vec<String>,
    pub capability_escalations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistrySnapshot {
    pub schema_version: String,
    pub plugins: Vec<RegistryPluginRecord>,
    pub system_capsules: Vec<RegistryPluginRecord>,
    pub extension_tree: RegistryExtensionTree,
    pub commands: Vec<RegistryCommandRecord>,
    pub tools: Vec<RegistryToolRecord>,
    pub views: Vec<RegistryViewRecord>,
    pub capabilities: Vec<RegistryCapabilityRecord>,
    pub policies: Vec<RegistryPolicyRecord>,
    pub events: Vec<RegistryEventRecord>,
    pub extension_points: Vec<RegistryExtensionPointRecord>,
    pub permissions: Vec<RegistryPermissionRecord>,
    pub permission_seeds: Vec<RegistryPermissionSeedRecord>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone)]
struct SourceRef {
    schema_version: String,
    id: String,
    kind: String,
    display_name: String,
    version: String,
    intent: String,
    parent: Option<PluginParent>,
    child_policy: Option<ChildPolicy>,
    platforms: Option<PlatformMatrix>,
    contributes: ContributionBlock,
    capabilities: Vec<CapabilityFormula>,
    events: EventBlock,
}

#[derive(Debug, Clone)]
pub struct SeedPermission {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub permission_type: String,
    pub path: String,
    pub component: String,
    pub icon: String,
    pub sort_order: i64,
    pub ordinary_user: bool,
}

pub fn builtin_registry() -> AppResult<PluginRegistrySnapshot> {
    registry_from_project_root(default_project_root())
}

pub fn default_project_root() -> PathBuf {
    if let Ok(root) = std::env::var("AIO_PLUGIN_PLATFORM_ROOT") {
        return PathBuf::from(root);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn registry_from_project_root(root: impl AsRef<Path>) -> AppResult<PluginRegistrySnapshot> {
    let root = root.as_ref();
    let system_capsules = load_system_capsules(&root.join("system-capsules"))?;
    let plugins = load_plugin_formulas(&root.join("plugins"))?;
    Ok(compile_registry(system_capsules, plugins))
}

pub fn registry_from_workspace(
    root: impl AsRef<Path>,
    data_dir: impl AsRef<Path>,
) -> AppResult<PluginRegistrySnapshot> {
    let root = root.as_ref();
    let store = PluginRegistryStore::new(data_dir);
    let mut system_capsules = load_system_capsules(&root.join("system-capsules"))?;
    let mut plugins = load_plugin_formulas(&root.join("plugins"))?;
    let (external_system_capsules, external_plugins) = load_external_sources(&store)?;
    system_capsules.extend(external_system_capsules);
    plugins.extend(external_plugins);
    Ok(compile_registry(system_capsules, plugins))
}

pub fn builtin_seed_permissions() -> AppResult<Vec<SeedPermission>> {
    let registry = builtin_registry()?;
    Ok(order_permissions_for_seed(registry.permissions)
        .into_iter()
        .map(|permission| SeedPermission {
            id: permission.permission_id,
            parent_id: permission.parent_permission_id,
            code: permission.code,
            name: permission.title,
            permission_type: permission.permission_type,
            path: permission.path,
            component: permission.component,
            icon: permission.icon,
            sort_order: permission.sort_order,
            ordinary_user: permission.ordinary_user,
        })
        .collect())
}

pub async fn snapshot(
    pool: &sqlx::SqlitePool,
    token: String,
    data_dir: &Path,
) -> AppResult<PluginRegistrySnapshot> {
    require_session(pool, &token).await?;
    registry_from_workspace(default_project_root(), data_dir)
}

#[cfg(test)]
pub fn command_source(command_id: &str) -> AppResult<Option<RegistryCommandRecord>> {
    Ok(builtin_registry()?
        .commands
        .into_iter()
        .find(|command| command.id == command_id))
}

fn compile_registry(
    system_capsules: Vec<SystemCapsule>,
    plugin_formulas: Vec<PluginFormula>,
) -> PluginRegistrySnapshot {
    let mut diagnostics = Vec::new();
    let mut parent_by_source = HashMap::new();
    for plugin in &plugin_formulas {
        if let Some(parent) = &plugin.parent {
            parent_by_source.insert(plugin.id.clone(), parent.plugin_id.clone());
        }
    }

    let system_sources = system_capsules
        .iter()
        .map(|capsule| SourceRef {
            schema_version: capsule.schema_version.clone(),
            id: capsule.id.clone(),
            kind: capsule.kind.clone(),
            display_name: capsule.display_name.clone(),
            version: String::new(),
            intent: capsule.intent.clone(),
            parent: None,
            child_policy: capsule.child_policy.clone(),
            platforms: capsule.platforms.clone(),
            contributes: capsule.contributes.clone(),
            capabilities: capsule.capabilities.clone(),
            events: EventBlock::default(),
        })
        .collect::<Vec<_>>();
    let plugin_sources = plugin_formulas
        .iter()
        .map(|plugin| SourceRef {
            schema_version: plugin.schema_version.clone(),
            id: plugin.id.clone(),
            kind: plugin.kind.clone(),
            display_name: plugin.display_name.clone(),
            version: plugin.version.clone(),
            intent: plugin.intent.clone(),
            parent: plugin.parent.clone(),
            child_policy: plugin.child_policy.clone(),
            platforms: plugin.platforms.clone(),
            contributes: plugin.contributes.clone(),
            capabilities: plugin.capabilities.clone(),
            events: plugin.events.clone(),
        })
        .collect::<Vec<_>>();
    let all_sources = system_sources
        .iter()
        .chain(plugin_sources.iter())
        .cloned()
        .collect::<Vec<_>>();

    let system_records = system_sources.iter().map(plugin_record).collect::<Vec<_>>();
    let plugin_records = plugin_sources.iter().map(plugin_record).collect::<Vec<_>>();

    let mut commands = Vec::new();
    let mut tools = Vec::new();
    let mut views = Vec::new();
    let mut capability_records = Vec::new();
    let mut policies = Vec::new();
    let mut events = Vec::new();
    let mut extension_points = Vec::new();
    let mut permissions = Vec::new();
    let mut seen_commands = HashSet::new();
    let mut seen_tools = HashSet::new();
    let mut seen_views = HashSet::new();
    let mut seen_extension_points = HashSet::new();
    let mut seen_permissions = HashSet::new();

    validate_source_graph(&all_sources, &mut diagnostics);

    for source in all_sources.iter() {
        let parent_chain = parent_chain_for(&source.id, &parent_by_source);
        let declared_capabilities = source
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str())
            .collect::<HashSet<_>>();

        for capability in &source.capabilities {
            PermissionCore::validate_platform_values(
                &capability.platforms,
                &format!("{}.capabilities.{}", source.id, capability.id),
                &mut diagnostics,
            );
            capability_records.push(RegistryCapabilityRecord {
                id: capability.id.clone(),
                scope: capability.scope.clone(),
                allow: capability.allow.clone(),
                reason: capability.reason.clone(),
                platforms: capability.platforms.clone(),
                optional: capability.optional,
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            });
        }

        append_event_records(source, &parent_chain, &mut events, &mut diagnostics);

        for policy in &source.contributes.policies {
            validate_policy_contribution(source, policy, &declared_capabilities, &mut diagnostics);
            policies.push(RegistryPolicyRecord {
                id: policy.id.clone(),
                title: policy.title.clone(),
                effect: policy.effect.clone(),
                reason: policy.reason.clone(),
                priority: policy.priority,
                source_ids: policy.matches.source_ids.clone(),
                source_kinds: policy.matches.source_kinds.clone(),
                capabilities: policy.matches.capabilities.clone(),
                scopes: policy.matches.scopes.clone(),
                target_contains: policy.matches.target_contains.clone(),
                platforms: policy.matches.platforms.clone(),
                when: policy.when.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            });
        }

        for command in &source.contributes.commands {
            if !seen_commands.insert(command.id.clone()) {
                diagnostics.push(format!("duplicate command contribution: {}", command.id));
            }
            validate_declared_capabilities(
                &source.id,
                "command",
                &command.id,
                &command.capabilities,
                &declared_capabilities,
                &mut diagnostics,
            );
            validate_contribution_when(
                source,
                "command",
                &command.id,
                &command.when,
                &declared_capabilities,
                &mut diagnostics,
            );
            commands.push(RegistryCommandRecord {
                id: command.id.clone(),
                title: command.title.clone(),
                category: command.category.clone(),
                when: command.when.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
                capabilities: command.capabilities.clone(),
            });
        }

        for tool in &source.contributes.tools {
            if !seen_tools.insert(tool.id.clone()) {
                diagnostics.push(format!("duplicate tool contribution: {}", tool.id));
            }
            validate_declared_capabilities(
                &source.id,
                "tool",
                &tool.id,
                &tool.capabilities,
                &declared_capabilities,
                &mut diagnostics,
            );
            validate_contribution_when(
                source,
                "tool",
                &tool.id,
                &tool.when,
                &declared_capabilities,
                &mut diagnostics,
            );
            tools.push(RegistryToolRecord {
                id: tool.id.clone(),
                title: tool.title.clone(),
                category: tool.category.clone(),
                input: tool.input.clone(),
                output: tool.output.clone(),
                when: tool.when.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
                capabilities: tool.capabilities.clone(),
            });
        }

        for view in &source.contributes.views {
            if !seen_views.insert(view.id.clone()) {
                diagnostics.push(format!("duplicate view contribution: {}", view.id));
            }
            views.push(RegistryViewRecord {
                id: view.id.clone(),
                schema: view.schema.clone(),
                contract: view.contract.clone(),
                slot: view.slot.clone(),
                path: view.path.clone(),
                when: view.when.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            });
            validate_contribution_when(
                source,
                "view",
                &view.id,
                &view.when,
                &declared_capabilities,
                &mut diagnostics,
            );
        }

        for extension_point in &source.contributes.extension_points {
            if !seen_extension_points.insert(extension_point.id.clone()) {
                diagnostics.push(format!(
                    "duplicate extension point contribution: {}",
                    extension_point.id
                ));
            }
            extension_points.push(RegistryExtensionPointRecord {
                id: extension_point.id.clone(),
                title: extension_point.title.clone(),
                contract: extension_point.contract.clone(),
                multiplicity: extension_point.multiplicity.clone(),
                activation: extension_point.activation.clone(),
                allowed_contribution_kinds: extension_point.allowed_contribution_kinds.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            });
        }

        for menu in &source.contributes.menus {
            let permission = RegistryPermissionRecord {
                permission_id: permission_id_for(&menu.permission_id, &menu.code),
                parent_permission_id: menu.parent_permission_id.clone(),
                code: menu.code.clone(),
                title: menu.title.clone(),
                permission_type: menu.permission_type.clone(),
                path: menu.path.clone(),
                component: menu.component.clone(),
                icon: menu.icon.clone(),
                sort_order: menu.sort_order,
                ordinary_user: menu.ordinary_user,
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            };
            if !seen_permissions.insert(permission.code.clone()) {
                diagnostics.push(format!(
                    "duplicate permission contribution: {}",
                    permission.code
                ));
            }
            permissions.push(permission);
        }

        for permission in &source.contributes.permissions {
            let record = RegistryPermissionRecord {
                permission_id: permission_id_for(&permission.permission_id, &permission.code),
                parent_permission_id: permission.parent_permission_id.clone(),
                code: permission.code.clone(),
                title: permission.title.clone(),
                permission_type: permission.permission_type.clone(),
                path: permission.path.clone(),
                component: permission.component.clone(),
                icon: permission.icon.clone(),
                sort_order: permission.sort_order,
                ordinary_user: permission.ordinary_user,
                source_id: source.id.clone(),
                source_kind: source.kind.clone(),
                parent_chain: parent_chain.clone(),
            };
            if !seen_permissions.insert(record.code.clone()) {
                diagnostics.push(format!(
                    "duplicate permission contribution: {}",
                    record.code
                ));
            }
            permissions.push(record);
        }
    }

    commands.sort_by(|a, b| a.id.cmp(&b.id));
    tools.sort_by(|a, b| a.id.cmp(&b.id));
    views.sort_by(|a, b| a.id.cmp(&b.id));
    capability_records.sort_by(|a, b| {
        a.source_id
            .cmp(&b.source_id)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.scope.cmp(&b.scope))
    });
    policies.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.source_id.cmp(&b.source_id))
    });
    extension_points.sort_by(|a, b| a.id.cmp(&b.id));
    permissions.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.code.cmp(&b.code))
    });
    let extension_tree = compile_extension_tree(&plugin_sources, &all_sources, &parent_by_source);
    validate_permission_graph(&permissions, &mut diagnostics);
    let permission_seeds = permissions
        .iter()
        .map(|permission| RegistryPermissionSeedRecord {
            id: permission.permission_id.clone(),
            parent_id: permission.parent_permission_id.clone(),
            code: permission.code.clone(),
            name: permission.title.clone(),
            permission_type: permission.permission_type.clone(),
            path: permission.path.clone(),
            component: permission.component.clone(),
            icon: permission.icon.clone(),
            sort_order: permission.sort_order,
            ordinary_user: permission.ordinary_user,
        })
        .collect();

    PluginRegistrySnapshot {
        schema_version: "aio-plugin-registry/v1".to_string(),
        plugins: plugin_records,
        system_capsules: system_records,
        extension_tree,
        commands,
        tools,
        views,
        capabilities: capability_records,
        policies,
        events,
        extension_points,
        permissions,
        permission_seeds,
        diagnostics,
    }
}

fn plugin_record(source: &SourceRef) -> RegistryPluginRecord {
    RegistryPluginRecord {
        id: source.id.clone(),
        kind: source.kind.clone(),
        display_name: source.display_name.clone(),
        version: source.version.clone(),
        intent: source.intent.clone(),
        parent_plugin_id: source
            .parent
            .as_ref()
            .map(|parent| parent.plugin_id.clone()),
        parent_mount: source.parent.as_ref().map(|parent| parent.mount.clone()),
        platform_supported: source
            .platforms
            .as_ref()
            .map(|platforms| platforms.supported.clone())
            .unwrap_or_default(),
        platform_degraded: source
            .platforms
            .as_ref()
            .map(|platforms| platforms.degraded.clone())
            .unwrap_or_default(),
        commands: source
            .contributes
            .commands
            .iter()
            .map(|command| command.id.clone())
            .collect(),
        tools: source
            .contributes
            .tools
            .iter()
            .map(|tool| tool.id.clone())
            .collect(),
        menus: source
            .contributes
            .menus
            .iter()
            .map(|menu| menu.code.clone())
            .collect(),
        views: source
            .contributes
            .views
            .iter()
            .map(|view| view.id.clone())
            .collect(),
        extension_points: source
            .contributes
            .extension_points
            .iter()
            .map(|extension_point| extension_point.id.clone())
            .collect(),
        capabilities: source
            .capabilities
            .iter()
            .map(|capability| capability.id.clone())
            .collect(),
    }
}

fn compile_extension_tree(
    plugin_sources: &[SourceRef],
    sources: &[SourceRef],
    parent_by_source: &HashMap<String, String>,
) -> RegistryExtensionTree {
    let mut child_count_by_parent: HashMap<String, usize> = HashMap::new();
    for source in plugin_sources {
        if let Some(parent) = &source.parent {
            *child_count_by_parent
                .entry(parent.plugin_id.clone())
                .or_default() += 1;
        }
    }

    let mut nodes = plugin_sources
        .iter()
        .map(|source| {
            let parent_chain = parent_chain_for(&source.id, parent_by_source);
            let capabilities = capability_ids(&source.capabilities);
            let capability_escalations = capability_escalations(source, sources);
            let effective_capabilities = capabilities
                .iter()
                .filter(|capability| !capability_escalations.contains(*capability))
                .cloned()
                .collect::<Vec<_>>();
            RegistryExtensionTreeNode {
                plugin_id: source.id.clone(),
                display_name: source.display_name.clone(),
                kind: source.kind.clone(),
                version: source.version.clone(),
                parent_plugin_id: source
                    .parent
                    .as_ref()
                    .map(|parent| parent.plugin_id.clone()),
                parent_mount: source.parent.as_ref().map(|parent| parent.mount.clone()),
                depth: parent_chain.len(),
                parent_chain,
                child_count: child_count_by_parent
                    .get(&source.id)
                    .copied()
                    .unwrap_or_default(),
                commands: command_ids(source),
                views: view_ids(source),
                menus: menu_codes(source),
                capabilities,
                extension_points: extension_point_ids(source),
                effective_capabilities,
                capability_escalations,
            }
        })
        .collect::<Vec<_>>();

    let mut mounts = plugin_sources
        .iter()
        .filter_map(|source| {
            let parent = source.parent.as_ref()?;
            let capabilities = capability_ids(&source.capabilities);
            let capability_escalations = capability_escalations(source, sources);
            let effective_capabilities = capabilities
                .iter()
                .filter(|capability| !capability_escalations.contains(*capability))
                .cloned()
                .collect::<Vec<_>>();
            Some(RegistryExtensionTreeMount {
                parent_plugin_id: parent.plugin_id.clone(),
                extension_point_id: parent.mount.clone(),
                child_plugin_id: source.id.clone(),
                compatible_parent_range: parent.compatible_parent_range.clone(),
                parent_chain: parent_chain_for(&source.id, parent_by_source),
                commands: command_ids(source),
                views: view_ids(source),
                menus: menu_codes(source),
                capabilities,
                effective_capabilities,
                capability_escalations,
            })
        })
        .collect::<Vec<_>>();

    nodes.sort_by(|a, b| {
        a.parent_chain
            .cmp(&b.parent_chain)
            .then_with(|| a.plugin_id.cmp(&b.plugin_id))
    });
    mounts.sort_by(|a, b| {
        a.parent_plugin_id
            .cmp(&b.parent_plugin_id)
            .then_with(|| a.extension_point_id.cmp(&b.extension_point_id))
            .then_with(|| a.child_plugin_id.cmp(&b.child_plugin_id))
    });
    RegistryExtensionTree { nodes, mounts }
}

fn capability_escalations(source: &SourceRef, sources: &[SourceRef]) -> Vec<String> {
    let Some(parent) = &source.parent else {
        return Vec::new();
    };
    let Some(parent_source) = sources
        .iter()
        .find(|candidate| candidate.id == parent.plugin_id)
    else {
        return capability_ids(&source.capabilities);
    };
    let allowed = allowed_child_capabilities(parent_source);
    capability_ids(&source.capabilities)
        .into_iter()
        .filter(|capability| !allowed.contains(capability))
        .collect()
}

fn allowed_child_capabilities(parent_source: &SourceRef) -> HashSet<String> {
    let mut allowed = capability_ids(&parent_source.capabilities)
        .into_iter()
        .collect::<HashSet<_>>();
    if let Some(policy) = &parent_source.child_policy {
        allowed.extend(policy.allowed_child_capabilities.iter().cloned());
    }
    allowed
}

fn capability_ids(source_capabilities: &[CapabilityFormula]) -> Vec<String> {
    sorted_unique(
        source_capabilities
            .iter()
            .map(|capability| capability.id.clone()),
    )
}

fn command_ids(source: &SourceRef) -> Vec<String> {
    sorted_unique(
        source
            .contributes
            .commands
            .iter()
            .map(|command| command.id.clone()),
    )
}

fn view_ids(source: &SourceRef) -> Vec<String> {
    sorted_unique(source.contributes.views.iter().map(|view| view.id.clone()))
}

fn menu_codes(source: &SourceRef) -> Vec<String> {
    sorted_unique(
        source
            .contributes
            .menus
            .iter()
            .map(|menu| menu.code.clone()),
    )
}

fn extension_point_ids(source: &SourceRef) -> Vec<String> {
    sorted_unique(
        source
            .contributes
            .extension_points
            .iter()
            .map(|extension_point| extension_point.id.clone()),
    )
}

fn sorted_unique(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut values = values.collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn parent_version_matches(range: &str, version: &str) -> bool {
    let range = range.trim();
    if range == "*" || range.is_empty() {
        return true;
    }
    if let Some(expected) = range.strip_prefix('^') {
        return caret_version_matches(expected, version);
    }
    range == version
}

fn caret_version_matches(expected: &str, actual: &str) -> bool {
    let Some(expected) = parse_semver_triplet(expected) else {
        return false;
    };
    let Some(actual) = parse_semver_triplet(actual) else {
        return false;
    };
    if actual < expected {
        return false;
    }
    if expected.0 > 0 {
        actual.0 == expected.0
    } else if expected.1 > 0 {
        actual.0 == 0 && actual.1 == expected.1
    } else {
        actual.0 == 0 && actual.1 == 0 && actual.2 == expected.2
    }
}

fn parse_semver_triplet(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn parent_chain_for(plugin_id: &str, parent_by_plugin: &HashMap<String, String>) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = plugin_id;
    let mut seen = HashSet::new();
    while let Some(parent) = parent_by_plugin.get(current) {
        if !seen.insert(parent.clone()) {
            break;
        }
        chain.push(parent.clone());
        current = parent;
    }
    chain.reverse();
    chain
}

fn permission_id_for(permission_id: &str, code: &str) -> String {
    if permission_id.is_empty() {
        format!("perm-{}", code.replace([':', '_'], "-"))
    } else {
        permission_id.to_string()
    }
}

fn order_permissions_for_seed(
    permissions: Vec<RegistryPermissionRecord>,
) -> Vec<RegistryPermissionRecord> {
    let mut by_id: HashMap<String, RegistryPermissionRecord> = permissions
        .into_iter()
        .map(|permission| (permission.permission_id.clone(), permission))
        .collect();
    let mut ordered = Vec::new();
    let mut visiting = HashSet::new();
    let keys = by_id.keys().cloned().collect::<Vec<_>>();

    for key in keys {
        visit_permission(&key, &mut by_id, &mut visiting, &mut ordered);
    }

    ordered
}

fn visit_permission(
    permission_id: &str,
    by_id: &mut HashMap<String, RegistryPermissionRecord>,
    visiting: &mut HashSet<String>,
    ordered: &mut Vec<RegistryPermissionRecord>,
) {
    if ordered
        .iter()
        .any(|permission| permission.permission_id == permission_id)
    {
        return;
    }

    if !visiting.insert(permission_id.to_string()) {
        return;
    }

    let parent_id = by_id
        .get(permission_id)
        .and_then(|permission| permission.parent_permission_id.clone());
    if let Some(parent_id) = parent_id {
        if by_id.contains_key(&parent_id) {
            visit_permission(&parent_id, by_id, visiting, ordered);
        }
    }

    if let Some(permission) = by_id.remove(permission_id) {
        ordered.push(permission);
    }
    visiting.remove(permission_id);
}

fn load_system_capsules(root: &Path) -> AppResult<Vec<SystemCapsule>> {
    discover_json_files(root, "system-capsule.json")
}

fn load_plugin_formulas(root: &Path) -> AppResult<Vec<PluginFormula>> {
    discover_json_files(root, "formula.json")
}

fn load_external_sources(
    store: &PluginRegistryStore,
) -> AppResult<(Vec<SystemCapsule>, Vec<PluginFormula>)> {
    let plugins_dir = store.plugins_dir();
    if !plugins_dir.is_dir() {
        return Ok((Vec::new(), Vec::new()));
    }

    let enabled = store.enabled_ids()?;
    let system_capsules = discover_optional_json_files(&plugins_dir, "system-capsule.json")?
        .into_iter()
        .filter(|capsule: &SystemCapsule| enabled.get(&capsule.id).copied().unwrap_or(false))
        .collect::<Vec<_>>();
    let plugin_formulas = discover_optional_json_files(&plugins_dir, "formula.json")?
        .into_iter()
        .filter(|plugin: &PluginFormula| enabled.get(&plugin.id).copied().unwrap_or(false))
        .collect::<Vec<_>>();

    Ok((system_capsules, plugin_formulas))
}

fn discover_optional_json_files<T>(root: &Path, file_name: &str) -> AppResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!(
                "扫描插件注册表目录失败：{}: {error}",
                root.display()
            ))
        })?;
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == file_name {
            paths.push(entry.into_path());
        }
    }
    paths.sort();

    if paths.is_empty() {
        return Ok(Vec::new());
    }

    paths
        .iter()
        .map(|path| parse_json_file(path))
        .collect::<AppResult<Vec<_>>>()
}

fn discover_json_files<T>(root: &Path, file_name: &str) -> AppResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if !root.is_dir() {
        return Err(AppError::BadRequest(format!(
            "插件注册表目录不存在：{}",
            root.display()
        )));
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!(
                "扫描插件注册表目录失败：{}: {error}",
                root.display()
            ))
        })?;
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == file_name {
            paths.push(entry.into_path());
        }
    }
    paths.sort();

    if paths.is_empty() {
        return Err(AppError::BadRequest(format!(
            "插件注册表目录未发现 {file_name}：{}",
            root.display()
        )));
    }

    paths
        .iter()
        .map(|path| parse_json_file(path))
        .collect::<AppResult<Vec<_>>>()
}

fn parse_json_file<T>(path: &Path) -> AppResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let value = std::fs::read_to_string(path)?;
    serde_json::from_str(&value).map_err(|source| {
        AppError::BadRequest(format!("解析插件公式失败：{}: {source}", path.display()))
    })
}

fn validate_source_graph(sources: &[SourceRef], diagnostics: &mut Vec<String>) {
    let mut source_ids = HashSet::new();
    let mut source_by_id = HashMap::new();
    for source in sources {
        source_by_id.insert(source.id.as_str(), source);
    }

    for source in sources {
        if !source_ids.insert(source.id.as_str()) {
            diagnostics.push(format!("duplicate source id: {}", source.id));
        }
        validate_platform_matrix(&source.id, source.platforms.as_ref(), diagnostics);

        match source.kind.as_str() {
            "system" => {
                PermissionCore::validate_schema_version(
                    &source.id,
                    "system-capsule/v1",
                    &source.schema_version,
                    diagnostics,
                );
            }
            "plugin" | "child-plugin" => {
                PermissionCore::validate_schema_version(
                    &source.id,
                    "plugin-formula/v1",
                    &source.schema_version,
                    diagnostics,
                );
            }
            other => diagnostics.push(format!("unknown source kind {other} for {}", source.id)),
        }

        if source.kind == "child-plugin" {
            if source.parent.is_none() {
                diagnostics.push(format!("child plugin missing parent: {}", source.id));
            }
        } else if source.parent.is_some() {
            diagnostics.push(format!("non-child plugin declares parent: {}", source.id));
        }

        if source.kind == "plugin" && source.version.trim().is_empty() {
            diagnostics.push(format!("plugin {} missing version", source.id));
        }
    }

    for source in sources.iter().filter(|source| source.kind != "system") {
        let Some(parent) = source.parent.as_ref() else {
            continue;
        };
        let Some(parent_source) = source_by_id.get(parent.plugin_id.as_str()) else {
            diagnostics.push(format!(
                "plugin {} mounts unknown parent {}",
                source.id, parent.plugin_id
            ));
            continue;
        };
        let parent_has_mount = parent_source
            .contributes
            .extension_points
            .iter()
            .any(|extension_point| extension_point.id == parent.mount);
        if !parent_has_mount {
            diagnostics.push(format!(
                "plugin {} mounts unknown extension point {} on {}",
                source.id, parent.mount, parent.plugin_id
            ));
        }
        validate_parent_compatibility(source, parent_source, parent, diagnostics);
        validate_child_capability_policy(source, parent_source, diagnostics);
    }
}

fn validate_parent_compatibility(
    source: &SourceRef,
    parent_source: &SourceRef,
    parent: &PluginParent,
    diagnostics: &mut Vec<String>,
) {
    if parent_source.kind == "system" {
        return;
    }
    let range = parent.compatible_parent_range.trim();
    if range.is_empty() {
        diagnostics.push(format!(
            "plugin {} missing compatibleParentRange for parent {}",
            source.id, parent.plugin_id
        ));
        return;
    }
    if parent_source.version.trim().is_empty() {
        diagnostics.push(format!(
            "plugin {} cannot verify parent compatibility because {} has no version",
            source.id, parent.plugin_id
        ));
        return;
    }
    if !parent_version_matches(range, &parent_source.version) {
        diagnostics.push(format!(
            "plugin {} requires parent {} {}, but installed version is {}",
            source.id, parent.plugin_id, range, parent_source.version
        ));
    }
}

fn validate_child_capability_policy(
    child_source: &SourceRef,
    parent_source: &SourceRef,
    diagnostics: &mut Vec<String>,
) {
    let child_capabilities = capability_ids(&child_source.capabilities);
    if child_capabilities.is_empty() {
        return;
    }

    let allowed = allowed_child_capabilities(parent_source);
    for capability in child_capabilities {
        if !allowed.contains(&capability) {
            diagnostics.push(format!(
                "child plugin {} requests capability {} outside parent {} child policy; platform approval required",
                child_source.id, capability, parent_source.id
            ));
        }
    }
}

fn validate_platform_matrix(
    source_id: &str,
    platforms: Option<&PlatformMatrix>,
    diagnostics: &mut Vec<String>,
) {
    let Some(platforms) = platforms else {
        return;
    };
    PermissionCore::validate_platform_lists(
        source_id,
        &platforms.supported,
        &platforms.degraded,
        &platforms.unsupported,
        diagnostics,
    );
}

fn validate_declared_capabilities(
    source_id: &str,
    contribution_kind: &str,
    contribution_id: &str,
    required_capabilities: &[String],
    declared_capabilities: &HashSet<&str>,
    diagnostics: &mut Vec<String>,
) {
    PermissionCore::validate_declared_capabilities(
        source_id,
        contribution_kind,
        contribution_id,
        required_capabilities,
        declared_capabilities,
        diagnostics,
    );
}

fn validate_contribution_when(
    source: &SourceRef,
    contribution_kind: &str,
    contribution_id: &str,
    when: &str,
    declared_capabilities: &HashSet<&str>,
    diagnostics: &mut Vec<String>,
) {
    PermissionCore::validate_when_expression(
        &source.id,
        contribution_kind,
        contribution_id,
        when,
        source.platforms.as_ref(),
        declared_capabilities,
        diagnostics,
    );
}

fn validate_policy_contribution(
    source: &SourceRef,
    policy: &PolicyContribution,
    declared_capabilities: &HashSet<&str>,
    diagnostics: &mut Vec<String>,
) {
    if !matches!(policy.effect.as_str(), "deny" | "warn") {
        diagnostics.push(format!(
            "{} policy.{} has unsupported effect {}",
            source.id, policy.id, policy.effect
        ));
    }

    PermissionCore::validate_platform_values(
        &policy.matches.platforms,
        &format!("{}.policies.{}.matches.platforms", source.id, policy.id),
        diagnostics,
    );
    validate_contribution_when(
        source,
        "policy",
        &policy.id,
        &policy.when,
        declared_capabilities,
        diagnostics,
    );
}

fn append_event_records(
    source: &SourceRef,
    parent_chain: &[String],
    records: &mut Vec<RegistryEventRecord>,
    diagnostics: &mut Vec<String>,
) {
    PermissionCore::validate_event_names(
        &source.id,
        "publishes",
        &source.events.publishes,
        diagnostics,
    );
    PermissionCore::validate_event_names(
        &source.id,
        "subscribes",
        &source.events.subscribes,
        diagnostics,
    );

    for event in source
        .events
        .publishes
        .iter()
        .filter(|event| !event.trim().is_empty())
    {
        records.push(RegistryEventRecord {
            event: event.clone(),
            direction: "publish".to_string(),
            source_id: source.id.clone(),
            source_kind: source.kind.clone(),
            parent_chain: parent_chain.to_vec(),
        });
    }
    for event in source
        .events
        .subscribes
        .iter()
        .filter(|event| !event.trim().is_empty())
    {
        records.push(RegistryEventRecord {
            event: event.clone(),
            direction: "subscribe".to_string(),
            source_id: source.id.clone(),
            source_kind: source.kind.clone(),
            parent_chain: parent_chain.to_vec(),
        });
    }
}

fn validate_permission_graph(
    permissions: &[RegistryPermissionRecord],
    diagnostics: &mut Vec<String>,
) {
    let parent_pairs = permissions
        .iter()
        .map(|permission| {
            (
                permission.permission_id.clone(),
                permission.parent_permission_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    PermissionCore::validate_permission_parent_ids(&parent_pairs, diagnostics);
}

fn menu_permission_type() -> String {
    "menu".to_string()
}

fn button_permission_type() -> String {
    "button".to_string()
}

fn deny_policy_effect() -> String {
    "deny".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        builtin_registry, builtin_seed_permissions, command_source, compile_registry,
        default_project_root, registry_from_project_root, registry_from_workspace,
        CapabilityFormula, ChildPolicy, CommandContribution, ContributionBlock, EventBlock,
        ExtensionPointContribution, PlatformMatrix, PluginAiHints, PluginFormula, PluginParent,
        ResourceContribution, SystemCapsule, ToolContribution, ViewContribution,
    };

    #[test]
    fn builtin_registry_compiles_current_asset_tree() {
        let registry = builtin_registry().expect("registry compiles");

        assert!(
            registry.diagnostics.is_empty(),
            "{:?}",
            registry.diagnostics
        );
        assert!(registry
            .plugins
            .iter()
            .any(|plugin| plugin.id == "asset-suite"));
        assert!(registry
            .plugins
            .iter()
            .any(|plugin| plugin.id == "asset.notes"
                && plugin.parent_plugin_id.as_deref() == Some("asset-suite")));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "note_page"
                && command.source_id == "asset.notes"
                && command.parent_chain == vec!["asset-suite".to_string()]));
        let notes_node = registry
            .extension_tree
            .nodes
            .iter()
            .find(|node| node.plugin_id == "asset.notes")
            .expect("notes tree node");
        assert_eq!(notes_node.parent_plugin_id.as_deref(), Some("asset-suite"));
        assert_eq!(
            notes_node.parent_mount.as_deref(),
            Some("asset-suite.assetView")
        );
        assert_eq!(notes_node.depth, 1);
        assert_eq!(notes_node.parent_chain, vec!["asset-suite".to_string()]);
        assert!(notes_node.commands.contains(&"note_page".to_string()));
        assert!(notes_node.views.contains(&"asset.notes.table".to_string()));
        assert_eq!(
            notes_node.effective_capabilities,
            vec!["db.sqlite.read".to_string(), "db.sqlite.write".to_string()]
        );
        assert!(notes_node.capability_escalations.is_empty());
        assert!(registry.extension_tree.mounts.iter().any(|mount| {
            mount.parent_plugin_id == "asset-suite"
                && mount.extension_point_id == "asset-suite.assetView"
                && mount.child_plugin_id == "asset.notes"
                && mount.compatible_parent_range == "^1.0.0"
        }));
        assert!(registry
            .extension_tree
            .mounts
            .iter()
            .any(|mount| mount.parent_plugin_id == "git-suite"
                && mount.extension_point_id == "git-suite.remoteProvider"
                && mount.child_plugin_id == "git-suite.github-provider"
                && mount.effective_capabilities
                    == vec!["browser.openUrl".to_string(), "network.fetch".to_string()]
                && mount.capability_escalations.is_empty()));
        assert!(registry.commands.iter().any(|command| {
            command.id == "github_sync_pull_requests"
                && command.source_id == "git-suite.github-provider"
                && command.parent_chain == vec!["git-suite".to_string()]
        }));
        assert!(registry.events.iter().any(|event| {
            event.event == "asset.item.changed"
                && event.direction == "publish"
                && event.source_id == "asset-suite"
        }));
        assert!(registry.events.iter().any(|event| {
            event.event == "git-suite.repository.changed"
                && event.direction == "publish"
                && event.source_id == "git-suite"
        }));
        assert!(registry
            .permissions
            .iter()
            .any(|permission| permission.code == "assets:docker_compose"
                && permission.parent_permission_id.as_deref() == Some("perm-notes")));
    }

    #[test]
    fn builtin_seed_permissions_preserve_existing_ids() {
        let permissions = builtin_seed_permissions().expect("permissions compile");

        assert!(permissions
            .iter()
            .any(|permission| permission.id == "perm-system-skill"
                && permission.code == "assets:skill"
                && permission.ordinary_user));
        assert!(permissions
            .iter()
            .any(|permission| permission.id == "perm-action-create"
                && permission.permission_type == "button"));
    }

    #[test]
    fn command_source_can_trace_plugin_command() {
        let source = command_source("asset_item_import_directory")
            .expect("registry compiles")
            .expect("command exists");

        assert_eq!(source.source_id, "asset-suite");
        assert_eq!(source.capabilities, vec!["fs.read".to_string()]);
    }

    #[test]
    fn command_source_can_trace_plugin_factory_command() {
        let source = command_source("plugin_create_from_prompt")
            .expect("registry compiles")
            .expect("command exists");

        assert_eq!(source.source_id, "platform.plugin-factory");
        assert_eq!(source.capabilities, vec!["fs.write".to_string()]);
    }

    #[test]
    fn command_source_can_trace_plugin_factory_publish_command() {
        let source = command_source("plugin_publish_local")
            .expect("registry compiles")
            .expect("command exists");

        assert_eq!(source.source_id, "platform.plugin-factory");
        assert_eq!(
            source.capabilities,
            vec!["fs.read".to_string(), "fs.write".to_string()]
        );
    }

    #[test]
    fn command_source_can_trace_plugin_factory_repair_command() {
        let source = command_source("plugin_repair_from_diagnostics")
            .expect("registry compiles")
            .expect("command exists");

        assert_eq!(source.source_id, "platform.plugin-factory");
        assert_eq!(
            source.capabilities,
            vec!["fs.read".to_string(), "fs.write".to_string()]
        );
    }

    #[test]
    fn command_source_can_trace_extension_host_load_command() {
        let source = command_source("plugin_host_load")
            .expect("registry compiles")
            .expect("command exists");

        assert_eq!(source.source_id, "platform.extension-host");
        assert_eq!(source.capabilities, vec!["fs.read".to_string()]);
    }

    #[test]
    fn filesystem_discovery_loads_registry_contract_files() {
        let registry =
            registry_from_project_root(default_project_root()).expect("registry loads from disk");

        assert!(registry
            .extension_points
            .iter()
            .any(|extension_point| extension_point.id == "asset-suite.assetView"));
        assert!(registry
            .capabilities
            .iter()
            .any(|capability| capability.id == "fs.read" && capability.source_id == "asset-suite"));
        assert_eq!(registry.system_capsules.len(), 8);
        assert!(registry
            .system_capsules
            .iter()
            .any(|capsule| capsule.id == "platform.permission-core"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "plugin_publish_local"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "plugin_repair_from_diagnostics"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "plugin_host_load"));
        assert!(registry
            .system_capsules
            .iter()
            .any(|capsule| capsule.id == "platform.event-bus"));
        assert!(registry
            .capabilities
            .iter()
            .any(|capability| capability.id == "event.publish"
                && capability.source_id == "platform.event-bus"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "event_bus_snapshot"));
        assert!(registry
            .system_capsules
            .iter()
            .any(|capsule| capsule.id == "platform.capability-broker"));
        assert!(registry
            .capabilities
            .iter()
            .any(|capability| capability.id == "clipboard.read"
                && capability.source_id == "platform.capability-broker"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "capability_clipboard_write"));
        assert!(registry
            .commands
            .iter()
            .any(|command| command.id == "capability_notification_send"));
        assert_eq!(registry.system_capsules.len(), 8);
        assert!(registry
            .plugins
            .iter()
            .any(|plugin| plugin.id == "platform.macos-clipboard"
                && plugin.platform_supported == vec!["macos".to_string()]
                && plugin.platform_degraded == vec!["web".to_string(), "remote".to_string()]));
        assert!(registry
            .plugins
            .iter()
            .any(|plugin| plugin.id == "platform.windows-notification"
                && plugin.platform_supported == vec!["windows".to_string()]
                && plugin.platform_degraded == vec!["web".to_string(), "remote".to_string()]));
    }

    #[test]
    fn workspace_registry_tolerates_empty_external_registry_dir() {
        let data_dir = tempfile::tempdir().expect("temp data dir");
        std::fs::create_dir_all(data_dir.path().join("plugin-registry/plugins"))
            .expect("create empty external registry dir");

        let registry = registry_from_workspace(default_project_root(), data_dir.path())
            .expect("workspace registry loads");
        let builtin =
            registry_from_project_root(default_project_root()).expect("builtin registry loads");

        assert_eq!(registry.plugins.len(), builtin.plugins.len());
        assert_eq!(
            registry.system_capsules.len(),
            builtin.system_capsules.len()
        );
        assert!(registry.diagnostics.is_empty());
    }

    #[test]
    fn child_policy_should_report_capability_escalation_and_parent_version_mismatch() {
        let parent = PluginFormula {
            schema_version: "plugin-formula/v1".to_string(),
            id: "parent-suite".to_string(),
            kind: "plugin".to_string(),
            display_name: "Parent Suite".to_string(),
            version: "1.2.0".to_string(),
            intent: String::new(),
            entry: None,
            parent: None,
            activation: Vec::new(),
            platforms: None,
            capabilities: vec![CapabilityFormula {
                id: "fs.read".to_string(),
                scope: "workspace".to_string(),
                allow: Vec::new(),
                reason: String::new(),
                platforms: Vec::new(),
                optional: false,
            }],
            contributes: ContributionBlock {
                extension_points: vec![ExtensionPointContribution {
                    id: "parent-suite.slot".to_string(),
                    title: "Slot".to_string(),
                    contract: String::new(),
                    multiplicity: "many".to_string(),
                    activation: "withParent".to_string(),
                    allowed_contribution_kinds: vec!["command".to_string()],
                }],
                ..ContributionBlock::default()
            },
            events: EventBlock::default(),
            expectations: Vec::new(),
            ai: PluginAiHints::default(),
            child_policy: Some(ChildPolicy {
                allow_third_party_children: true,
                capability_mode: "intersection".to_string(),
                event_namespace: "parent-suite/*".to_string(),
                requires_platform_approval_for_new_capabilities: true,
                allowed_child_capabilities: vec!["fs.read".to_string()],
            }),
        };
        let child = PluginFormula {
            schema_version: "plugin-formula/v1".to_string(),
            id: "parent-suite.child".to_string(),
            kind: "child-plugin".to_string(),
            display_name: "Child".to_string(),
            version: String::new(),
            intent: String::new(),
            entry: None,
            parent: Some(PluginParent {
                plugin_id: "parent-suite".to_string(),
                mount: "parent-suite.slot".to_string(),
                compatible_parent_range: "^2.0.0".to_string(),
            }),
            activation: Vec::new(),
            platforms: None,
            capabilities: vec![
                CapabilityFormula {
                    id: "fs.read".to_string(),
                    scope: "workspace".to_string(),
                    allow: Vec::new(),
                    reason: String::new(),
                    platforms: Vec::new(),
                    optional: false,
                },
                CapabilityFormula {
                    id: "process.exec".to_string(),
                    scope: "node".to_string(),
                    allow: Vec::new(),
                    reason: String::new(),
                    platforms: Vec::new(),
                    optional: false,
                },
            ],
            contributes: ContributionBlock {
                commands: vec![CommandContribution {
                    id: "child.run".to_string(),
                    title: "Run Child".to_string(),
                    category: "child".to_string(),
                    capabilities: vec!["fs.read".to_string()],
                    when: String::new(),
                }],
                ..ContributionBlock::default()
            },
            events: EventBlock::default(),
            expectations: Vec::new(),
            ai: PluginAiHints::default(),
            child_policy: None,
        };

        let registry = compile_registry(Vec::new(), vec![parent, child]);
        let child_node = registry
            .extension_tree
            .nodes
            .iter()
            .find(|node| node.plugin_id == "parent-suite.child")
            .expect("child node");

        assert_eq!(
            child_node.capability_escalations,
            vec!["process.exec".to_string()]
        );
        assert_eq!(
            child_node.effective_capabilities,
            vec!["fs.read".to_string()]
        );
        assert!(registry.diagnostics.iter().any(|diagnostic| diagnostic
            .contains("plugin parent-suite.child requires parent parent-suite ^2.0.0")));
        assert!(registry.diagnostics.iter().any(|diagnostic| diagnostic.contains(
            "child plugin parent-suite.child requests capability process.exec outside parent parent-suite child policy"
        )));
    }

    #[test]
    fn child_plugin_can_mount_system_capsule_extension_point() {
        let permission_core = SystemCapsule {
            schema_version: "system-capsule/v1".to_string(),
            id: "platform.permission-core".to_string(),
            kind: "system".to_string(),
            display_name: "Permission Core".to_string(),
            intent: String::new(),
            mutable: false,
            replaceable: false,
            managed_by: "platform".to_string(),
            platforms: None,
            capabilities: vec![CapabilityFormula {
                id: "permission.audit".to_string(),
                scope: "runtime-permission-history".to_string(),
                allow: Vec::new(),
                reason: String::new(),
                platforms: Vec::new(),
                optional: false,
            }],
            provides: None,
            child_policy: Some(ChildPolicy {
                allow_third_party_children: true,
                capability_mode: "intersection".to_string(),
                event_namespace: "permission-core/*".to_string(),
                requires_platform_approval_for_new_capabilities: true,
                allowed_child_capabilities: vec!["fs.write".to_string()],
            }),
            contributes: ContributionBlock {
                extension_points: vec![ExtensionPointContribution {
                    id: "permission.auditSink".to_string(),
                    title: "Permission Audit Sink".to_string(),
                    contract: "contracts/permission.audit-sink.v1.json".to_string(),
                    multiplicity: "many".to_string(),
                    activation: "withAudit".to_string(),
                    allowed_contribution_kinds: vec!["resource".to_string()],
                }],
                ..ContributionBlock::default()
            },
            ai: PluginAiHints::default(),
        };
        let audit_sink = PluginFormula {
            schema_version: "plugin-formula/v1".to_string(),
            id: "governance.audit-file-sink".to_string(),
            kind: "child-plugin".to_string(),
            display_name: "Audit File Sink".to_string(),
            version: "0.1.0".to_string(),
            intent: String::new(),
            entry: None,
            parent: Some(PluginParent {
                plugin_id: "platform.permission-core".to_string(),
                mount: "permission.auditSink".to_string(),
                compatible_parent_range: String::new(),
            }),
            activation: Vec::new(),
            platforms: None,
            capabilities: vec![CapabilityFormula {
                id: "fs.write".to_string(),
                scope: "app-data/plugin-audit".to_string(),
                allow: Vec::new(),
                reason: String::new(),
                platforms: Vec::new(),
                optional: false,
            }],
            contributes: ContributionBlock {
                resources: vec![ResourceContribution {
                    id: "governance.audit-file-sink.file".to_string(),
                    title: "Permission Decision Audit File".to_string(),
                    kind: "permission-audit-sink".to_string(),
                    schema: "contracts/permission.audit-sink.v1.json".to_string(),
                }],
                ..ContributionBlock::default()
            },
            events: EventBlock::default(),
            expectations: Vec::new(),
            ai: PluginAiHints::default(),
            child_policy: None,
        };

        let registry = compile_registry(vec![permission_core], vec![audit_sink]);

        assert!(
            registry.diagnostics.is_empty(),
            "{:?}",
            registry.diagnostics
        );
        let mount = registry
            .extension_tree
            .mounts
            .iter()
            .find(|mount| mount.child_plugin_id == "governance.audit-file-sink")
            .expect("system capsule child mount");
        assert_eq!(mount.parent_plugin_id, "platform.permission-core");
        assert_eq!(mount.extension_point_id, "permission.auditSink");
        assert_eq!(
            mount.parent_chain,
            vec!["platform.permission-core".to_string()]
        );
        assert_eq!(mount.effective_capabilities, vec!["fs.write".to_string()]);
        assert!(mount.capability_escalations.is_empty());
    }

    #[test]
    fn policy_child_plugin_compiles_policy_records() {
        let permission_core = SystemCapsule {
            schema_version: "system-capsule/v1".to_string(),
            id: "platform.permission-core".to_string(),
            kind: "system".to_string(),
            display_name: "Permission Core".to_string(),
            intent: String::new(),
            mutable: false,
            replaceable: false,
            managed_by: "platform".to_string(),
            platforms: None,
            capabilities: Vec::new(),
            provides: None,
            child_policy: Some(ChildPolicy {
                allow_third_party_children: true,
                capability_mode: "intersection".to_string(),
                event_namespace: "permission-core/*".to_string(),
                requires_platform_approval_for_new_capabilities: true,
                allowed_child_capabilities: Vec::new(),
            }),
            contributes: ContributionBlock {
                extension_points: vec![ExtensionPointContribution {
                    id: "permission.policy".to_string(),
                    title: "Permission Policy".to_string(),
                    contract: "contracts/permission.policy.v1.json".to_string(),
                    multiplicity: "many".to_string(),
                    activation: "withPlatformPolicy".to_string(),
                    allowed_contribution_kinds: vec!["policy".to_string()],
                }],
                ..ContributionBlock::default()
            },
            ai: PluginAiHints::default(),
        };
        let policy_plugin = PluginFormula {
            schema_version: "plugin-formula/v1".to_string(),
            id: "governance.high-risk-deny-policy".to_string(),
            kind: "child-plugin".to_string(),
            display_name: "High Risk Deny Policy".to_string(),
            version: "0.1.0".to_string(),
            intent: String::new(),
            entry: None,
            parent: Some(PluginParent {
                plugin_id: "platform.permission-core".to_string(),
                mount: "permission.policy".to_string(),
                compatible_parent_range: String::new(),
            }),
            activation: Vec::new(),
            platforms: None,
            capabilities: Vec::new(),
            contributes: ContributionBlock {
                policies: vec![super::PolicyContribution {
                    id: "policy.block-system-write".to_string(),
                    title: "Block System Write".to_string(),
                    effect: "deny".to_string(),
                    reason: "system write requires approval".to_string(),
                    priority: 20,
                    matches: super::PolicyMatchContribution {
                        source_ids: vec!["platform.capability-broker".to_string()],
                        source_kinds: vec!["system".to_string()],
                        capabilities: vec!["fs.write".to_string()],
                        scopes: vec!["/System".to_string()],
                        target_contains: Vec::new(),
                        platforms: Vec::new(),
                    },
                    when: String::new(),
                }],
                ..ContributionBlock::default()
            },
            events: EventBlock::default(),
            expectations: Vec::new(),
            ai: PluginAiHints::default(),
            child_policy: None,
        };

        let registry = compile_registry(vec![permission_core], vec![policy_plugin]);

        assert!(
            registry.diagnostics.is_empty(),
            "{:?}",
            registry.diagnostics
        );
        assert_eq!(registry.policies.len(), 1);
        assert_eq!(registry.policies[0].id, "policy.block-system-write");
        assert_eq!(
            registry.policies[0].parent_chain,
            vec!["platform.permission-core".to_string()]
        );
    }

    #[test]
    fn compile_registry_should_report_when_platform_and_capability_mismatches() {
        let plugin = PluginFormula {
            schema_version: "plugin-formula/v1".to_string(),
            id: "platform.when-check".to_string(),
            kind: "plugin".to_string(),
            display_name: "When Check".to_string(),
            version: "1.0.0".to_string(),
            intent: String::new(),
            entry: None,
            parent: None,
            activation: Vec::new(),
            platforms: Some(PlatformMatrix {
                supported: vec!["macos".to_string()],
                degraded: vec!["web".to_string()],
                unsupported: vec!["windows".to_string()],
                reason: String::new(),
            }),
            capabilities: vec![CapabilityFormula {
                id: "clipboard.write".to_string(),
                scope: "system-clipboard".to_string(),
                allow: Vec::new(),
                reason: String::new(),
                platforms: Vec::new(),
                optional: false,
            }],
            contributes: ContributionBlock {
                commands: vec![CommandContribution {
                    id: "when_check_command".to_string(),
                    title: "Command".to_string(),
                    category: "samples".to_string(),
                    capabilities: vec!["clipboard.write".to_string()],
                    when: "platform == 'windows' && capability('missing.cap')".to_string(),
                }],
                tools: vec![ToolContribution {
                    id: "when_check_tool".to_string(),
                    title: "Tool".to_string(),
                    category: "samples".to_string(),
                    input: String::new(),
                    output: String::new(),
                    capabilities: vec!["clipboard.write".to_string()],
                    when: "platform == 'windows'".to_string(),
                }],
                views: vec![ViewContribution {
                    id: "when_check_view".to_string(),
                    slot: "main".to_string(),
                    schema: "summary-list".to_string(),
                    contract: "schemas/plugin-ui-view.v1.schema.json".to_string(),
                    path: "/when-check".to_string(),
                    asset_item_kind: String::new(),
                    when: "platform == 'linux'".to_string(),
                }],
                ..ContributionBlock::default()
            },
            events: EventBlock::default(),
            expectations: Vec::new(),
            ai: PluginAiHints::default(),
            child_policy: None,
        };

        let registry = compile_registry(Vec::new(), vec![plugin]);

        assert!(registry.diagnostics.iter().any(|diagnostic| diagnostic
            .contains("command.when_check_command when requires platform windows")));
        assert!(registry
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(
                "command.when_check_command when references unknown capability missing.cap"
            )));
        assert!(registry
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic
                .contains("tool.when_check_tool when requires platform windows")));
        assert!(registry
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic
                .contains("view.when_check_view when requires platform linux")));
    }
}
