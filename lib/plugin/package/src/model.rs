use std::collections::BTreeMap;

use az_plugin_manifest::RepositoryManifest;
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 2;
pub const PACKAGE_CONTENT_TYPE: &str = "application/vnd.aio.plugin+gzip";
pub const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 128 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 48 * 1024 * 1024;
pub const MAX_PACKAGE_JSON_BYTES: usize = 48 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginPackage {
    pub format_version: u32,
    pub git: String,
    pub version: String,
    pub source_revision: Option<String>,
    pub manifest_toml: String,
    pub artifact_base64: String,
    pub artifact_sha256: String,
    pub frontend: BTreeMap<String, FrontendAsset>,
    pub rev: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendAsset {
    pub content_base64: String,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub struct VerifiedPluginPackage {
    pub manifest: RepositoryManifest,
    pub artifact: Vec<u8>,
    pub frontend: BTreeMap<String, Vec<u8>>,
}
