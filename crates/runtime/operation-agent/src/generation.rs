//! 使用 Rig 生成低代码 operation 草稿。

use std::env;

use anyhow::{Context, bail};
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
    /// 纯计算 Rhai 源码，最后一个表达式必须是可序列化返回值。
    pub source_text: String,
    /// JSON Schema object，描述请求 body 和 query 的约束。
    pub input_schema: Value,
    /// JSON Schema object，描述 Rhai 返回值。
    pub output_schema: Value,
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
你负责为 AIO 低代码 engine 生成可立即试运行的 operation 草稿。

必须遵守以下契约：
- operation_key 只能包含 ASCII 字母、数字、点、横线和下划线。
- method 只能是 GET 或 POST。
- source_text 必须是合法 Rhai，最后一个表达式必须返回 JSON 可序列化的值。
- 可用变量只有 request、body、query、operation_key、method。
- query 的每个字段都是字符串数组，例如 query.tag[0]。
- 不得使用 eval、文件系统、网络、数据库、shell、环境变量或未声明函数。
- input_schema 和 output_schema 必须是 JSON Schema object。
- 对缺失字段给出显式判断或领域默认值，不要生成静默吞错的占位逻辑。

合法 source_text 示例：
#{ operation: operation_key, device_id: body.deviceId, start_time: query.startTime[0] }
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_contract_keeps_execution_authority_in_host() {
        let contract = operation_generation_contract();

        // Agent 只能生成受控 Rhai，不能自行扩展系统能力或执行器类型。
        assert!(contract.contains("source_text"));
        assert!(contract.contains("不得使用 eval"));
        assert!(!contract.contains("executor_kind"));
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
