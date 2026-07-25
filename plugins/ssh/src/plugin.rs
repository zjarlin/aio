//! SSH 服务器运维 AIO 子插件注册。

use std::sync::Arc;

use anyhow::anyhow;
use az_aio_nature_generated::enums::{
    AdminMenuNodeKind, PluginActivation, PluginKind, UiContributionSlot,
};
use az_aio_platform::plugin::contract::{
    AdminCliContribution, AdminMenuNode, AdminMenuSection, AdminMenuTree, BackendApiContribution,
    ContributionSet, DynAdminPluginProvider, NativePluginContext, NativePluginProvider,
    NativePluginRuntime, NativeUiRenderer, NavItemContribution, PageContribution, PluginDependency,
    PluginDescriptor, UiContribution,
};
use az_engine::EngineStore;
use rudi::Singleton;

use crate::{
    contract::{
        APPLY_TEMPLATE_PATH, COLLECT_PATH, COMMANDS_PATH, EXECUTE_PATH, OP_COLLECT,
        OP_COMMAND_UPSERT, OP_EXECUTE, OP_TARGET_UPSERT, OP_TEMPLATE_APPLY, ROUTE, STATUS_PATH,
        TARGETS_PATH,
    },
    routes::{SshApiState, ssh_router},
    service::SshService,
    state::install_service,
    ui::SshOperationsPage,
};

const PLUGIN_ID: &str = "ssh";
const RENDERER_ID: &str = "ssh.page";
const DOMAIN_ID: &str = "server-operations";

/// 低代码驱动的 SSH 服务器运维插件。
#[derive(Default)]
pub struct SshPlugin;

