//! nature 编译、门禁与持久化编排。

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, anyhow};
use az_aio_platform::system::store::SystemAdminStore;
use az_engine::EngineStore;
use az_engine::route::EngineApplicationDeployment;
use az_micro_dict::contribution::DictSourceBundle;
use az_remote_ui::ComponentIndex;
use nature_compiler::{
    ArtifactFile, ArtifactSet, CompileRequest, CompileResult, CompileStage, CompileStageStatus,
    CompileTrace, Compiler,
};
use serde_json::{Value, json};

use crate::{
    contract::{NatureGeneratedFile, PublishedNatureRevision},
    deployment::{ApplicationDeployment, lower_application},
    dictionary_source::enabled_dictionary_bundle,
    gate::ArtifactGate,
    store::{CompletedGenerationEventInput, GenerationEventHandle, NatureStore},
};

/// AIO 宿主侧生成服务。
#[derive(Clone)]
pub struct NatureService {
    store: NatureStore,
    compiler: Arc<Compiler>,
    gate: ArtifactGate,
    dictionary_store: SystemAdminStore,
    components: Arc<ComponentIndex>,
    engine_store: EngineStore,
}

impl NatureService {
    pub fn new(
        store: NatureStore,
        compiler: Compiler,
        output_root: PathBuf,
        dictionary_store: SystemAdminStore,
        components: Arc<ComponentIndex>,
        engine_store: EngineStore,
    ) -> Self {
        Self {
            store,
            compiler: Arc::new(compiler),
            gate: ArtifactGate::new(output_root),
            dictionary_store,
            components,
            engine_store,
        }
    }

    pub fn store(&self) -> &NatureStore {
        &self.store
    }

    /// 校验 artifact 并原子物化低代码应用后发布 Revision。
    pub async fn publish_revision(
        &self,
        revision_id: &str,
        registered_hash: &str,
    ) -> anyhow::Result<PublishedNatureRevision> {
        let revision = self.store.revision(revision_id).await?;
        if revision.status != "succeeded" {
            return Err(anyhow!(
                "只有生成成功的 revision 可以发布，当前状态: {}",
                revision.status
            ));
        }
        if revision.artifact_hash != registered_hash {
            return Err(anyhow!(
                "运行中的 AIO artifact hash 不匹配: revision={}, runtime={registered_hash}",
                revision.artifact_hash
            ));
        }
        let files =
            serde_json::from_str::<Vec<NatureGeneratedFile>>(&revision.generated_files_json)
                .context("读取 revision 生成文件失败")?;
        let deployment_source = files
            .iter()
            .find(|file| file.path == "deployment.json")
            .map(|file| file.source.as_str())
            .context("revision 缺少 deployment.json")?;
        let deployment = serde_json::from_str::<ApplicationDeployment>(deployment_source)
            .context("解析 ApplicationDeployment 失败")?;
        let manifest = serde_json::to_value(&deployment)
            .context("序列化 ApplicationDeployment manifest 失败")?;
        self.engine_store
            .deploy_application(EngineApplicationDeployment {
                project_id: revision.project_id,
                revision_id: revision.id,
                artifact_hash: revision.artifact_hash,
                domain_code: deployment.domain_code,
                manifest,
                models: deployment.models,
                operations: deployment.operations,
                pages: deployment.pages,
                routes: deployment.routes,
            })
            .await
            .context("物化低代码应用部署失败")?;
        self.store
            .publish_revision(revision_id, registered_hash)
            .await
    }

    pub async fn generate_revision(&self, revision_id: String) -> anyhow::Result<()> {
        let run_id = self.store.create_run(&revision_id).await?;
        let mut timeline = GenerationTimeline::new(run_id.clone(), revision_id.clone());
        let outcome = self
            .generate_revision_inner(&revision_id, &mut timeline)
            .await;
        match outcome {
            Ok(artifacts) => {
                self.store
                    .finish_run(&run_id, "succeeded", "complete", &artifacts.hash, "")
                    .await?;
                Ok(())
            }
            Err(error) => {
                let message = format!("{error:#}");
                let revision = self.store.revision(&revision_id).await?;
                if revision.status != "failed" {
                    self.store
                        .fail_revision(&revision_id, None, &message)
                        .await?;
                }
                self.store
                    .finish_run(&run_id, "failed", "failed", "", &message)
                    .await?;
                Err(error)
            }
        }
    }

