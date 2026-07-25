use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail};
use az_aio_codegen::gate::ArtifactGate;
use nature_compiler::{CapabilityCatalog, CompileRequest, Compiler, MotherTongueInferenceEngine};

#[tokio::test]
async fn committed_sources_match_deterministic_generation() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
    let source = fs::read_to_string(root.join("blueprint-source.txt"))?;
    let compiler = Compiler::new(
        Arc::new(MotherTongueInferenceEngine),
        CapabilityCatalog::with_fixture_map(),
    );
    let result = compiler
        .compile(CompileRequest {
            source_text: source,
            previous_blueprint: None,
        })
        .await?;
    let Some(artifacts) = result.artifacts else {
        bail!("提交的 fixture Blueprint 未生成 artifact");
    };
    let artifacts = ArtifactGate::new(&root).format_artifacts(&artifacts, None)?;
    for file in artifacts.files {
        if file.relative_path == "src/enums.rs" {
            continue;
        }
        let committed = fs::read_to_string(root.join(&file.relative_path))
            .with_context(|| format!("读取提交生成物失败: {}", file.relative_path))?;
        let expected = if file.relative_path == "src/lib.rs" {
            format!(
                "{}\npub const ARTIFACT_HASH: &str = \"{}\";\n",
                file.source, artifacts.hash
            )
        } else {
            file.source
        };
        assert_eq!(committed, expected, "生成物漂移: {}", file.relative_path);
    }
    Ok(())
}
