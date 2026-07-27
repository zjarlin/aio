//! nature-compiler 的 PostgreSQL 正式模型。

use az_aio_platform::core::db::ToastyModelContribution;
use serde::{Deserialize, Serialize};

/// 母语编译项目。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "nature_projects"]
pub struct NatureProjectRecord {
    #[key]
    pub id: String,
    pub native_name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 母语源码及完整编译结果快照。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "nature_revisions"]
pub struct NatureRevisionRecord {
    #[key]
    pub id: String,
    #[index]
    pub project_id: String,
    pub source_text: String,
    pub status: String,
    pub blueprint_json: String,
    pub inference_decisions_json: String,
    pub defaults_json: String,
    pub diagnostics_json: String,
    pub breaking_changes_json: String,
    pub generated_files_json: String,
    pub artifact_hash: String,
    pub error_message: String,
    pub published_at_ms: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 每次后台生成尝试的恢复与审计记录。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "nature_generation_runs"]
pub struct NatureGenerationRunRecord {
    #[key]
    pub id: String,
    #[index]
    pub revision_id: String,
    pub status: String,
    pub stage: String,
    pub artifact_hash: String,
    pub error_message: String,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}

/// 单次生成任务中的可观测阶段事件。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "nature_generation_events"]
pub struct NatureGenerationEventRecord {
    #[key]
    pub id: String,
    #[index]
    pub run_id: String,
    #[index]
    pub revision_id: String,
    pub parent_event_id: String,
    pub sequence: i64,
    pub stage: String,
    pub status: String,
    pub message: String,
    pub metadata_json: String,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
    pub duration_ms: i64,
}

/// 产品或设备模型拥有的字段数据源绑定。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "engine_field_bindings"]
pub struct EngineFieldBindingRecord {
    #[key]
    pub id: String,
    #[index]
    pub project_id: String,
    #[index]
    pub owner_model_code: String,
    pub field_code: String,
    pub source_name: String,
    pub transform_json: String,
    pub domain_metadata_json: String,
    pub validation_json: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 注册 nature 工作台的全部 Toasty 模型。
#[rudi::Singleton(name = "nature-compiler-toasty-models")]
pub fn nature_model_contribution() -> ToastyModelContribution {
    ToastyModelContribution::new(toasty::models!(
        NatureProjectRecord,
        NatureRevisionRecord,
        NatureGenerationRunRecord,
        NatureGenerationEventRecord,
        EngineFieldBindingRecord,
    ))
}
