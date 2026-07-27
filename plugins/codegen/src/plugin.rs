//! nature-compiler 工作台注册与能力聚合。

use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use az_aio_nature_generated::enums::{
    AdminFieldKind, AdminMenuNodeKind, PluginActivation, PluginKind, UiContributionSlot,
};
use az_aio_platform::plugin::contract::{
    AdminCliContribution, AdminFieldContract, AdminMenuNode, AdminMenuSection, AdminMenuTree,
    AdminResourceContract, BackendApiContribution, ContributionSet, DynAdminPluginProvider,
    NativePluginContext, NativePluginProvider, NativePluginRuntime, NativeUiRenderer,
    NavItemContribution, PageContribution, PluginDescriptor, UiContribution,
};
use nature_compiler::{
    AppliedDefault, Blueprint, CapabilityCatalog, CapabilityProvider, Compiler, CompilerCatalog,
    Diagnostic, FixtureMapProvider, SemanticDescriptor,
};
use rudi::Singleton;
use serde_json::Value;

use crate::{
    contract::{
        OP_REVISION_CREATE, OP_REVISION_GET, OP_REVISION_PUBLISH, PROJECT_REVISIONS_PATH,
        REVISION_PATH, REVISION_PUBLISH_PATH,
    },
    inference_agent::NatureInferenceAgent,
    routes::{NatureApiState, nature_router},
    service::NatureService,
    store::NatureStore,
    ui::NatureCompilerPage,
};

const PLUGIN_ID: &str = "nature-compiler";
const ROUTE: &str = "/nature";
const RENDERER_ID: &str = "nature-compiler.page";

/// Rudi 注册的模拟 Map 能力。
#[derive(Clone, Copy, Debug, Default)]
#[Singleton(name = "nature-fixture-map", binds = [bind_capability::<FixtureMapCapability>])]
pub struct FixtureMapCapability;

impl CapabilityProvider for FixtureMapCapability {
    fn descriptor(&self) -> SemanticDescriptor {
        FixtureMapProvider.descriptor()
    }

    fn aliases(&self) -> &'static [&'static str] {
        FixtureMapProvider.aliases()
    }

    fn config_schema(&self) -> Value {
        FixtureMapProvider.config_schema()
    }

    fn defaults(&self) -> Vec<AppliedDefault> {
        FixtureMapProvider.defaults()
    }

    fn validate(&self, blueprint: &Blueprint) -> Vec<Diagnostic> {
        FixtureMapProvider.validate(blueprint)
    }

    fn lower(&self, blueprint: &mut Blueprint) -> anyhow::Result<()> {
        FixtureMapProvider.lower(blueprint)
    }
}

fn bind_capability<T>(provider: T) -> Arc<dyn CapabilityProvider>
where
    T: CapabilityProvider + 'static,
{
    Arc::new(provider)
}

/// 由母语 revision 驱动的代码生成插件。
pub struct CodegenPlugin {
    capabilities: Vec<Arc<dyn CapabilityProvider>>,
}

impl CodegenPlugin {
    pub fn new(capabilities: Vec<Arc<dyn CapabilityProvider>>) -> Self {
        Self { capabilities }
    }
}

impl Default for CodegenPlugin {
    fn default() -> Self {
        Self::new(vec![Arc::new(FixtureMapCapability)])
    }
}

