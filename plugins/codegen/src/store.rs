//! nature revision 的 PostgreSQL 读写边界。

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, anyhow, bail};
use az_aio_platform::core::db;
use nature_compiler::{ArtifactSet, Blueprint, CompileResult};
use serde::de::DeserializeOwned;
use toasty::stmt::{List, Query};
use tokio::sync::Mutex;

use crate::{
    contract::{
        NatureGeneratedFile, NatureGenerationEventView, NatureGenerationRunView,
        NatureRevisionView, PublishedNatureRevision,
    },
    model::{
        EngineFieldBindingRecord, NatureGenerationEventRecord, NatureGenerationRunRecord,
        NatureProjectRecord, NatureRevisionRecord,
    },
};

/// 正在执行的生成阶段句柄。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationEventHandle {
    pub id: String,
    pub started_at_ms: i64,
}

/// 已完成编译器子阶段的持久化输入。
pub struct CompletedGenerationEventInput<'a> {
    pub run_id: &'a str,
    pub revision_id: &'a str,
    pub parent_event_id: &'a str,
    pub sequence: i64,
    pub stage: &'a str,
    pub status: &'a str,
    pub metadata: serde_json::Value,
    pub started_at_ms: i64,
    pub duration_ms: i64,
}

/// nature 工作台正式存储。
#[derive(Clone)]
pub struct NatureStore {
    db: Arc<Mutex<toasty::Db>>,
}

