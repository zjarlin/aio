use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::BundleManifest;

pub const MAX_BUNDLE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENCODED_BYTES: usize = 96 * 1024 * 1024;
pub(crate) const MAX_MANIFEST_BYTES: usize = 128 * 1024;
pub(crate) const MAX_FILES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub abi_version: u32,
    pub git: String,
    pub commit: String,
    pub version: String,
    pub manifest: String,
    pub files: BTreeMap<String, String>,
    pub digest: String,
}

#[derive(Debug)]
pub struct VerifiedBundle {
    pub(crate) manifest: BundleManifest,
    pub(crate) files: BTreeMap<String, Vec<u8>>,
    pub(crate) digest: String,
}

impl VerifiedBundle {
    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn manifest(&self) -> &BundleManifest {
        &self.manifest
    }

    pub fn component(&self) -> &[u8] {
        &self.files[&self.manifest.plugin.runtime.artifact]
    }

    pub fn frontend(&self, entry: &str) -> Option<&[u8]> {
        crate::validate_relative_path(entry).ok()?;
        self.files
            .get(&format!("{}/{entry}", self.manifest.plugin.frontend.path))
            .map(Vec::as_slice)
    }

    pub fn migrations(&self) -> impl Iterator<Item = (&str, &str)> {
        let prefix = self
            .manifest
            .plugin
            .database
            .as_ref()
            .map(|database| format!("{}/", database.migrations));
        self.files.iter().filter_map(move |(path, bytes)| {
            let relative = path.strip_prefix(prefix.as_deref()?)?;
            Some((relative, std::str::from_utf8(bytes).ok()?))
        })
    }
}
