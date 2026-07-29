use az_plugin_core::plugin::{BackendApiContribution, ContributionSet, PluginDescriptor};
use az_plugin_core::{PluginActivation, PluginKind};

const PLUGIN_ID: &str = "algorithm-center";
pub const ROUTE: &str = "/algorithms";

pub fn descriptor() -> PluginDescriptor {
    PluginDescriptor {
        id: PLUGIN_ID.to_string(),
        name: "算法中心".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "浏览 az-algorithm 组件目录，查看输入输出契约与状态。".to_string(),
        activation: PluginActivation::Eager,
        priority: 880,
        dependencies: Vec::new(),
        capabilities: vec!["dioxus-ui-contract-page".to_string(), "axum-api".to_string()],
        permissions: vec!["read-algorithm-catalog".to_string()],
        kind: PluginKind::Native,
    }
}

pub fn contributions() -> ContributionSet {
    ContributionSet {
        backend_apis: vec![
            BackendApiContribution {
                id: "algorithm-center.api.status".to_string(),
                method: "GET".to_string(),
                path: "/api/algorithm-center/status".to_string(),
                label: "Algorithm Center Status".to_string(),
                description: "Reports component count.".to_string(),
                order: 10,
            },
            BackendApiContribution {
                id: "algorithm-center.api.components".to_string(),
                method: "GET".to_string(),
                path: "/api/algorithm-center/components".to_string(),
                label: "Algorithm Components".to_string(),
                description: "Returns the full algorithm component catalog as descriptors."
                    .to_string(),
                order: 20,
            },
            BackendApiContribution {
                id: "algorithm-center.api.process".to_string(),
                method: "POST".to_string(),
                path: "/api/algorithm-center/process".to_string(),
                label: "Process Video".to_string(),
                description:
                    "Accepts video_url plus algorithm codes and returns a processed video URL."
                        .to_string(),
                order: 30,
            },
            BackendApiContribution {
                id: "algorithm-center.api.upload".to_string(),
                method: "POST".to_string(),
                path: "/api/algorithm-center/upload".to_string(),
                label: "Upload Video".to_string(),
                description:
                    "Accepts multipart video upload and returns a video URL for processing."
                        .to_string(),
                order: 40,
            },
        ],
        catalog_providers: Vec::new(),
        ..ContributionSet::default()
    }
}