impl NatureStore {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }

    pub async fn create_revision(
        &self,
        project_id: &str,
        source_text: &str,
    ) -> anyhow::Result<NatureRevisionRecord> {
        let project_id = required(project_id, "项目 ID")?;
        let source_text = required(source_text, "母语源码")?;
        self.ensure_project(project_id).await?;
        let now = db::timestamp_ms();
        let mut database = self.db.lock().await;
        NatureRevisionRecord::create()
            .id(db::new_uuid_id())
            .project_id(project_id)
            .source_text(source_text)
            .status("queued")
            .blueprint_json("")
            .inference_decisions_json("[]")
            .defaults_json("[]")
            .diagnostics_json("[]")
            .breaking_changes_json("[]")
            .generated_files_json("[]")
            .artifact_hash("")
            .error_message("")
            .published_at_ms(0)
            .created_at_ms(now)
            .updated_at_ms(now)
            .exec(&mut *database)
            .await
            .context("创建 nature revision 失败")
    }

    pub async fn revision(&self, revision_id: &str) -> anyhow::Result<NatureRevisionRecord> {
        let revision_id = required(revision_id, "revision ID")?;
        let mut database = self.db.lock().await;
        Query::<List<NatureRevisionRecord>>::filter(
            NatureRevisionRecord::fields().id().eq(revision_id),
        )
        .first()
        .exec(&mut *database)
        .await
        .context("读取 nature revision 失败")?
        .ok_or_else(|| anyhow!("nature revision 不存在: {revision_id}"))
    }

    pub async fn revision_view(&self, revision_id: &str) -> anyhow::Result<NatureRevisionView> {
        let record = self.revision(revision_id).await?;
        let runs = self.generation_runs(revision_id).await?;
        revision_view(record, runs)
    }

    pub async fn generation_runs(
        &self,
        revision_id: &str,
    ) -> anyhow::Result<Vec<NatureGenerationRunView>> {
        let mut database = self.db.lock().await;
        let mut runs = Query::<List<NatureGenerationRunRecord>>::filter(
            NatureGenerationRunRecord::fields()
                .revision_id()
                .eq(revision_id),
        )
        .exec(&mut *database)
        .await
        .context("读取 nature generation runs 失败")?;
        let events = Query::<List<NatureGenerationEventRecord>>::filter(
            NatureGenerationEventRecord::fields()
                .revision_id()
                .eq(revision_id),
        )
        .exec(&mut *database)
        .await
        .context("读取 nature generation events 失败")?;
        drop(database);

        let mut events_by_run = BTreeMap::<String, Vec<NatureGenerationEventView>>::new();
        for event in events {
            events_by_run
                .entry(event.run_id.clone())
                .or_default()
                .push(generation_event_view(event)?);
        }
        for events in events_by_run.values_mut() {
            events.sort_by_key(|event| event.sequence);
        }
        runs.sort_by_key(|run| run.started_at_ms);
        Ok(runs
            .into_iter()
            .map(|run| {
                let events = events_by_run.remove(&run.id).unwrap_or_default();
                generation_run_view(run, events)
            })
            .collect())
    }

    pub async fn latest_blueprint(&self, project_id: &str) -> anyhow::Result<Option<Blueprint>> {
        let mut database = self.db.lock().await;
        let records = Query::<List<NatureRevisionRecord>>::filter(
            NatureRevisionRecord::fields().project_id().eq(project_id),
        )
        .exec(&mut *database)
        .await
        .context("读取项目历史 Blueprint 失败")?;
        drop(database);
        let latest = records
            .into_iter()
            .filter(|record| matches!(record.status.as_str(), "succeeded" | "published"))
            .max_by_key(|record| record.updated_at_ms);
        match latest {
            Some(record) if !record.blueprint_json.is_empty() => {
                serde_json::from_str(&record.blueprint_json)
                    .context("解析上一版 Blueprint 失败")
                    .map(Some)
            }
            _ => Ok(None),
        }
    }

    pub async fn pending_revision_ids(&self) -> anyhow::Result<Vec<String>> {
        let mut database = self.db.lock().await;
        let records = Query::<List<NatureRevisionRecord>>::all()
            .exec(&mut *database)
            .await
            .context("读取待恢复 nature revision 失败")?;
        Ok(records
            .into_iter()
            .filter(|record| matches!(record.status.as_str(), "queued" | "running"))
            .map(|record| record.id)
            .collect())
    }

    pub async fn mark_revision_status(
        &self,
        revision_id: &str,
        status: &str,
    ) -> anyhow::Result<()> {
        let mut database = self.db.lock().await;
        NatureRevisionRecord::filter(NatureRevisionRecord::fields().id().eq(revision_id))
            .update()
            .status(status)
            .updated_at_ms(db::timestamp_ms())
            .exec(&mut *database)
            .await
            .with_context(|| format!("更新 nature revision 状态失败: {status}"))?;
        Ok(())
    }

    pub async fn complete_revision(
        &self,
        revision_id: &str,
        result: &CompileResult,
        artifacts: &ArtifactSet,
    ) -> anyhow::Result<()> {
        let blueprint = result
            .blueprint
            .as_ref()
            .ok_or_else(|| anyhow!("成功编译结果缺少 Blueprint"))?;
        let generated_files = artifacts
            .files
            .iter()
            .map(|file| NatureGeneratedFile {
                path: file.relative_path.clone(),
                source: file.source.clone(),
            })
            .collect::<Vec<_>>();
        let mut database = self.db.lock().await;
        NatureRevisionRecord::filter(NatureRevisionRecord::fields().id().eq(revision_id))
            .update()
            .status("succeeded")
            .blueprint_json(serde_json::to_string(blueprint)?)
            .inference_decisions_json(serde_json::to_string(&blueprint.inference_decisions)?)
            .defaults_json(serde_json::to_string(&blueprint.defaults)?)
            .diagnostics_json(serde_json::to_string(&result.diagnostics)?)
            .breaking_changes_json(serde_json::to_string(&result.breaking_changes)?)
            .generated_files_json(serde_json::to_string(&generated_files)?)
            .artifact_hash(&artifacts.hash)
            .error_message("")
            .updated_at_ms(db::timestamp_ms())
            .exec(&mut *database)
            .await
            .context("保存 nature revision 编译结果失败")?;
        Ok(())
    }

    pub async fn replace_field_bindings(
        &self,
        project_id: &str,
        blueprint: &Blueprint,
    ) -> anyhow::Result<()> {
        let now = db::timestamp_ms();
        let mut database = self.db.lock().await;
        EngineFieldBindingRecord::filter(
            EngineFieldBindingRecord::fields()
                .project_id()
                .eq(project_id),
        )
        .delete()
        .exec(&mut *database)
        .await
        .context("清理项目旧字段绑定失败")?;
        for binding in &blueprint.bindings {
            let field = blueprint
                .structs
                .iter()
                .find(|definition| definition.descriptor == binding.owner)
                .and_then(|definition| {
                    definition
                        .fields
                        .iter()
                        .find(|field| field.descriptor == binding.field)
                })
                .ok_or_else(|| anyhow!("字段绑定引用不存在: {}", binding.field.native_name))?;
            EngineFieldBindingRecord::create()
                .id(db::new_uuid_id())
                .project_id(project_id)
                .owner_model_code(&binding.owner.code)
                .field_code(&binding.field.code)
                .source_name(&binding.source.native_name)
                .transform_json(serde_json::to_string(&binding.transform)?)
                .domain_metadata_json(serde_json::to_string(&field.domain_metadata)?)
                .validation_json(serde_json::to_string(&field.validations)?)
                .created_at_ms(now)
                .updated_at_ms(now)
                .exec(&mut *database)
                .await
                .context("保存项目字段绑定失败")?;
        }
        Ok(())
    }

    pub async fn fail_revision(
        &self,
        revision_id: &str,
        result: Option<&CompileResult>,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let diagnostics = result
            .map(|result| serde_json::to_string(&result.diagnostics))
            .transpose()?
            .unwrap_or_else(|| "[]".to_string());
        let breaking_changes = result
            .map(|result| serde_json::to_string(&result.breaking_changes))
            .transpose()?
            .unwrap_or_else(|| "[]".to_string());
        let blueprint = result
            .and_then(|result| result.blueprint.as_ref())
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        let mut database = self.db.lock().await;
        NatureRevisionRecord::filter(NatureRevisionRecord::fields().id().eq(revision_id))
            .update()
            .status("failed")
            .blueprint_json(blueprint)
            .diagnostics_json(diagnostics)
            .breaking_changes_json(breaking_changes)
            .error_message(error_message)
            .updated_at_ms(db::timestamp_ms())
            .exec(&mut *database)
            .await
            .context("保存 nature revision 失败结果失败")?;
        Ok(())
    }

    pub async fn create_run(&self, revision_id: &str) -> anyhow::Result<String> {
        let id = db::new_uuid_id();
        let mut database = self.db.lock().await;
        NatureGenerationRunRecord::create()
            .id(&id)
            .revision_id(revision_id)
            .status("running")
            .stage("inference")
            .artifact_hash("")
            .error_message("")
            .started_at_ms(db::timestamp_ms())
            .finished_at_ms(0)
            .exec(&mut *database)
            .await
            .context("创建 nature generation run 失败")?;
        Ok(id)
    }

    pub async fn finish_run(
        &self,
        run_id: &str,
        status: &str,
        stage: &str,
        artifact_hash: &str,
        error_message: &str,
    ) -> anyhow::Result<()> {
        let mut database = self.db.lock().await;
        NatureGenerationRunRecord::filter(NatureGenerationRunRecord::fields().id().eq(run_id))
            .update()
            .status(status)
            .stage(stage)
            .artifact_hash(artifact_hash)
            .error_message(error_message)
            .finished_at_ms(db::timestamp_ms())
            .exec(&mut *database)
            .await
            .context("结束 nature generation run 失败")?;
        Ok(())
    }

    pub async fn start_run_event(
        &self,
        run_id: &str,
        revision_id: &str,
        parent_event_id: &str,
        sequence: i64,
        stage: &str,
        metadata: &serde_json::Value,
    ) -> anyhow::Result<GenerationEventHandle> {
        let id = db::new_uuid_id();
        let started_at_ms = db::timestamp_ms();
        let mut database = self.db.lock().await;
        NatureGenerationRunRecord::filter(NatureGenerationRunRecord::fields().id().eq(run_id))
            .update()
            .stage(stage)
            .exec(&mut *database)
            .await
            .context("更新 nature generation run 阶段失败")?;
        NatureGenerationEventRecord::create()
            .id(&id)
            .run_id(run_id)
            .revision_id(revision_id)
            .parent_event_id(parent_event_id)
            .sequence(sequence)
            .stage(stage)
            .status("running")
            .message("")
            .metadata_json(serde_json::to_string(metadata)?)
            .started_at_ms(started_at_ms)
            .finished_at_ms(0)
            .duration_ms(0)
            .exec(&mut *database)
            .await
            .context("创建 nature generation event 失败")?;
        Ok(GenerationEventHandle { id, started_at_ms })
    }

    pub async fn finish_run_event(
        &self,
        handle: &GenerationEventHandle,
        status: &str,
        message: &str,
        metadata: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let finished_at_ms = db::timestamp_ms();
        let duration_ms = finished_at_ms.saturating_sub(handle.started_at_ms);
        let mut database = self.db.lock().await;
        NatureGenerationEventRecord::filter(
            NatureGenerationEventRecord::fields().id().eq(&handle.id),
        )
        .update()
        .status(status)
        .message(message)
        .metadata_json(serde_json::to_string(metadata)?)
        .finished_at_ms(finished_at_ms)
        .duration_ms(duration_ms)
        .exec(&mut *database)
        .await
        .context("结束 nature generation event 失败")?;
        Ok(())
    }

    pub async fn record_completed_run_event(
        &self,
        input: CompletedGenerationEventInput<'_>,
    ) -> anyhow::Result<()> {
        let finished_at_ms = input.started_at_ms.saturating_add(input.duration_ms);
        let mut database = self.db.lock().await;
        NatureGenerationEventRecord::create()
            .id(db::new_uuid_id())
            .run_id(input.run_id)
            .revision_id(input.revision_id)
            .parent_event_id(input.parent_event_id)
            .sequence(input.sequence)
            .stage(input.stage)
            .status(input.status)
            .message("")
            .metadata_json(serde_json::to_string(&input.metadata)?)
            .started_at_ms(input.started_at_ms)
            .finished_at_ms(finished_at_ms)
            .duration_ms(input.duration_ms)
            .exec(&mut *database)
            .await
            .context("保存 nature compiler 子阶段事件失败")?;
        Ok(())
    }

    pub async fn publish_revision(
        &self,
        revision_id: &str,
        registered_hash: &str,
    ) -> anyhow::Result<PublishedNatureRevision> {
        let record = self.revision(revision_id).await?;
        if record.status != "succeeded" {
            bail!(
                "只有生成成功的 revision 可以发布，当前状态: {}",
                record.status
            );
        }
        if record.artifact_hash != registered_hash {
            bail!(
                "运行中的 AIO artifact hash 不匹配: revision={}, runtime={registered_hash}",
                record.artifact_hash
            );
        }
        let published_at_ms = db::timestamp_ms();
        let mut database = self.db.lock().await;
        NatureRevisionRecord::filter(NatureRevisionRecord::fields().id().eq(revision_id))
            .update()
            .status("published")
            .published_at_ms(published_at_ms)
            .updated_at_ms(published_at_ms)
            .exec(&mut *database)
            .await
            .context("发布 nature revision 失败")?;
        Ok(PublishedNatureRevision {
            revision_id: revision_id.to_string(),
            artifact_hash: registered_hash.to_string(),
            published_at_ms,
        })
    }

    async fn ensure_project(&self, project_id: &str) -> anyhow::Result<()> {
        let mut database = self.db.lock().await;
        let existing = Query::<List<NatureProjectRecord>>::filter(
            NatureProjectRecord::fields().id().eq(project_id),
        )
        .first()
        .exec(&mut *database)
        .await
        .context("读取 nature project 失败")?;
        if existing.is_some() {
            return Ok(());
        }
        let now = db::timestamp_ms();
        NatureProjectRecord::create()
            .id(project_id)
            .native_name(project_id)
            .created_at_ms(now)
            .updated_at_ms(now)
            .exec(&mut *database)
            .await
            .context("创建 nature project 失败")?;
        Ok(())
    }
}