impl NativePluginProvider for CodegenPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: PLUGIN_ID.to_string(),
            name: "nature-compiler".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "把母语需求编译为受控 Blueprint 和确定性 Rust。".to_string(),
            activation: PluginActivation::Eager,
            priority: 610,
            dependencies: Vec::new(),
            capabilities: vec![
                "mother-tongue-blueprint".to_string(),
                "deterministic-rust".to_string(),
                "postgres-persistence".to_string(),
            ],
            permissions: vec!["本机生成目录写入".to_string()],
            kind: PluginKind::Native,
        }
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(ContributionSet {
            nav_items: vec![NavItemContribution {
                id: "nature.nav".to_string(),
                label: "自然编译".to_string(),
                icon: "⌘".to_string(),
                route: ROUTE.to_string(),
                order: 60,
            }],
            pages: vec![PageContribution {
                route: ROUTE.to_string(),
                title: "nature-compiler".to_string(),
                subtitle: "母语 Blueprint 与 Rust 生成门禁".to_string(),
                renderer_id: RENDERER_ID.to_string(),
                placeholder_mark: "⌘".to_string(),
                order: 60,
            }],
            ui_contributions: vec![UiContribution {
                id: "nature.ui.content".to_string(),
                slot: UiContributionSlot::Content,
                label: "自然编译内容区".to_string(),
                renderer_id: RENDERER_ID.to_string(),
                route: Some(ROUTE.to_string()),
                order: 60,
            }],
            backend_apis: vec![
                backend_api(
                    OP_REVISION_CREATE,
                    "POST",
                    PROJECT_REVISIONS_PATH,
                    "提交母语 revision",
                    10,
                ),
                backend_api(OP_REVISION_GET, "GET", REVISION_PATH, "读取 revision", 20),
                backend_api(
                    OP_REVISION_PUBLISH,
                    "POST",
                    REVISION_PUBLISH_PATH,
                    "发布 revision",
                    30,
                ),
            ],
            ..ContributionSet::default()
        })
    }

    fn admin_menu(&self, _contributions: &ContributionSet) -> AdminMenuTree {
        AdminMenuTree {
            sections: vec![AdminMenuSection {
                domain_id: "lowcode".to_string(),
                label: "低代码".to_string(),
                default_href: String::new(),
                order: 600,
                menus: vec![AdminMenuNode {
                    id: "nature.root".to_string(),
                    kind: AdminMenuNodeKind::Page,
                    label: "自然编译".to_string(),
                    href: ROUTE.to_string(),
                    icon: "⌘".to_string(),
                    order: 20,
                    active_patterns: vec![ROUTE.to_string()],
                    permissions_any_of: vec!["nature:write".to_string()],
                    children: Vec::new(),
                }],
            }],
        }
    }

    fn admin_cli(&self) -> Vec<AdminCliContribution> {
        vec![AdminCliContribution {
            id: "nature.cli.revision-create".to_string(),
            label: "提交母语 revision".to_string(),
            command: "az nature revision create --source <需求.txt>".to_string(),
            resource_id: None,
            operation_id: Some(OP_REVISION_CREATE.to_string()),
        }]
    }

    fn admin_resources(&self) -> Vec<AdminResourceContract> {
        vec![
            nature_resource(
                "nature.projects",
                "母语项目",
                "nature_projects",
                vec![
                    admin_field("native_name", "母语名称", AdminFieldKind::Text, true),
                    admin_field("updated_at_ms", "更新时间", AdminFieldKind::Time, false),
                ],
            ),
            nature_resource(
                "nature.revisions",
                "编译版本",
                "nature_revisions",
                vec![
                    admin_field("project_id", "项目", AdminFieldKind::Relation, true),
                    admin_field("source_text", "母语源码", AdminFieldKind::Json, true),
                    admin_field("status", "状态", AdminFieldKind::Badge, true),
                    admin_field("artifact_hash", "产物摘要", AdminFieldKind::Text, false),
                ],
            ),
            nature_resource(
                "nature.runs",
                "生成运行",
                "nature_generation_runs",
                vec![
                    admin_field("revision_id", "编译版本", AdminFieldKind::Relation, true),
                    admin_field("stage", "阶段", AdminFieldKind::Badge, true),
                    admin_field("status", "状态", AdminFieldKind::Badge, true),
                ],
            ),
            nature_resource(
                "nature.events",
                "生成事件",
                "nature_generation_events",
                vec![
                    admin_field("run_id", "生成运行", AdminFieldKind::Relation, true),
                    admin_field("stage", "阶段", AdminFieldKind::Badge, true),
                    admin_field("status", "状态", AdminFieldKind::Badge, true),
                    admin_field("duration_ms", "耗时", AdminFieldKind::Number, false),
                    admin_field("message", "消息", AdminFieldKind::Text, false),
                ],
            ),
            nature_resource(
                "nature.field-bindings",
                "字段绑定",
                "engine_field_bindings",
                vec![
                    admin_field("project_id", "母语项目", AdminFieldKind::Relation, true),
                    admin_field(
                        "owner_model_code",
                        "产品或设备模型",
                        AdminFieldKind::Text,
                        true,
                    ),
                    admin_field("field_code", "字段", AdminFieldKind::Text, true),
                    admin_field("source_name", "原始数据", AdminFieldKind::Text, true),
                    admin_field("transform_json", "值获取逻辑", AdminFieldKind::Json, true),
                ],
            ),
        ]
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let shared_db = context
            .shared_db
            .ok_or_else(|| anyhow!("nature-compiler 启动需要共享 PostgreSQL Db"))?;
        let store = NatureStore::new(shared_db.shared_handle());
        let dictionary_store =
            az_aio_platform::system::store::SystemAdminStore::from_shared(shared_db);
        let compiler = Compiler::new(
            Arc::new(NatureInferenceAgent::from_env()?),
            CompilerCatalog::new(
                CapabilityCatalog::new(self.capabilities.clone()),
                Vec::new(),
                Vec::new(),
            ),
        );
        let output_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
        let service = NatureService::new(store, compiler, output_root, dictionary_store);
        let recovery_service = service.clone();
        tokio::spawn(async move {
            if let Err(error) = recovery_service.resume_incomplete().await {
                tracing::error!(error = %error, "恢复 nature 生成任务失败");
            }
        });
        Ok(NativePluginRuntime {
            renderers: vec![NativeUiRenderer {
                renderer_id: RENDERER_ID.to_string(),
                slot: UiContributionSlot::Content,
                route: Some(ROUTE.to_string()),
                render: NatureCompilerPage,
            }],
            router: nature_router(NatureApiState::new(service)),
            startup: None,
        })
    }
}

