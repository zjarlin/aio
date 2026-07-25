use std::{path::PathBuf, sync::Arc};

use az_aio_codegen::gate::ArtifactGate;
use nature_compiler::{CapabilityCatalog, CompileRequest, Compiler, MotherTongueInferenceEngine};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let compiler = Compiler::new(
        Arc::new(MotherTongueInferenceEngine),
        CapabilityCatalog::with_fixture_map(),
    );
    let source = include_str!("../../../crates/generated/nature/blueprint-source.txt");
    let result = compiler
        .compile(CompileRequest {
            source_text: source.to_string(),
            previous_blueprint: None,
        })
        .await?;
    let artifacts = result
        .artifacts
        .ok_or_else(|| anyhow::anyhow!("fixture Blueprint 未生成 artifact"))?;
    let output_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
    let artifacts = ArtifactGate::new(output_root).verify_and_publish(&artifacts, None)?;
    println!("{}", artifacts.hash);
    Ok(())
}
