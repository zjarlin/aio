use az_plugin_core::plugin::{
    BackendApiContribution, BackendPageContribution, ContributionSet, PluginDescriptor,
};
use az_plugin_core::{PluginActivation, PluginKind};

pub const PLUGIN_ID: &str = "software-center";
pub const ROUTE: &str = "/software";

pub fn descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: PLUGIN_ID.to_string(),
        name: "软件中心".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description:
            "Installer scan, organize/archive, and catalog-linked package detail surfaces."
                .to_string(),
        activation: PluginActivation::Eager,
        priority: 880,
        dependencies: Vec::new(),
        capabilities: vec![
            "dioxus-ui-contract-page".to_string(),
            "axum-api".to_string(),
            "toasty-persistence".to_string(),
            "installer-scan".to_string(),
        ],
        permissions: vec![
            "read Downloads and Desktop".to_string(),
            "write installer archive".to_string(),
            "postgres-read-write".to_string(),
        ],
        kind: PluginKind::Native,
    }
}

pub fn contributions() -> ContributionSet {
    ContributionSet {
        backend_page: Some(BackendPageContribution {
            name: "software".to_string(),
            title: "软件中心".to_string(),
            route: ROUTE.to_string(),
        }),
        backend_apis: vec![
            backend_api(
                "software-center.api.status",
                "GET",
                "/api/software-center/status",
                "Software Center Status",
                "Reports runtime, database URL availability, and table prefix.",
                10,
            ),
            backend_api(
                "software-center.api.installers",
                "GET",
                "/api/software-center/installers",
                "Installer Scan",
                "Scans Downloads and Desktop for installer packages.",
                20,
            ),
            backend_api(
                "software-center.api.organize",
                "POST",
                "/api/software-center/organize",
                "Organize Installers",
                "Archives detected installers into Software Center storage.",
                30,
            ),
            backend_api(
                "software-center.api.packages",
                "GET",
                "/api/software-center/packages",
                "Software Packages",
                "Lists persisted software package records.",
                40,
            ),
            backend_api(
                "software-center.api.package-upsert",
                "POST",
                "/api/software-center/package",
                "Save Software Package",
                "Creates or updates a software package record.",
                50,
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