    pub async fn resume_incomplete(&self) -> anyhow::Result<()> {
        for revision_id in self.store.pending_revision_ids().await? {
            let service = self.clone();
            tokio::spawn(async move {
                if let Err(error) = service.generate_revision(revision_id).await {
                    tracing::error!(error = %error, "恢复的 nature 生成任务失败");
                }
            });
        }
        Ok(())
    }

    async fn generate_revision_inner(
        &self,
        revision_id: &str,
        timeline: &mut GenerationTimeline,
    ) -> anyhow::Result<ArtifactSet> {
        let context_event = timeline
            .start(&self.store, GenerationStage::LoadContext, json!({}))
            .await?;
        let context_result = async {
            let revision = self.store.revision(revision_id).await?;
            let project_id = revision.project_id.clone();
            self.store
                .mark_revision_status(revision_id, "running")
                .await?;
            let previous_blueprint = self.store.latest_blueprint(&project_id).await?;
            Ok::<_, anyhow::Error>((revision, project_id, previous_blueprint))
        }
        .await;
        let (revision, project_id, previous_blueprint) = match context_result {
            Ok(context) => {
                timeline
                    .finish(
                        &self.store,
                        &context_event,
                        true,
                        "",
                        json!({
                            "projectId": &context.1,
                            "previousBlueprint": context.2.is_some(),
                        }),
                    )
                    .await?;
                context
            }
            Err(error) => {
                timeline
                    .finish(
                        &self.store,
                        &context_event,
                        false,
                        &format!("{error:#}"),
                        json!({}),
                    )
                    .await?;
                return Err(error);
            }
        };

        let compile_event = timeline
            .start(&self.store, GenerationStage::Compile, json!({}))
            .await?;
        let compile_result = self
            .compiler
            .compile(CompileRequest {
                source_text: revision.source_text,
                previous_blueprint,
            })
            .await;
        let result = match compile_result {
            Ok(result) => result,
            Err(error) => {
                let error = error.context("nature-compiler 推导与生成失败");
                timeline
                    .finish(
                        &self.store,
                        &compile_event,
                        false,
                        &format!("{error:#}"),
                        json!({}),
                    )
                    .await?;
                return Err(error);
            }
        };
        timeline
            .record_compiler_trace(&self.store, &compile_event, &result.trace)
            .await?;
        let compile_metadata = json!({
            "inference": &result.trace.inference,
            "diagnosticCount": result.diagnostics.len(),
            "breakingChangeCount": result.breaking_changes.len(),
        });
        let Some(mut artifacts) = result.artifacts.clone() else {
            let message = diagnostic_summary(&result);
            timeline
                .finish(
                    &self.store,
                    &compile_event,
                    false,
                    &message,
                    compile_metadata,
                )
                .await?;
            self.store
                .fail_revision(revision_id, Some(&result), &message)
                .await?;
            return Err(anyhow!(message));
        };
        timeline
            .finish(&self.store, &compile_event, true, "", compile_metadata)
            .await?;

        let blueprint = result
            .blueprint
            .as_ref()
            .context("nature-compiler 成功结果缺少 Blueprint")?;
        let deployment = lower_application(blueprint, &self.components)
            .context("lowering AIO ApplicationDeployment 失败")?;
        artifacts.files.push(ArtifactFile {
            relative_path: "deployment.json".to_string(),
            source: serde_json::to_string_pretty(&deployment)
                .context("序列化 ApplicationDeployment 失败")?,
        });
        artifacts = ArtifactSet::new(artifacts.files);

        let dictionary_event = timeline
            .start(
                &self.store,
                GenerationStage::DictionaryGeneration,
                json!({}),
            )
            .await?;
        let bundle = match enabled_dictionary_bundle(&self.dictionary_store).await {
            Ok(bundle) => bundle,
            Err(error) => {
                timeline
                    .finish(
                        &self.store,
                        &dictionary_event,
                        false,
                        &format!("{error:#}"),
                        json!({}),
                    )
                    .await?;
                return Err(error);
            }
        };
        if let Some(bundle) = bundle.as_ref() {
            artifacts = match attach_dictionary_bundle(artifacts, bundle) {
                Ok(artifacts) => artifacts,
                Err(error) => {
                    timeline
                        .finish(
                            &self.store,
                            &dictionary_event,
                            false,
                            &format!("{error:#}"),
                            json!({}),
                        )
                        .await?;
                    return Err(error);
                }
            };
        }
        timeline
            .finish(
                &self.store,
                &dictionary_event,
                true,
                "",
                json!({
                    "enabled": bundle.is_some(),
                    "sourceFileCount": bundle.as_ref().map(|bundle| bundle.files.len()).unwrap_or(0),
                }),
            )
            .await?;

        self.store
            .mark_revision_status(revision_id, "checking")
            .await?;
        let gate_event = timeline
            .start(
                &self.store,
                GenerationStage::CargoGate,
                json!({
                    "commands": [
                        "cargo fmt --all -- --check",
                        "cargo check --all-targets",
                        "cargo test --all-targets",
                        "cargo clippy --all-targets",
                    ],
                }),
            )
            .await?;
        let gate = self.gate.clone();
        let gate_artifacts = artifacts.clone();
        let gate_result =
            tokio::task::spawn_blocking(move || gate.verify_and_publish(&gate_artifacts, None))
                .await
                .context("nature 生成门禁任务异常退出");
        let artifacts = match gate_result {
            Ok(Ok(artifacts)) => {
                timeline
                    .finish(
                        &self.store,
                        &gate_event,
                        true,
                        "",
                        json!({
                            "artifactHash": &artifacts.hash,
                            "generatedFileCount": artifacts.files.len(),
                        }),
                    )
                    .await?;
                artifacts
            }
            Ok(Err(error)) | Err(error) => {
                timeline
                    .finish(
                        &self.store,
                        &gate_event,
                        false,
                        &format!("{error:#}"),
                        json!({}),
                    )
                    .await?;
                return Err(error);
            }
        };
        let blueprint = result
            .blueprint
            .as_ref()
            .ok_or_else(|| anyhow!("成功生成结果缺少 Blueprint"))?;

        let binding_event = timeline
            .start(
                &self.store,
                GenerationStage::FieldBindings,
                json!({ "bindingCount": blueprint.bindings.len() }),
            )
            .await?;
        if let Err(error) = self
            .store
            .replace_field_bindings(&project_id, blueprint)
            .await
        {
            timeline
                .finish(
                    &self.store,
                    &binding_event,
                    false,
                    &format!("{error:#}"),
                    json!({}),
                )
                .await?;
            return Err(error);
        }
        timeline
            .finish(
                &self.store,
                &binding_event,
                true,
                "",
                json!({ "bindingCount": blueprint.bindings.len() }),
            )
            .await?;

        let persist_event = timeline
            .start(&self.store, GenerationStage::PersistResult, json!({}))
            .await?;
        if let Err(error) = self
            .store
            .complete_revision(revision_id, &result, &artifacts)
            .await
        {
            timeline
                .finish(
                    &self.store,
                    &persist_event,
                    false,
                    &format!("{error:#}"),
                    json!({}),
                )
                .await?;
            return Err(error);
        }
        timeline
            .finish(
                &self.store,
                &persist_event,
                true,
                "",
                json!({ "artifactHash": &artifacts.hash }),
            )
            .await?;
        Ok(artifacts)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GenerationStage {
    LoadContext,
    Compile,
    DictionaryGeneration,
    CargoGate,
    FieldBindings,
    PersistResult,
}

impl GenerationStage {
    fn encode(self) -> &'static str {
        match self {
            Self::LoadContext => "load_context",
            Self::Compile => "compile",
            Self::DictionaryGeneration => "dictionary_generation",
            Self::CargoGate => "cargo_gate",
            Self::FieldBindings => "field_bindings",
            Self::PersistResult => "persist_result",
        }
    }
}

struct GenerationTimeline {
    run_id: String,
    revision_id: String,
    next_sequence: i64,
}

impl GenerationTimeline {
    fn new(run_id: String, revision_id: String) -> Self {
        Self {
            run_id,
            revision_id,
            next_sequence: 1,
        }
    }