impl NativePluginProvider for SshPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: PLUGIN_ID.to_string(),
            name: "SSH 服务器运维".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "通过低代码命令模型适配不同服务器硬件并执行 SSH 监测。".to_string(),
            activation: PluginActivation::Eager,
            priority: 520,
            dependencies: vec![PluginDependency {
                id: "lowcode".to_string(),
                optional: false,
            }],
            capabilities: vec![
                "dioxus-ui-contract-page".to_string(),
                "engine-meta-model".to_string(),
                "ssh-server-operations".to_string(),
                "hardware-adaptive-monitoring".to_string(),
                "postgres-persistence".to_string(),
            ],
            permissions: vec![
                "PostgreSQL engine_* 表读写".to_string(),
                "使用配置的认证材料连接远程 SSH 服务器".to_string(),
                "执行管理员配置的远程监测命令".to_string(),
            ],
            kind: PluginKind::Native,
        }
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(ContributionSet {
            nav_items: vec![NavItemContribution {
                id: "ssh.nav".to_string(),
                label: "SSH 运维".to_string(),
                icon: ">_".to_string(),
                route: ROUTE.to_string(),
                order: 60,
            }],
            pages: vec![PageContribution {
                route: ROUTE.to_string(),
                title: "SSH 服务器运维".to_string(),
                subtitle: "跨硬件监测与低代码命令".to_string(),
                renderer_id: RENDERER_ID.to_string(),
                placeholder_mark: ">_".to_string(),
                order: 60,
            }],
            ui_contributions: vec![UiContribution {
                id: "ssh.ui.content".to_string(),
                slot: UiContributionSlot::Content,
                label: "SSH 服务器运维内容区".to_string(),
                renderer_id: RENDERER_ID.to_string(),
                route: Some(ROUTE.to_string()),
                order: 60,
            }],
            backend_apis: vec![
                api("ssh.status", "GET", STATUS_PATH, "SSH 运维状态", 10),
                api(
                    OP_TEMPLATE_APPLY,
                    "POST",
                    APPLY_TEMPLATE_PATH,
                    "初始化 SSH 低代码模板",
                    20,
                ),
                api(OP_TARGET_UPSERT, "POST", TARGETS_PATH, "保存 SSH 目标", 30),
                api(
                    OP_COMMAND_UPSERT,
                    "POST",
                    COMMANDS_PATH,
                    "保存 SSH 命令",
                    40,
                ),
                api(OP_COLLECT, "POST", COLLECT_PATH, "采集目标监测项", 50),
                api(OP_EXECUTE, "POST", EXECUTE_PATH, "执行指定 SSH 命令", 60),
            ],
            ..ContributionSet::default()
        })
    }

    fn admin_menu(&self, _contributions: &ContributionSet) -> AdminMenuTree {
        AdminMenuTree {
            sections: vec![AdminMenuSection {
                domain_id: DOMAIN_ID.to_string(),
                label: "服务器运维".to_string(),
                default_href: "/ssh?view=overview".to_string(),
                order: 350,
                menus: vec![AdminMenuNode {
                    id: "ssh.operations".to_string(),
                    kind: AdminMenuNodeKind::Branch,
                    label: "SSH 运维".to_string(),
                    href: ROUTE.to_string(),
                    icon: ">_".to_string(),
                    order: 10,
                    active_patterns: vec![ROUTE.to_string()],
                    permissions_any_of: vec!["ssh:read".to_string()],
                    children: vec![
                        menu("ssh.overview", "监测总览", "overview", "#", 10),
                        menu("ssh.targets", "SSH 目标", "targets", "@", 20),
                        menu("ssh.commands", "监测命令", "commands", ">", 30),
                        menu("ssh.results", "执行结果", "results", "=", 40),
                    ],
                }],
            }],
        }
    }

    fn admin_cli(&self) -> Vec<AdminCliContribution> {
        vec![
            AdminCliContribution {
                id: "ssh.cli.template-apply".to_string(),
                label: "初始化 SSH 命令模板".to_string(),
                command: "az ssh template apply --seed-builtins".to_string(),
                resource_id: None,
                operation_id: Some(OP_TEMPLATE_APPLY.to_string()),
            },
            AdminCliContribution {
                id: "ssh.cli.collect".to_string(),
                label: "执行 SSH 监测采集".to_string(),
                command: "az ssh collect --target <target-code>".to_string(),
                resource_id: None,
                operation_id: Some(OP_COLLECT.to_string()),
            },
        ]
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let shared_db = context
            .shared_db
            .ok_or_else(|| anyhow!("SSH 服务器运维启动需要共享 PostgreSQL Db"))?;
        let store = EngineStore::from_shared_db(shared_db.shared_handle());
        let service = SshService::new(store);
        install_service(service.clone());
        Ok(NativePluginRuntime {
            renderers: vec![NativeUiRenderer {
                renderer_id: RENDERER_ID.to_string(),
                slot: UiContributionSlot::Content,
                route: Some(ROUTE.to_string()),
                render: SshOperationsPage,
            }],
            router: ssh_router(SshApiState::new(service)),
            startup: None,
        })
    }
}

#[Singleton(name = "ssh")]
pub fn ssh_plugin() -> DynAdminPluginProvider {
    Arc::new(SshPlugin)
}

fn api(id: &str, method: &str, path: &str, label: &str, order: i32) -> BackendApiContribution {
    BackendApiContribution {
        id: id.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        label: label.to_string(),
        description: format!("{label}: {path}"),
        order,
    }
}

fn menu(id: &str, label: &str, view: &str, icon: &str, order: i32) -> AdminMenuNode {
    let href = format!("/ssh?view={view}");
    AdminMenuNode {
        id: id.to_string(),
        kind: AdminMenuNodeKind::Page,
        label: label.to_string(),
        href: href.clone(),
        icon: icon.to_string(),
        order,
        active_patterns: vec![href],
        permissions_any_of: vec!["ssh:read".to_string()],
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_contributes_server_operations_context_and_lowcode_dependency() -> anyhow::Result<()> {
        let plugin = SshPlugin;
        let contributions = plugin.contributions()?;
        let menu = plugin.admin_menu(&contributions);

        assert_eq!(menu.sections[0].domain_id, "server-operations");
        assert_eq!(menu.sections[0].label, "服务器运维");
        assert!(
            plugin
                .descriptor()
                .dependencies
                .iter()
                .any(|dependency| dependency.id == "lowcode" && !dependency.optional)
        );
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == COLLECT_PATH)
        );
        Ok(())
    }
}