fn nature_resource(
    id: &str,
    label: &str,
    table_name: &str,
    fields: Vec<AdminFieldContract>,
) -> AdminResourceContract {
    AdminResourceContract {
        id: id.to_string(),
        label: label.to_string(),
        description: format!("{label} 的 PostgreSQL 正式数据"),
        route: ROUTE.to_string(),
        table_name: table_name.to_string(),
        permissions_any_of: vec!["nature:read".to_string()],
        fields,
        operations: Vec::new(),
    }
}

fn admin_field(
    name: &str,
    label: &str,
    kind: AdminFieldKind,
    required: bool,
) -> AdminFieldContract {
    AdminFieldContract {
        name: name.to_string(),
        label: label.to_string(),
        kind,
        required,
        searchable: matches!(kind, AdminFieldKind::Text | AdminFieldKind::Badge),
        table_visible: true,
        form_visible: false,
    }
}

#[Singleton(name = "nature-compiler")]
pub fn codegen_plugin(
    #[di(vec)] capabilities: Vec<Arc<dyn CapabilityProvider>>,
) -> DynAdminPluginProvider {
    Arc::new(CodegenPlugin::new(capabilities))
}

fn backend_api(
    id: &str,
    method: &str,
    path: &str,
    label: &str,
    order: i32,
) -> BackendApiContribution {
    BackendApiContribution {
        id: id.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        label: label.to_string(),
        description: format!("{label}: {path}"),
        order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_exposes_only_nature_revision_operations() -> anyhow::Result<()> {
        let plugin = CodegenPlugin::default();
        let contributions = plugin.contributions()?;

        assert!(contributions.pages.iter().any(|page| page.route == ROUTE));
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.id == OP_REVISION_CREATE)
        );
        assert!(
            contributions
                .backend_apis
                .iter()
                .all(|api| !api.path.starts_with("/api/codegen"))
        );
        Ok(())
    }

    #[test]
    fn capability_catalog_accepts_rudi_style_extension_without_compiler_changes() {
        let providers: Vec<Arc<dyn CapabilityProvider>> = vec![Arc::new(FixtureMapCapability)];
        let catalog = CapabilityCatalog::new(providers);

        assert!(catalog.resolve("模拟采集", "模拟采集提供原值").is_ok());
    }
}
