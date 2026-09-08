use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryManifest {
    pub plugin: PluginManifest,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    #[serde(default)]
    pub client: Option<RepositoryPackage>,
    #[serde(default)]
    pub server: Option<RepositoryPackage>,
    #[serde(default)]
    pub runtime: Option<RuntimeManifest>,
    #[serde(default)]
    pub capabilities: CapabilityManifest,
    #[serde(default)]
    pub subplugins: Vec<SubpluginManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryPackage {
    #[serde(default)]
    pub runtime: PluginRuntime,
    #[serde(default = "current_directory")]
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeManifest {
    pub kind: PluginRuntime,
    pub artifact: String,
    #[serde(default)]
    pub host_version: Option<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub health_check: Option<String>,
    #[serde(default)]
    pub shutdown_timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginRuntime {
    #[default]
    RustSource,
    PageDefinition,
    WasmComponent,
    Process,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityManifest {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub filesystem: Vec<String>,
    #[serde(default)]
    pub database: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubpluginManifest {
    pub id: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub account_actions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageDefinition {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub scene: SceneDefinition,
    #[serde(default)]
    pub required_permission: Option<String>,
    pub body: PageBody,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDefinition {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PageBody {
    Counter { title: String, button: String },
    Text { title: String, content: String },
}

fn current_directory() -> String {
    ".".to_owned()
}
