//! SSH 后端的类型化 ProgramGraph Capability。

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use rudi::Singleton;
use serde_json::Value;
use studio::capability::{CapabilityProvider, DynCapabilityProvider};
use studio::{CapabilityContract, CapabilityOperationContract, EffectKind, ValueType};

use crate::{
    contract::{ApplySshTemplateRequest, RunSshCommandsRequest},
    state,
};

#[derive(Clone, Debug, Default)]
pub struct SshCapability;

#[async_trait]
impl CapabilityProvider for SshCapability {
    fn contract(&self) -> CapabilityContract {
        CapabilityContract {
            canonical_id: "aio.ssh".to_owned(),
            operations: BTreeMap::from([
                (
                    "dashboard".to_owned(),
                    operation(BTreeMap::new(), vec![EffectKind::DatabaseRead]),
                ),
                (
                    "apply_template".to_owned(),
                    operation(
                        BTreeMap::from([("request".to_owned(), ValueType::Any)]),
                        vec![EffectKind::DatabaseWrite],
                    ),
                ),
                (
                    "run_commands".to_owned(),
                    operation(
                        BTreeMap::from([("request".to_owned(), ValueType::Any)]),
                        vec![EffectKind::Secret, EffectKind::Capability],
                    ),
                ),
            ]),
        }
    }

    async fn execute(&self, operation: &str, input: Value) -> Result<Value> {
        let service = state::service()?;
        match operation {
            "dashboard" => serde_json::to_value(service.dashboard().await?).map_err(Into::into),
            "apply_template" => {
                let request = serde_json::from_value::<ApplySshTemplateRequest>(input)
                    .context("解析 SSH 模板 Capability 输入失败")?;
                serde_json::to_value(service.apply_template(request).await?).map_err(Into::into)
            }
            "run_commands" => {
                let request = serde_json::from_value::<RunSshCommandsRequest>(input)
                    .context("解析 SSH 命令 Capability 输入失败")?;
                serde_json::to_value(service.run_commands(request).await?).map_err(Into::into)
            }
            _ => bail!("SSH Capability operation 未注册: {operation}"),
        }
    }
}

fn operation(
    inputs: BTreeMap<String, ValueType>,
    effects: Vec<EffectKind>,
) -> CapabilityOperationContract {
    CapabilityOperationContract {
        inputs,
        outputs: BTreeMap::from([("result".to_owned(), ValueType::Any)]),
        effects,
    }
}

fn bind_ssh_capability(provider: SshCapability) -> DynCapabilityProvider {
    Arc::new(provider)
}

#[Singleton(name = "ssh-capability", binds = [bind_ssh_capability])]
fn ssh_capability_provider() -> SshCapability {
    SshCapability
}
