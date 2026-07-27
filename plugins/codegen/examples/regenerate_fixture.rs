use std::{path::PathBuf, sync::Arc};

use az_aio_codegen::deployment::lower_application;
use az_aio_codegen::gate::ArtifactGate;
use az_remote_ui::ComponentIndex;
use nature_compiler::{
    ArtifactFile, ArtifactSet, CompileRequest, Compiler, CompilerCatalog,
    MotherTongueInferenceEngine,
};
use rudi::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let compiler = Compiler::new(
        Arc::new(MotherTongueInferenceEngine),
        CompilerCatalog::with_fixture_map(),
    );
    let source = include_str!("../../../crates/generated/nature/blueprint-source.txt");
    let result = compiler
        .compile(CompileRequest {
            source_text: source.to_string(),
            previous_blueprint: None,
        })
        .await?;
    let blueprint = result
        .blueprint
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("fixture 编译结果缺少 Blueprint"))?;
    let mut artifacts = result
        .artifacts
        .ok_or_else(|| anyhow::anyhow!("fixture Blueprint 未生成 artifact"))?;
    let mut context = Context::auto_register();
    let components = ComponentIndex::from_context(&mut context)?;
    let deployment = lower_application(blueprint, &components)?;
    artifacts.files.push(ArtifactFile {
        relative_path: "deployment.json".to_string(),
        source: serde_json::to_string_pretty(&deployment)?,
    });
    let artifacts = ArtifactSet::new(artifacts.files);
    let output_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
    let artifacts = ArtifactGate::new(output_root).verify_and_publish(&artifacts, None)?;
    println!("{}", artifacts.hash);
    Ok(())
}
