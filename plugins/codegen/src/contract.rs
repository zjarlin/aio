//! nature-compiler 工作台的共享操作契约。

use serde::{Deserialize, Serialize};

pub const PROJECT_REVISIONS_PATH: &str = "/api/nature/projects/{project_id}/revisions";
pub const REVISION_PATH: &str = "/api/nature/revisions/{revision_id}";
pub const REVISION_PUBLISH_PATH: &str = "/api/nature/revisions/{revision_id}/publish";
pub const UI_ACTION_PATH: &str = "/api/nature/ui-action";
pub const OP_REVISION_CREATE: &str = "nature.revisions.create";
pub const OP_REVISION_GET: &str = "nature.revisions.get";
pub const OP_REVISION_PUBLISH: &str = "nature.revisions.publish";

/// 产品端唯一可提交的 revision 输入。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNatureRevisionRequest {
    pub source_text: String,
}

/// 异步任务接收结果。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedNatureRevision {
    pub revision_id: String,
    pub status: String,
}

/// 生成文件的可审查投影。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NatureGeneratedFile {
    pub path: String,
    pub source: String,
}

/// revision 的完整审查视图。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NatureRevisionView {
    pub id: String,
    pub project_id: String,
    pub source_text: String,
    pub status: String,
    pub blueprint: Option<serde_json::Value>,
    pub inference_decisions: Vec<serde_json::Value>,
    pub defaults: Vec<serde_json::Value>,
    pub diagnostics: Vec<serde_json::Value>,
    pub breaking_changes: Vec<serde_json::Value>,
    pub generated_files: Vec<NatureGeneratedFile>,
    pub artifact_hash: Option<String>,
    pub error_message: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 显式发布结果。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedNatureRevision {
    pub revision_id: String,
    pub artifact_hash: String,
    pub published_at_ms: i64,
}
