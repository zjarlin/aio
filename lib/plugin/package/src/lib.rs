#![forbid(unsafe_code)]

mod archive;
mod frontend;
mod integrity;
mod model;

pub use integrity::normalize_git_source;
pub use model::{
    FORMAT_VERSION, FrontendAsset, MAX_ARTIFACT_BYTES, MAX_MANIFEST_BYTES, MAX_PACKAGE_BYTES,
    MAX_PACKAGE_JSON_BYTES, PACKAGE_CONTENT_TYPE, PluginPackage, VerifiedPluginPackage,
};

#[cfg(test)]
mod tests;
