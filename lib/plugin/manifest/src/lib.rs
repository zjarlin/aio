#![forbid(unsafe_code)]

mod model;
#[cfg(feature = "schema")]
mod schema;
#[cfg(feature = "validation")]
mod validation;
#[cfg(feature = "validation")]
mod wasm_component;

pub use model::{
    CapabilityManifest, ComponentResponse, PageActionDefinition, PageActionResult, PageBody,
    PageDefinition, PluginManifest, PluginRequest, PluginRuntime, RepositoryManifest,
    RepositoryPackage, RuntimeManifest, SceneDefinition, SubpluginManifest,
};
#[cfg(feature = "schema")]
pub use schema::{PluginSchema, schemas};
#[cfg(feature = "validation")]
pub use validation::{
    ValidationReport, artifact_path, parse_manifest, read_manifest, validate_declared_pages,
    validate_host_compatibility, validate_manifest, validate_page_definitions, validate_repository,
};
#[cfg(feature = "validation")]
pub use wasm_component::validate_wasm_component;
