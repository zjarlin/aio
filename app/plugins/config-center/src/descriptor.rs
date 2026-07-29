use az_plugin_core::plugin::{
    BackendApiContribution, ContributionSet, PluginDescriptor,
};
use az_plugin_core::{PluginActivation, PluginKind};

pub const PLUGIN_ID: &str = "config-center";
pub const ROUTE: &str = "/config";

pub fn descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: PLUGIN_ID.to_string(),
        name: "配置中心".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Dotfiles monitor, pairing identity, XDG paths, and provider configuration."
            .to_string(),
        activation: PluginActivation::Eager,
        priority: 910,
        dependencies: Vec::new(),
        capabilities: vec![
            "dioxus-ui-contract-page".to_string(),
            "axum-api".to_string(),
            "toasty-persistence".to_string(),
            "dotfiles-monitor".to_string(),
            "pairing".to_string(),
        ],
        permissions: vec![
            "read-write xdg config".to_string(),
            "read dotfiles root".to_string(),
            "postgres-read-write".to_string(),
        ],
        kind: PluginKind::Native,
    }
}

pub fn contributions() -> ContributionSet {
    ContributionSet {
        backend_apis: vec![
            backend_api(
                "config-center.api.status",
                "GET",
                "/api/config-center/status",
                "Config Center Status",
                "Reports runtime, database URL availability, table prefix, and XDG paths.",
                10,
            ),
            backend_api(
                "config-center.api.dotfiles",
                "GET",
                "/api/config-center/dotfiles",
                "Dotfiles Status",
                "Scans dotfiles status and conflicts.",
                20,
            ),
            backend_api(
                "config-center.api.pairing",
                "GET",
                "/api/config-center/pairing",
                "Pairing Identity",
                "Returns current machine pairing identity.",
                30,
            ),
            backend_api(
                "config-center.api.entries",
                "GET",
                "/api/config-center/entries",
                "Config Entries",
                "Lists persisted config entries by namespace.",
                40,
            ),
            backend_api(
                "config-center.api.entry-upsert",
                "POST",
                "/api/config-center/entry",
                "Save Config Entry",
                "Creates or updates a config entry.",
                50,
            ),
        ],
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
