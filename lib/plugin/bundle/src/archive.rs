use std::io::{Read, Write};

use anyhow::{Result, ensure};
use flate2::{Compression, GzBuilder, bufread::GzDecoder};

use crate::{Bundle, MAX_ENCODED_BYTES};

impl Bundle {
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.verify()?;
        let json = serde_json::to_vec(self)?;
        ensure!(json.len() <= MAX_ENCODED_BYTES, "编码包超过配额");
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::default());
        encoder.write_all(&json)?;
        let archive = encoder.finish()?;
        ensure!(archive.len() <= MAX_ENCODED_BYTES, "压缩包超过配额");
        Ok(archive)
    }

    pub fn decode(archive: &[u8]) -> Result<Self> {
        ensure!(archive.len() <= MAX_ENCODED_BYTES, "压缩包超过配额");
        let mut decoder = GzDecoder::new(archive);
        let mut json = Vec::new();
        decoder
            .by_ref()
            .take(MAX_ENCODED_BYTES as u64 + 1)
            .read_to_end(&mut json)?;
        ensure!(json.len() <= MAX_ENCODED_BYTES, "解压包超过配额");
        ensure!(
            decoder.into_inner().is_empty(),
            "压缩包包含尾随数据或多个成员"
        );
        let bundle: Self = serde_json::from_slice(&json)?;
        bundle.verify()?;
        Ok(bundle)
    }
}
