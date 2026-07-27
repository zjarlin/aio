//! 使用 Rig 生成低代码 operation 草稿。

use std::env;

use anyhow::{Context, bail};
use az_engine::operation::{
    OperationDraft, OperationExecutorDefinition, OperationPlan, OperationPlanStep,
};
use rig::providers::openai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_OPERATION_MODEL: &str = "gpt-5.2";

/// Agent 生成的受控 operation 草稿。
///
/// 执行器类型、能力策略和资源限制不交给模型决定，由 engine 宿主统一补齐。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GeneratedOperationDraft {
    /// 稳定的 operation 标识，只使用字母、数字、点、横线和下划线。
    pub operation_key: String,
    /// 面向管理端展示的名称。
    pub display_name: String,
    /// operation 的用途和输入输出说明。
    pub description: String,
    /// HTTP 方法，首版只能是 GET 或 POST。
    pub method: String,
    /// 领域模型的母语名称，由宿主规范化后进入 Blueprint。
    pub model_name: String,
    /// 受控执行步骤，不包含脚本源码。
    pub steps: Vec<GeneratedOperationPlanStep>,
    /// JSON Schema object，描述请求 body 和 query 的约束。
    pub input_schema: Value,
    /// JSON Schema object，描述 Rhai 返回值。
    pub output_schema: Value,
}

/// Agent 可选择的有限操作步骤。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedOperationPlanStep {
    ValidateInput,
    QueryRecords,
    LoadRecord,
    CreateRecord,
    UpdateRecord,
    DeleteRecord,
    ReturnResult,
}

impl GeneratedOperationDraft {
    /// 把 Agent 推导转换为 Engine 强类型草稿，执行策略由宿主补齐。
    pub fn into_operation_draft(self) -> OperationDraft {
        let steps = self
            .steps
            .into_iter()
            .map(|step| match step {
                GeneratedOperationPlanStep::ValidateInput => OperationPlanStep::ValidateInput,
                GeneratedOperationPlanStep::QueryRecords => OperationPlanStep::QueryRecords,
                GeneratedOperationPlanStep::LoadRecord => OperationPlanStep::LoadRecord,
                GeneratedOperationPlanStep::CreateRecord => OperationPlanStep::CreateRecord,
                GeneratedOperationPlanStep::UpdateRecord => OperationPlanStep::UpdateRecord,
                GeneratedOperationPlanStep::DeleteRecord => OperationPlanStep::DeleteRecord,
                GeneratedOperationPlanStep::ReturnResult => OperationPlanStep::ReturnResult,
            })
            .collect();
        OperationDraft {
            operation_key: self.operation_key,
            display_name: self.display_name,
            description: self.description,
            method: self.method,
            executor: OperationExecutorDefinition::Plan(OperationPlan {
                model_name: self.model_name,
                steps,
            }),
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            capability_policy: serde_json::json!({}),
            timeout_ms: 3_000,
            generated_by_model: None,
        }
    }
}

/// operation Agent 的生成结果。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationVibeResult {
    pub draft: GeneratedOperationDraft,
    pub model: String,
}

/// 使用 Rig typed extractor 的 operation Vibe Agent。
#[derive(Clone)]
pub struct OperationVibeAgent {
    api_key: String,
    api_base: String,
    model: String,
}

impl OperationVibeAgent {
    /// 使用显式 OpenAI 兼容配置创建 Agent。
    pub fn new(
        api_key: impl Into<String>,
        api_base: impl Into<String>,
        model: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            bail!("operation Agent API key 不能为空");
        }
        let api_base = normalize_api_base(&api_base.into())?;
        let model = model.into();
        if model.trim().is_empty() {
            bail!("operation Agent model 不能为空");
        }
        Ok(Self {
            api_key,
            api_base,
            model,
        })
    }

    /// 在存在 provider 凭据时从环境创建 Agent；未配置时返回 `None`。
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(api_key) = first_env(["OPENAI_API_KEY", "API_KEY"]) else {
            return Ok(None);
        };
        let api_base = match first_env(["OPENAI_BASE_URL", "OPENAI_BASEURL", "API_BASEURL"]) {
            Some(value) => value,
            None => DEFAULT_API_BASE.to_string(),
        };
        let model = match first_env(["AZ_AIO_OPERATION_AGENT_MODEL", "OPENAI_MODEL"]) {
            Some(value) => value,
            None => DEFAULT_OPERATION_MODEL.to_string(),
        };
        Self::new(api_key, api_base, model).map(Some)
    }

    /// 把自然语言需求提取成强类型 operation 草稿。
    pub async fn generate(&self, prompt: &str) -> anyhow::Result<OperationVibeResult> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            bail!("operation Vibe prompt 不能为空");
        }

        let client = openai::Client::builder()
            .api_key(&self.api_key)
            .base_url(&self.api_base)
            .build()
            .context("创建 Rig OpenAI client 失败")?;
        let extractor = client
            .extractor::<GeneratedOperationDraft>(&self.model)
            .preamble(operation_generation_contract())
            .max_tokens(4_096)
            .retries(2)
            .build();
        let draft = extractor
            .extract(prompt)
            .await
            .context("Rig operation Agent 生成失败")?;
        Ok(OperationVibeResult {
            draft,
            model: self.model.clone(),
        })
    }

    /// 返回当前 Agent 使用的模型 ID。
    pub fn model(&self) -> &str {
        &self.model
    }
}

fn normalize_api_base(value: &str) -> anyhow::Result<String> {
    let value = value.trim().trim_end_matches('/');
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        bail!("operation Agent API base 必须以 http:// 或 https:// 开头");
    }
    if value.ends_with("/v1") {
        Ok(value.to_string())
    } else {
        Ok(format!("{value}/v1"))
    }
}

fn first_env<const N: usize>(names: [&str; N]) -> Option<String> {
    names.into_iter().find_map(|name| {
        env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
}

fn operation_generation_contract() -> &'static str {
    r#"
你负责为 AIO 低代码 engine 生成可验证的强类型 operation 计划。

必须遵守以下契约：
- operation_key 只能包含 ASCII 字母、数字、点、横线和下划线。
- method 只能是 GET 或 POST。
- model_name 必须逐字引用输入中出现的母语模型名称。
- steps 只能使用结构化输出 Schema 中声明的步骤，最后一步必须是 return_result。
- 不得返回 Rust、SQL、Rhai、WASM、文件系统、网络、shell、环境变量或任意源码。
- input_schema 和 output_schema 必须是 JSON Schema object。
- 对写入操作必须先 validate_input，再执行 create_record 或 update_record。
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_contract_keeps_execution_authority_in_host() {
        let contract = operation_generation_contract();

        // Agent 只能生成受控 Rhai，不能自行扩展系统能力或执行器类型。
        assert!(contract.contains("steps"));
        assert!(contract.contains("不得返回 Rust"));
        assert!(!contract.contains("source_text"));
    }

    #[test]
    fn normalizes_openai_compatible_api_base() {
        let base = normalize_api_base("https://api.example.com/");
        let base = match base {
            Ok(value) => value,
            Err(error) => panic!("合法 API base 应通过校验: {error}"),
        };

        // OpenAI 兼容 provider 的根地址必须稳定落到 /v1。
        assert_eq!(base, "https://api.example.com/v1");
    }

    #[test]
    fn rejects_blank_model_without_network_access() {
        let result = OperationVibeAgent::new(
            "test-key",
            "https://api.openai.com/v1",
            " ",
        );

        // 空模型不能延迟到远程请求阶段才暴露。
        assert!(result.is_err());
    }
}
