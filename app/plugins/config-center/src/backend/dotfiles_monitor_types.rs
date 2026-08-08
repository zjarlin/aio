#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DotfilesBaselineEntry {
    pub relative_path: String,
    pub content: String,
    pub repo_modified: u64,
    pub home_modified: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesPeerDeviceInput {
    pub id: String,
    pub name: String,
    pub home_path: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesDevicesRequest {
    pub devices: Vec<DotfilesPeerDeviceInput>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDotfilesConflictRequest {
    pub conflict_id: String,
    pub strategy: String,
    pub merged_text: Option<String>,
}
