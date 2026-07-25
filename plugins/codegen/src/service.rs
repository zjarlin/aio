//! nature 编译、门禁与持久化编排。

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, anyhow};
use az_aio_platform::system::store::SystemAdminStore;
use az_micro_dict::contribution::DictSourceBundle;
use nature_compiler::{ArtifactFile, ArtifactSet, CompileRequest, CompileResult, Compiler};

use crate::{dictionary_source::enabled_dictionary_bundle, gate::ArtifactGate, store::NatureStore};

/// AIO 宿主侧生成服务。
#[derive(Clone)]
pub struct NatureService {
    store: NatureStore,
    compiler: Arc<Compiler>,
    gate: ArtifactGate,
    dictionary_store: SystemAdminStore,
}

impl NatureService {
    pub fn new(
        store: NatureStore,
        compiler: Compiler,
        output_root: PathBuf,
        dictionary_store: SystemAdminStore,
    ) -> Self {
        Self {
            store,
            compiler: Arc::new(compiler),
            gate: ArtifactGate::new(output_root),
            dictionary_store,
        }
    }

    pub fn store(&self) -> &NatureStore {
        &self.store
    }

    pub async fn generate_revision(&self, revision_id: String) -> anyhow::Result<()> {
        let run_id = self.store.create_run(&revision_id).await?;
        let outcome = self.generate_revision_inner(&revision_id).await;
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

    async fn generate_revision_inner(&self, revision_id: &str) -> anyhow::Result<ArtifactSet> {
        let revision = self.store.revision(revision_id).await?;
        let project_id = revision.project_id.clone();
        self.store
            .mark_revision_status(revision_id, "running")
            .await?;
        let previous_blueprint = self.store.latest_blueprint(&project_id).await?;
        let result = self
            .compiler
            .compile(CompileRequest {
                source_text: revision.source_text,
                previous_blueprint,
            })
            .await
            .context("nature-compiler 推导与生成失败")?;
        let Some(mut artifacts) = result.artifacts.clone() else {
            let message = diagnostic_summary(&result);
            self.store
                .fail_revision(revision_id, Some(&result), &message)
                .await?;
            return Err(anyhow!(message));
        };
        if let Some(bundle) = enabled_dictionary_bundle(&self.dictionary_store).await? {
            artifacts = attach_dictionary_bundle(artifacts, &bundle)?;
        }

        self.store
            .mark_revision_status(revision_id, "checking")
            .await?;
        let gate = self.gate.clone();
        let gate_artifacts = artifacts.clone();
        let artifacts =
            tokio::task::spawn_blocking(move || gate.verify_and_publish(&gate_artifacts, None))
                .await
                .context("nature 生成门禁任务异常退出")??;
        let blueprint = result
            .blueprint
            .as_ref()
            .ok_or_else(|| anyhow!("成功生成结果缺少 Blueprint"))?;
        self.store
            .replace_field_bindings(&project_id, blueprint)
            .await?;
        self.store
            .complete_revision(revision_id, &result, &artifacts)
            .await?;
        Ok(artifacts)
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
            if file.relative_path == "src/enums.rs" {
                file.source = format!("{}\n{}", file.source, dictionary_enums);
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
