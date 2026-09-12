use std::collections::BTreeMap;

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use anyhow::{Context, Result, anyhow, ensure};
use az_plugin_contract::InvocationScope;

#[derive(Clone)]
pub struct Keyring {
    active: String,
    keys: BTreeMap<String, [u8; 32]>,
}

impl Keyring {
    pub fn new(active: String, keys: BTreeMap<String, [u8; 32]>) -> Result<Self> {
        ensure!(keys.contains_key(&active), "当前加密密钥未配置");
        ensure!(
            keys.len() <= 32
                && keys
                    .keys()
                    .all(|id| !id.is_empty() && id.len() <= 64 && id.is_ascii()),
            "加密密钥版本无效"
        );
        Ok(Self { active, keys })
    }

    pub fn seal(
        &self,
        scope: &InvocationScope,
        purpose: &str,
        plaintext: &[u8],
    ) -> Result<Vec<u8>> {
        ensure!(plaintext.len() <= 512_000, "加密内容超过配额");
        let aad = associated_data(scope, purpose)?;
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| anyhow!("安全随机源不可用"))?;
        let cipher =
            Aes256Gcm::new_from_slice(&self.keys[&self.active]).map_err(|_| anyhow!("密钥无效"))?;
        let encrypted = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow!("加密失败"))?;
        let mut result = vec![1, self.active.len() as u8];
        result.extend_from_slice(self.active.as_bytes());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub fn open(
        &self,
        scope: &InvocationScope,
        purpose: &str,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        ensure!(
            ciphertext.len() >= 31 && ciphertext.len() <= 512_094 && ciphertext[0] == 1,
            "加密信封无效"
        );
        let key_len = ciphertext[1] as usize;
        ensure!(
            (1..=64).contains(&key_len) && ciphertext.len() >= 30 + key_len,
            "加密信封无效"
        );
        let key_id = std::str::from_utf8(&ciphertext[2..2 + key_len]).context("加密信封无效")?;
        let key = self.keys.get(key_id).context("所需历史密钥未配置")?;
        let offset = 2 + key_len;
        let aad = associated_data(scope, purpose)?;
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("密钥无效"))?;
        cipher
            .decrypt(
                Nonce::from_slice(&ciphertext[offset..offset + 12]),
                Payload {
                    msg: &ciphertext[offset + 12..],
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow!("密文校验失败"))
    }
}

fn associated_data(scope: &InvocationScope, purpose: &str) -> Result<Vec<u8>> {
    ensure!(!purpose.is_empty() && purpose.len() <= 512, "加密用途无效");
    let tenant = scope
        .context
        .tenant_id
        .as_deref()
        .filter(|v| !v.is_empty())
        .context("加密需要租户身份")?;
    ensure!(!scope.source_id.is_empty(), "加密需要插件身份");
    Ok(serde_json::to_vec(&(&scope.source_id, tenant, purpose))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciphertext_is_scoped_authenticated_and_readable_after_rotation() -> Result<()> {
        let mut scope = InvocationScope {
            source_id: "source".into(),
            revision: "v1".into(),
            context: Default::default(),
            grants: Default::default(),
        };
        scope.context.tenant_id = Some("tenant".into());
        let keys = BTreeMap::from([("first".into(), [7; 32]), ("second".into(), [9; 32])]);
        let first = Keyring::new("first".into(), keys.clone())?;
        let sealed = first.seal(&scope, "space/record/password", b"canary-secret")?;
        assert!(!sealed.windows(13).any(|v| v == b"canary-secret"));
        assert_ne!(
            sealed,
            first.seal(&scope, "space/record/password", b"canary-secret")?
        );
        let rotated = Keyring::new("second".into(), keys)?;
        scope.revision = "v2".into();
        assert_eq!(
            rotated.open(&scope, "space/record/password", &sealed)?,
            b"canary-secret"
        );
        assert!(
            rotated
                .open(&scope, "space/other/password", &sealed)
                .is_err()
        );
        scope.context.tenant_id = Some("other".into());
        assert!(
            rotated
                .open(&scope, "space/record/password", &sealed)
                .is_err()
        );
        scope.context.tenant_id = Some("tenant".into());
        scope.source_id = "other".into();
        assert!(
            rotated
                .open(&scope, "space/record/password", &sealed)
                .is_err()
        );
        scope.source_id = "source".into();
        let mut corrupted = sealed;
        *corrupted.last_mut().unwrap() ^= 1;
        assert!(
            rotated
                .open(&scope, "space/record/password", &corrupted)
                .is_err()
        );
        for size in 0..corrupted.len() {
            assert!(
                rotated
                    .open(&scope, "space/record/password", &corrupted[..size])
                    .is_err()
            );
        }
        Ok(())
    }
}
