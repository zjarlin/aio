//! SSH 服务器运维 AIO 子插件注册。

use std::sync::Arc;

use anyhow::anyhow;
use az_plugin_core::RecordStore;
use az_plugin_core::plugin::{
    AdminCliContribution, BackendApiContribution, BackendPageContribution, ContributionSet,
    DynAdminPluginProvider, NativePluginContext, NativePluginProvider, NativePluginRuntime,
    PluginDescriptor,
};
use az_plugin_core::{PluginActivation, PluginKind};
use rudi::Singleton;

use crate::{
    contract::{
        APPLY_TEMPLATE_PATH, COLLECT_PATH, COMMANDS_PATH, EXECUTE_PATH, OP_COLLECT,
        OP_COMMAND_UPSERT, OP_EXECUTE, OP_TARGET_UPSERT, OP_TEMPLATE_APPLY, ROUTE, STATUS_PATH,
        TARGETS_PATH, UI_ACTION_PATH,
    },
    routes::{SshApiState, ssh_router},
    service::SshService,
    state::install_service,
};

const PLUGIN_ID: &str = "ssh";

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
            dependencies: Vec::new(),
            capabilities: vec![
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
            backend_page: Some(BackendPageContribution {
                name: "ssh".to_string(),
                title: "SSH 服务器运维".to_string(),
                route: ROUTE.to_string(),
            }),
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
                api("ssh.ui-action", "POST", UI_ACTION_PATH, "SSH 页面操作", 70),
            ],
            ..ContributionSet::default()
        })
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
        let store = RecordStore::from_shared_db(shared_db.shared_handle(), shared_db.pg_pool());
        let service = SshService::new(store);
        install_service(service.clone());
        Ok(NativePluginRuntime {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_contributes_server_backend_apis() -> anyhow::Result<()> {
        let plugin = SshPlugin;
        let contributions = plugin.contributions()?;
        assert!(plugin.descriptor().dependencies.is_empty());
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.path == COLLECT_PATH)
        );
        Ok(())
    }
}
