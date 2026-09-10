use anyhow::{Context as _, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use url::Url;

use crate::{
    FORMAT_VERSION, FrontendAsset, MAX_ARTIFACT_BYTES, MAX_MANIFEST_BYTES, PluginPackage,
    VerifiedPluginPackage,
};

const PACKAGE_DIGEST_DOMAIN: &[u8] = b"aio-plugin-package-v2\0";
const MAX_BASE64_BYTES: usize = MAX_ARTIFACT_BYTES.div_ceil(3) * 4;

impl PluginPackage {
    pub fn new(
        git: String,
        version: String,
        source_revision: Option<String>,
        manifest_toml: String,
        artifact: &[u8],
        frontend: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        ensure!(
            artifact.len() <= MAX_ARTIFACT_BYTES,
            "插件 artifact 超过 32 MiB"
        );
        ensure!(
            frontend.len() <= az_plugin_manifest::MAX_FRONTEND_FILES,
            "前端资产数量超过限制"
        );
        let total = frontend
            .values()
            .try_fold(artifact.len(), |total, bytes| {
                total.checked_add(bytes.len())
            })
            .context("插件产物大小溢出")?;
        ensure!(total <= MAX_ARTIFACT_BYTES, "前后端产物合计超过 32 MiB");
        let mut package = Self {
            format_version: FORMAT_VERSION,
            git: normalize_git_source(&git)?,
            version,
            source_revision: source_revision.map(|value| value.to_ascii_lowercase()),
            manifest_toml,
            artifact_base64: STANDARD.encode(artifact),
            artifact_sha256: format!("{:x}", Sha256::digest(artifact)),
            frontend: frontend
                .into_iter()
                .map(|(path, content)| {
                    (
                        path,
                        FrontendAsset {
                            content_base64: STANDARD.encode(&content),
                            sha256: format!("{:x}", Sha256::digest(&content)),
                        },
                    )
                })
                .collect(),
            rev: String::new(),
        };
        package.rev = package.content_revision()?;
        package.verify()?;
        Ok(package)
    }

    pub fn verify(&self) -> Result<VerifiedPluginPackage> {
        ensure!(
            self.format_version == FORMAT_VERSION,
            "不支持的插件包格式版本"
        );
        ensure!(self.git.len() <= 4096, "插件来源 URL 过长");
        ensure!(
            self.git == normalize_git_source(&self.git)?,
            "插件包 Git 来源未规范化"
        );
        ensure!(self.version.len() <= 128, "插件版本过长");
        let version = semver::Version::parse(&self.version).context("插件版本必须是 SemVer")?;
        ensure!(version.to_string() == self.version, "插件版本未规范化");
        if let Some(revision) = &self.source_revision {
            ensure!(
                is_lower_hex(revision, 40),
                "源码审计版本必须是完整小写 SHA-1"
            );
        }
        ensure!(
            self.manifest_toml.len() <= MAX_MANIFEST_BYTES,
            "插件清单超过 128 KiB"
        );
        let manifest = az_plugin_manifest::parse_manifest(&self.manifest_toml)?;
        ensure!(
            manifest.plugin.marketplace.is_some(),
            "发布插件必须声明 [plugin.marketplace]"
        );
        ensure!(
            manifest.plugin.runtime.is_some(),
            "Rust 源码插件不能直接在线发布，请先构建可独立运行的产物"
        );
        ensure!(
            manifest
                .plugin
                .runtime
                .as_ref()
                .is_none_or(|runtime| runtime.artifact != "aio-plugin.toml"),
            "插件 artifact 不能覆盖 aio-plugin.toml 清单"
        );
        ensure!(
            manifest.plugin.client.is_none() && manifest.plugin.server.is_none(),
            "二进制插件包不能携带尚未编译装配的 Rust client/server 声明"
        );
        ensure!(
            self.artifact_base64.len() <= MAX_BASE64_BYTES,
            "插件 artifact base64 超过限制"
        );
        ensure!(
            is_lower_hex(&self.artifact_sha256, 64),
            "插件 artifact SHA-256 无效"
        );
        ensure!(
            is_lower_hex(&self.rev, 64),
            "插件包内容版本必须是完整小写 SHA-256"
        );
        ensure!(
            self.rev == self.content_revision()?,
            "插件包内容版本与元数据不一致"
        );
        let artifact = STANDARD
            .decode(&self.artifact_base64)
            .context("插件 artifact base64 无效")?;
        ensure!(!artifact.is_empty(), "插件 artifact 不能为空");
        ensure!(
            artifact.len() <= MAX_ARTIFACT_BYTES,
            "插件 artifact 超过 32 MiB"
        );
        ensure!(
            self.artifact_sha256 == format!("{:x}", Sha256::digest(&artifact)),
            "插件 artifact SHA-256 校验失败"
        );
        let frontend = self.verify_frontend(&manifest, artifact.len())?;
        Ok(VerifiedPluginPackage {
            manifest,
            artifact,
            frontend,
        })
    }

    fn content_revision(&self) -> Result<String> {
        let identity = serde_json::to_vec(&(
            self.format_version,
            &self.git,
            &self.version,
            &self.source_revision,
            &self.manifest_toml,
            &self.artifact_sha256,
            self.frontend
                .iter()
                .map(|(path, asset)| (path, &asset.sha256))
                .collect::<BTreeMap<_, _>>(),
        ))
        .context("序列化插件包内容身份失败")?;
        let mut digest = Sha256::new();
        digest.update(PACKAGE_DIGEST_DOMAIN);
        digest.update(identity);
        Ok(format!("{:x}", digest.finalize()))
    }
}

pub fn normalize_git_source(source: &str) -> Result<String> {
    ensure!(source.len() <= 4096, "插件来源 URL 过长");
    let mut url = Url::parse(source).context("插件 Git 来源不是有效 URL")?;
    ensure!(
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "插件来源只接受无凭证、无查询参数的 HTTPS Git URL"
    );
    let path = url.path().trim_end_matches('/');
    ensure!(!path.is_empty() && path != "/", "插件 Git 来源缺少仓库路径");
    let path = if path.ends_with(".git") {
        path.to_owned()
    } else {
        format!("{path}.git")
    };
    url.set_path(&path);
    Ok(url.into())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
