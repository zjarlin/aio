use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

const MAX_OBJECT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct ObjectStore {
    root: PathBuf,
    gate: Arc<Mutex<()>>,
    source: String,
    tenant: String,
}

impl ObjectStore {
    pub async fn open(root: impl AsRef<Path>, source: &str, tenant: &str) -> Result<Self> {
        let namespace = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(source, tenant))?)
        );
        let root = root.as_ref().join(namespace);
        tokio::fs::create_dir_all(&root)
            .await
            .context("创建插件对象目录失败")?;
        ensure!(
            !tokio::fs::symlink_metadata(&root)
                .await?
                .file_type()
                .is_symlink(),
            "对象目录不能是符号链接"
        );
        Ok(Self {
            root: tokio::fs::canonicalize(root).await?,
            gate: Arc::new(Mutex::new(())),
            source: source.into(),
            tenant: tenant.into(),
        })
    }

    pub(crate) fn matches(&self, source: &str, tenant: &str) -> bool {
        self.source == source && self.tenant == tenant
    }

    fn path(&self, key: &str) -> Result<PathBuf> {
        ensure!(!key.is_empty() && key.len() <= 1024, "对象键长度无效");
        Ok(self
            .root
            .join(format!("{:x}", Sha256::digest(key.as_bytes()))))
    }

    pub async fn read(&self, key: &str) -> Result<Vec<u8>> {
        let _guard = self.gate.lock().await;
        let path = self.path(key)?;
        let metadata = tokio::fs::symlink_metadata(&path)
            .await
            .context("对象不存在")?;
        ensure!(
            metadata.is_file() && metadata.len() <= MAX_OBJECT_BYTES as u64,
            "对象类型或大小无效"
        );
        Ok(tokio::fs::read(path).await?)
    }

    pub async fn write(&self, key: &str, bytes: &[u8]) -> Result<()> {
        ensure!(bytes.len() <= MAX_OBJECT_BYTES, "对象超过 16 MiB 配额");
        let _guard = self.gate.lock().await;
        let path = self.path(key)?;
        let mut random = [0; 16];
        getrandom::fill(&mut random)
            .map_err(|error| anyhow::anyhow!("生成对象临时标识失败: {error}"))?;
        let temporary = self.root.join(format!(
            ".{}.tmp",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
        let result = async {
            tokio::fs::write(&temporary, bytes).await?;
            tokio::fs::rename(&temporary, path).await?;
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(temporary).await;
        }
        result
    }

    pub async fn remove(&self, key: &str) -> Result<()> {
        let _guard = self.gate.lock().await;
        match tokio::fs::remove_file(self.path(key)?).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("删除对象失败"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn tenant_keys_are_isolated_and_paths_cannot_escape() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let first = super::ObjectStore::open(root.path(), "source", "first").await?;
        let second = super::ObjectStore::open(root.path(), "source", "second").await?;
        first.write("../../private", b"payload").await?;
        assert_eq!(first.read("../../private").await?, b"payload");
        assert!(second.read("../../private").await.is_err());
        first.remove("../../private").await?;
        assert!(first.read("../../private").await.is_err());
        Ok(())
    }
}
