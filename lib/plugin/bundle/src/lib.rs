#![forbid(unsafe_code)]

mod archive;
mod directory;
mod manifest;
mod model;
mod validation;

pub use manifest::{
    BundleManifest, ComponentManifest, DatabaseManifest, FrontendManifest, RuntimeManifest,
};
pub use model::{Bundle, MAX_BUNDLE_BYTES, MAX_ENCODED_BYTES, VerifiedBundle};
pub use validation::validate_relative_path;

#[cfg(test)]
mod tests;
