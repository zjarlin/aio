use az_plugin_core::plugin::{BackendApiContribution, ContributionSet, PluginDescriptor};
use az_plugin_core::{PluginActivation, PluginKind};

pub const PLUGIN_ID: &str = "asset-hub";
pub const ROUTE: &str = "/assets";

pub fn descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: PLUGIN_ID.to_string(),
        name: "资产中心".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "资产提要、技能扫描、合成资产和子类型元数据。".to_string(),
        activation: PluginActivation::Eager,
        priority: 920,
        dependencies: Vec::new(),
        capabilities: vec![
            "dioxus-ui-contract-page".to_string(),
            "axum-api".to_string(),
            "toasty-persistence".to_string(),
            "skill-scan".to_string(),
        ],
        permissions: vec![
            "read ~/.agents/skills".to_string(),
            "postgres-read-write".to_string(),
        ],
        kind: PluginKind::Native,
    }
}

pub fn contributions() -> ContributionSet {
    ContributionSet {
        backend_apis: vec![
            backend_api(
                "asset-hub.api.status",
                "GET",
                "/api/asset-hub/status",
                "Asset Hub Status",
                "Reports runtime, database URL availability, and table prefix.",
                10,
            ),
            backend_api(
                "asset-hub.api.skills",
                "GET",
                "/api/asset-hub/skills",
                "Scanned Skills",
                "Scans skill directories and returns skill assets.",
                20,
            ),
            backend_api(
                "asset-hub.api.assets",
                "GET",
                "/api/asset-hub/assets",
                "Asset List",
                "Lists persisted Asset Hub records.",
                30,
            ),
            backend_api(
                "asset-hub.api.asset-upsert",
                "POST",
                "/api/asset-hub/asset",
                "Save Asset",
                "Creates or updates one Asset Hub record.",
                40,
            ),
        ],
        catalog_providers: Vec::new(),
        ..ContributionSet::default()
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
        id: id.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        order,
    }
}
