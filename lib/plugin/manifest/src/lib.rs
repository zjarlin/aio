#![forbid(unsafe_code)]

mod model;
#[cfg(feature = "validation")]
mod validation;

pub use model::{
    CapabilityManifest, PageBody, PageDefinition, PluginManifest, PluginRuntime,
    RepositoryManifest, RepositoryPackage, RuntimeManifest, SceneDefinition, SubpluginManifest,
};
#[cfg(feature = "validation")]
pub use validation::{
    ValidationReport, artifact_path, parse_manifest, read_manifest, validate_manifest,
    validate_page_definitions, validate_repository, validate_wasm_component,
};