    async fn start(
        &mut self,
        store: &NatureStore,
        stage: GenerationStage,
        metadata: Value,
    ) -> anyhow::Result<GenerationEventHandle> {
        let sequence = self.take_sequence();
        store
            .start_run_event(
                &self.run_id,
                &self.revision_id,
                "",
                sequence,
                stage.encode(),
                &metadata,
            )
            .await
    }

    async fn finish(
        &self,
        store: &NatureStore,
        event: &GenerationEventHandle,
        succeeded: bool,
        message: &str,
        metadata: Value,
    ) -> anyhow::Result<()> {
        let status = if succeeded { "succeeded" } else { "failed" };
        store
            .finish_run_event(event, status, message, &metadata)
            .await
    }

    async fn record_compiler_trace(
        &mut self,
        store: &NatureStore,
        parent: &GenerationEventHandle,
        trace: &CompileTrace,
    ) -> anyhow::Result<()> {
        let mut started_at_ms = parent.started_at_ms;
        for observation in &trace.stages {
            let duration_ms = observation
                .duration_ms
                .min(i64::MAX as u64)
                .try_into()
                .unwrap_or(i64::MAX);
            let sequence = self.take_sequence();
            store
                .record_completed_run_event(CompletedGenerationEventInput {
                    run_id: &self.run_id,
                    revision_id: &self.revision_id,
                    parent_event_id: &parent.id,
                    sequence,
                    stage: compiler_stage_code(observation.stage),
                    status: compiler_stage_status(observation.status),
                    metadata: json!({}),
                    started_at_ms,
                    duration_ms,
                })
                .await?;
            started_at_ms = started_at_ms.saturating_add(duration_ms);
        }
        Ok(())
    }

