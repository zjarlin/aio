//! 物联网中心 AIO 子插件注册。

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
        APPLY_TEMPLATE_PATH, DEVICES_PATH, FIXTURE_TELEMETRY_PATH, OP_DEVICES_CREATE,
        OP_FIXTURE_TELEMETRY_INGEST, OP_TEMPLATE_APPLY, ROUTE, STATUS_PATH, UI_ACTION_PATH,
    },
    routes::{IotApiState, iot_router},
    service::IotService,
    state::install_service,
};

const PLUGIN_ID: &str = "iot-center";

/// 低代码驱动的物联网中心插件。
#[derive(Default)]
pub struct IotCenterPlugin;

impl NativePluginProvider for IotCenterPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor {
            id: PLUGIN_ID.to_string(),
            name: "物联网中心".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "产品、网关、设备、遥测和告警统一由低代码 RecordStore 驱动。".to_string(),
            activation: PluginActivation::Eager,
            priority: 500,
            dependencies: Vec::new(),
            capabilities: vec![
                "engine-meta-model".to_string(),
                "iot-device-health".to_string(),
                "postgres-persistence".to_string(),
            ],
            permissions: vec!["PostgreSQL engine_* 表读写".to_string()],
            kind: PluginKind::Native,
        }
    }

    fn contributions(&self) -> anyhow::Result<ContributionSet> {
        Ok(ContributionSet {
            backend_page: Some(BackendPageContribution {
                name: "iot".to_string(),
                title: "物联网中心".to_string(),
                route: ROUTE.to_string(),
            }),
            backend_apis: vec![
                api("iot.status", "GET", STATUS_PATH, "物联网状态", 10),
                api(
                    OP_TEMPLATE_APPLY,
                    "POST",
                    APPLY_TEMPLATE_PATH,
                    "初始化物联网模板",
                    20,
                ),
                api(OP_DEVICES_CREATE, "POST", DEVICES_PATH, "新建设备", 30),
                api(
                    OP_FIXTURE_TELEMETRY_INGEST,
                    "POST",
                    FIXTURE_TELEMETRY_PATH,
                    "接收模拟遥测",
                    40,
                ),
                api(
                    "iot.ui-action",
                    "POST",
                    UI_ACTION_PATH,
                    "物联网页面操作",
                    50,
                ),
            ],
            ..ContributionSet::default()
        })
    }

    fn admin_cli(&self) -> Vec<AdminCliContribution> {
        vec![
            AdminCliContribution {
                id: "iot.cli.template-apply".to_string(),
                label: "初始化物联网模板".to_string(),
                command: "az iot template apply --seed-demo".to_string(),
                resource_id: None,
                operation_id: Some(OP_TEMPLATE_APPLY.to_string()),
            },
            AdminCliContribution {
                id: "iot.cli.device-create".to_string(),
                label: "新建设备".to_string(),
                command: "az iot device create --request <device.json>".to_string(),
                resource_id: None,
                operation_id: Some(OP_DEVICES_CREATE.to_string()),
            },
        ]
    }

    fn runtime(&self, context: NativePluginContext) -> anyhow::Result<NativePluginRuntime> {
        let shared_db = context
            .shared_db
            .ok_or_else(|| anyhow!("物联网中心启动需要共享 PostgreSQL Db"))?;
        let store = RecordStore::from_shared_db(shared_db.shared_handle(), shared_db.pg_pool());
        let service = IotService::new(store);
        install_service(service.clone());
        Ok(NativePluginRuntime {
            router: iot_router(IotApiState::new(service)),
            startup: None,
        })
    }
}

#[Singleton(name = "iot-center")]
pub fn iot_center_plugin() -> DynAdminPluginProvider {
    Arc::new(IotCenterPlugin)
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
    fn plugin_contributes_iot_domain_and_engine_contracts() -> anyhow::Result<()> {
        let plugin = IotCenterPlugin;
        let contributions = plugin.contributions()?;
        assert!(plugin.descriptor().dependencies.is_empty());
        assert!(
            contributions
                .backend_apis
                .iter()
                .any(|api| api.id == OP_TEMPLATE_APPLY)
        );
        Ok(())
    }
}