fn revision_view(
    record: NatureRevisionRecord,
    runs: Vec<NatureGenerationRunView>,
) -> anyhow::Result<NatureRevisionView> {
    let blueprint = if record.blueprint_json.is_empty() {
        None
    } else {
        Some(serde_json::from_str(&record.blueprint_json).context("解析 Blueprint 视图失败")?)
    };
    Ok(NatureRevisionView {
        id: record.id,
        project_id: record.project_id,
        source_text: record.source_text,
        status: record.status,
        blueprint,
        inference_decisions: parse_json_list(&record.inference_decisions_json)?,
        defaults: parse_json_list(&record.defaults_json)?,
        diagnostics: parse_json_list(&record.diagnostics_json)?,
        breaking_changes: parse_json_list(&record.breaking_changes_json)?,
        generated_files: parse_json_list(&record.generated_files_json)?,
        runs,
        artifact_hash: non_empty(record.artifact_hash),
        error_message: non_empty(record.error_message),
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
    })
}

fn generation_run_view(
    record: NatureGenerationRunRecord,
    events: Vec<NatureGenerationEventView>,
) -> NatureGenerationRunView {
    NatureGenerationRunView {
        id: record.id,
        status: record.status,
        stage: record.stage,
        artifact_hash: non_empty(record.artifact_hash),
        error_message: non_empty(record.error_message),
        started_at_ms: record.started_at_ms,
        finished_at_ms: positive(record.finished_at_ms),
        duration_ms: positive(record.finished_at_ms)
            .map(|finished| finished.saturating_sub(record.started_at_ms)),
        events,
    }
}

