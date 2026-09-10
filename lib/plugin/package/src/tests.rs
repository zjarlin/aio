use std::io::Write as _;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::{Compression, write::GzEncoder};

use crate::*;

const MANIFEST: &str = "[plugin.runtime]\nkind='page-definition'\nartifact='dist/pages.json'\n[plugin.marketplace]\ntitle='Pages'\nsummary='Demo pages'\nlicense='MIT'\ntags=['test']\n";

fn package() -> Result<PluginPackage> {
    PluginPackage::new(
        "https://EXAMPLE.com/team/pages/".to_owned(),
        "1.2.3".to_owned(),
        None,
        MANIFEST.to_owned(),
        b"[]",
    )
}

#[test]
fn package_is_portable_deterministic_and_canonical() -> Result<()> {
    let package = package()?;
    assert_eq!(package.git, "https://example.com/team/pages.git");
    assert_eq!(package.source_revision, None);
    assert_eq!(package.rev.len(), 64);
    let first = package.encode()?;
    assert_eq!(first, package.encode()?);
    assert_eq!(&first[4..8], &[0, 0, 0, 0]);
    assert_eq!(first[9], 255);
    let decoded = PluginPackage::decode(&first)?;
    assert_eq!(decoded, package);
    assert_eq!(decoded.verify()?.artifact, b"[]");
    Ok(())
}

#[test]
fn content_version_covers_every_identity_field() -> Result<()> {
    let original = package()?;
    let modifications: &[fn(&mut PluginPackage)] = &[
        |package| package.git = "https://example.com/another.git".to_owned(),
        |package| package.version = "1.2.4".to_owned(),
        |package| package.source_revision = Some("a".repeat(40)),
        |package| package.manifest_toml.push('\n'),
        |package| package.artifact_sha256 = "0".repeat(64),
        |package| package.rev = "0".repeat(64),
        |package| package.artifact_base64 = STANDARD.encode(b"{}"),
        |package| package.format_version = 2,
    ];
    for change in modifications {
        let mut tampered = original.clone();
        change(&mut tampered);
        assert!(tampered.verify().is_err());
        let gzip = compress(&serde_json::to_vec(&tampered)?)?;
        assert!(PluginPackage::decode(&gzip).is_err());
    }
    Ok(())
}

#[test]
fn rejects_truncated_concatenated_and_oversized_archives() -> Result<()> {
    let mut encoded = package()?.encode()?;
    assert!(PluginPackage::decode(&encoded[..encoded.len() - 1]).is_err());
    encoded.extend_from_slice(&package()?.encode()?);
    assert!(PluginPackage::decode(&encoded).is_err());
    assert!(PluginPackage::decode(&vec![0; MAX_PACKAGE_BYTES + 1]).is_err());
    let bomb = compress(&vec![b' '; MAX_PACKAGE_JSON_BYTES + 1])?;
    let error = PluginPackage::decode(&bomb).expect_err("超限解压必须失败");
    assert!(error.to_string().contains("解压后超过"));
    Ok(())
}

#[test]
fn bounds_base64_before_decoding_and_manifest_before_parsing() -> Result<()> {
    let mut package = package()?;
    package.artifact_base64 = "!".repeat(MAX_ARTIFACT_BYTES.div_ceil(3) * 4 + 1);
    assert!(
        package
            .verify()
            .expect_err("超限 artifact 必须失败")
            .to_string()
            .contains("base64 超过")
    );
    package.manifest_toml = "!".repeat(MAX_MANIFEST_BYTES + 1);
    assert!(
        package
            .verify()
            .expect_err("超限清单必须失败")
            .to_string()
            .contains("清单超过")
    );
    Ok(())
}

#[test]
fn rejects_invalid_versions_sources_and_legacy_publication() -> Result<()> {
    let mut package = package()?;
    let reserved_artifact = package
        .manifest_toml
        .replace("dist/pages.json", "aio-plugin.toml");
    assert!(
        PluginPackage::new(
            package.git.clone(),
            package.version.clone(),
            None,
            reserved_artifact,
            b"[]",
        )
        .is_err()
    );
    for version in ["latest", "v1.0.0", "1.0", "01.0.0"] {
        package.version = version.to_owned();
        assert!(package.verify().is_err());
    }
    for source in [
        "http://example.com/p",
        "git@example.com:p",
        "https://a:b@example.com/p",
        "https://example.com/p?q=1",
        "https://example.com/",
    ] {
        assert!(normalize_git_source(source).is_err());
    }
    let legacy = serde_json::json!({"git": "https://example.com/p.git", "rev": "a".repeat(40), "git_proof": {}});
    assert!(PluginPackage::decode(&compress(&serde_json::to_vec(&legacy)?)?).is_err());
    assert!(
        PluginPackage::new(
            "https://example.com/p.git".to_owned(),
            "1.0.0".to_owned(),
            None,
            "[plugin.client]\npath='.'\n".to_owned(),
            b"[]"
        )
        .is_err()
    );
    Ok(())
}

fn compress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?)
}
