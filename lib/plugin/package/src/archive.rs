use std::io::{Read as _, Write as _};

use anyhow::{Context as _, Result, ensure};
use flate2::{Compression, GzBuilder, bufread::GzDecoder};

use crate::{MAX_PACKAGE_BYTES, MAX_PACKAGE_JSON_BYTES, PluginPackage};

impl PluginPackage {
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.verify()?;
        let json = serde_json::to_vec(self).context("序列化插件包失败")?;
        ensure!(
            json.len() <= MAX_PACKAGE_JSON_BYTES,
            "插件包 JSON 超过 48 MiB"
        );
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(Vec::new(), Compression::default());
        encoder.write_all(&json).context("压缩插件包失败")?;
        let bytes = encoder.finish().context("完成插件包压缩失败")?;
        ensure!(bytes.len() <= MAX_PACKAGE_BYTES, "插件包超过 48 MiB");
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_PACKAGE_BYTES, "插件包超过 48 MiB");
        let mut decoder = GzDecoder::new(bytes);
        let mut json = Vec::new();
        decoder
            .by_ref()
            .take(MAX_PACKAGE_JSON_BYTES as u64 + 1)
            .read_to_end(&mut json)
            .context("解压插件包失败")?;
        ensure!(
            json.len() <= MAX_PACKAGE_JSON_BYTES,
            "插件包解压后超过 48 MiB"
        );
        ensure!(decoder.get_ref().is_empty(), "插件包 gzip 尾部包含额外数据");
        let package: Self = serde_json::from_slice(&json).context("解析插件包失败")?;
        package.verify()?;
        Ok(package)
    }
}
