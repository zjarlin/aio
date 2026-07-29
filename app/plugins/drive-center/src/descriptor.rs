use az_plugin_core::plugin::{BackendApiContribution, ContributionSet, PluginDescriptor};
use az_plugin_core::{PluginActivation, PluginKind};

pub const PLUGIN_ID: &str = "drive-center";
pub const ROUTE: &str = "/drive";

pub fn descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: PLUGIN_ID.to_string(),
        name: "网盘中心".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Drive hosting, tracked items, queue, conflicts, and root aliases."
            .to_string(),
        activation: PluginActivation::Eager,
        priority: 900,
        dependencies: Vec::new(),
        capabilities: vec![
            "dioxus-ui-contract-page".to_string(),
            "axum-api".to_string(),
            "toasty-persistence".to_string(),
            "drive-queue".to_string(),
        ],
        permissions: vec![
            "postgres-read-write".to_string(),
            "read hosted paths".to_string(),
        ],
        kind: PluginKind::Native,
    }
}

pub fn contributions() -> ContributionSet {
    ContributionSet {
        backend_apis: vec![
            backend_api(
                "drive-center.api.status",
                "GET",
                "/api/drive-center/status",
                "Drive Center Status",
                "Reports runtime, database URL availability, and table prefix.",
                10,
            ),
            backend_api(
                "drive-center.api.tasks",
                "GET",
                "/api/drive-center/tasks",
                "Drive Task List",
                "Lists queued and tracked drive tasks.",
                20,
            ),
            backend_api(
                "drive-center.api.task-enqueue",
                "POST",
                "/api/drive-center/task",
                "Enqueue Drive Task",
                "Adds a drive sync/host/unhost task.",
                30,
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
