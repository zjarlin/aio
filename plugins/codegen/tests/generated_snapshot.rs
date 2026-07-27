use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use az_aio_codegen::deployment::{ApplicationDeployment, lower_application};
use az_aio_codegen::gate::ArtifactGate;
use az_remote_ui::ComponentIndex;
use nature_compiler::{ArtifactFile, ArtifactSet, Blueprint, RustBackend};
use rudi::Context as RudiContext;

#[tokio::test]
async fn committed_sources_match_deterministic_generation() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
    let blueprint =
        serde_json::from_str::<Blueprint>(&fs::read_to_string(root.join("blueprint.json"))?)?;
    let mut artifacts = RustBackend.generate(&blueprint)?;
    let mut context = RudiContext::auto_register();
    let components = ComponentIndex::from_context(&mut context)?;
    let deployment: ApplicationDeployment = lower_application(&blueprint, &components)?;
    artifacts.files.push(ArtifactFile {
        relative_path: "deployment.json".to_string(),
        source: serde_json::to_string_pretty(&deployment)?,
    });
    let artifacts = ArtifactSet::new(artifacts.files);
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