fn generation_event_view(
    record: NatureGenerationEventRecord,
) -> anyhow::Result<NatureGenerationEventView> {
    let finished_at_ms = positive(record.finished_at_ms);
    Ok(NatureGenerationEventView {
        id: record.id,
        parent_event_id: non_empty(record.parent_event_id),
        sequence: record.sequence,
        label: generation_stage_label(&record.stage).to_string(),
        stage: record.stage,
        status: record.status,
        message: non_empty(record.message),
        metadata: serde_json::from_str(&record.metadata_json)
            .context("解析 nature generation event metadata 失败")?,
        started_at_ms: record.started_at_ms,
        finished_at_ms,
        duration_ms: finished_at_ms.map(|_| record.duration_ms.max(0)),
    })
}

fn generation_stage_label(stage: &str) -> &str {
    match stage {
        "load_context" => "加载编译上下文",
        "compile" => "母语编译",
        "source_contract" => "输入契约校验",
        "inference" => "语义推导",
        "capability_resolution" => "能力解析",
        "blueprint_policy" => "Blueprint 策略校验",
        "rust_generation" => "Rust 生成",
        "dictionary_generation" => "字典源码合并",
        "cargo_gate" => "Cargo 编译门禁",
        "field_bindings" => "字段绑定持久化",
        "persist_result" => "编译结果持久化",
        _ => stage,
    }
}

fn parse_json_list<T: DeserializeOwned>(value: &str) -> anyhow::Result<Vec<T>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value).context("解析 nature revision 列表字段失败")
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn positive(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}

fn required<'a>(value: &'a str, label: &str) -> anyhow::Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{label}不能为空");
    }
    Ok(value)
}