    fn take_sequence(&mut self) -> i64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }
}

fn compiler_stage_code(stage: CompileStage) -> &'static str {
    match stage {
        CompileStage::SourceContract => "source_contract",
        CompileStage::Inference => "inference",
        CompileStage::CapabilityResolution => "capability_resolution",
        CompileStage::BlueprintPolicy => "blueprint_policy",
        CompileStage::RustGeneration => "rust_generation",
    }
}

fn compiler_stage_status(status: CompileStageStatus) -> &'static str {
    match status {
        CompileStageStatus::Succeeded => "succeeded",
        CompileStageStatus::Failed => "failed",
    }
}

fn attach_dictionary_bundle(
    artifacts: ArtifactSet,
    bundle: &DictSourceBundle,
) -> anyhow::Result<ArtifactSet> {
    let dictionary_enums = bundle
        .files
        .iter()
        .find(|file| file.relative_path == std::path::Path::new("enums.rs"))
        .map(|file| file.source.as_str())
        .unwrap_or("");
    let mut files = artifacts
        .files
        .into_iter()
        .map(|mut file| {
            if file.relative_path == "src/enums.rs" && !dictionary_enums.trim().is_empty() {
                file.source = format!(
                    "{}\n\n{}\n",
                    file.source.trim_end(),
                    dictionary_enums.trim()
                );
            }
            file
        })
        .collect::<Vec<_>>();
    for file in &bundle.files {
        if !file.relative_path.starts_with("specs") {
            continue;
        }
        files.push(ArtifactFile {
            relative_path: format!("src/{}", file.relative_path.display()),
            source: file.source.clone(),
        });
    }
    if !dictionary_enums.is_empty()
        && !files
            .iter()
            .any(|file| file.relative_path == "src/enums.rs")
    {
        return Err(anyhow!("nature artifact 缺少 src/enums.rs"));
    }
    Ok(ArtifactSet::new(files))
}

fn diagnostic_summary(result: &CompileResult) -> String {
    let summary = result
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}：{}", diagnostic.subject, diagnostic.message))
        .collect::<Vec<_>>()
        .join("；");
    if summary.is_empty() {
        "nature-compiler 未生成 artifact".to_string()
    } else {
        summary
    }
}
