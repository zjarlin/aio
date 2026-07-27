//! Rig Agent 的强类型母语语义提示适配器。

use std::{collections::BTreeMap, env};

use anyhow::{Context, bail};
use async_trait::async_trait;
use nature_compiler::{
    Blueprint, CompilerCatalog, DescriptorEncoder, FieldType, InferenceDecision, InferenceEngine,
    InferenceMetrics, InferenceMode, InferenceResult, MotherTongueInferenceEngine,
    OperationPlanStep, SemanticDescriptor,
};
use rig::providers::openai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-5.2";

/// Agent 只能返回的语义提示，不包含 code、Rust、SQL 或路径。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
struct SemanticHints {
    elements: Vec<SemanticHint>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
struct SemanticHint {
    native_name: String,
    english_stem: String,
    field_type: Option<InferredFieldType>,
}

struct ObservedSemanticHints {
    hints: SemanticHints,
    metrics: InferenceMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum InferredFieldType {
    String,
    Integer,
    Decimal,
    Boolean,
    Timestamp,
    Json,
}

impl From<InferredFieldType> for FieldType {
    fn from(value: InferredFieldType) -> Self {
        match value {
            InferredFieldType::String => Self::String,
            InferredFieldType::Integer => Self::Integer,
            InferredFieldType::Decimal => Self::Decimal,
            InferredFieldType::Boolean => Self::Boolean,
            InferredFieldType::Timestamp => Self::Timestamp,
            InferredFieldType::Json => Self::Json,
        }
    }
}

#[derive(Clone, Debug)]
struct AgentConfig {
    api_key: String,
    api_base: String,
    model: String,
    protocol: AgentProtocol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgentProtocol {
    Responses,
    ChatCompletions,
}

impl AgentProtocol {
    fn from_env() -> anyhow::Result<Self> {
        match first_env(["AZ_AIO_NATURE_AGENT_PROTOCOL"]).as_deref() {
            None | Some("responses") => Ok(Self::Responses),
            Some("chat_completions") => Ok(Self::ChatCompletions),
            Some(value) => bail!("不支持的 nature Agent 协议: {value}"),
        }
    }

    fn engine(self) -> &'static str {
        match self {
            Self::Responses => "rig.openai.responses",
            Self::ChatCompletions => "rig.openai.chat_completions",
        }
    }
}

/// 先生成基础 Blueprint，再用 Rig 的强类型语义提示增强英文 stem 与歧义类型。
#[derive(Clone, Debug)]
pub struct NatureInferenceAgent {
    config: Option<AgentConfig>,
}

impl NatureInferenceAgent {
    pub fn from_env() -> anyhow::Result<Self> {
        let Some(api_key) = first_env(["OPENAI_API_KEY", "API_KEY"]) else {
            return Ok(Self { config: None });
        };
        let api_base = first_env(["OPENAI_BASE_URL", "OPENAI_BASEURL", "API_BASEURL"])
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
        let api_base = normalize_api_base(&api_base)?;
        let model = first_env(["AZ_AIO_NATURE_AGENT_MODEL", "OPENAI_MODEL"])
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let protocol = AgentProtocol::from_env()?;
        Ok(Self {
            config: Some(AgentConfig {
                api_key,
                api_base,
                model,
                protocol,
            }),
        })
    }

    #[cfg(test)]
    fn without_remote() -> Self {
        Self { config: None }
    }

    async fn infer_hints(
        &self,
        source_text: &str,
        catalog: &CompilerCatalog,
    ) -> anyhow::Result<Option<ObservedSemanticHints>> {
        let Some(config) = self.config.as_ref() else {
            return Ok(None);
        };
        let prompt = inference_prompt(source_text, catalog);
        let response = match config.protocol {
            AgentProtocol::Responses => {
                let client = openai::Client::builder()
                    .api_key(&config.api_key)
                    .base_url(&config.api_base)
                    .build()
                    .context("创建 nature Rig Responses client 失败")?;
                client
                    .extractor::<SemanticHints>(&config.model)
                    .preamble(inference_contract())
                    .max_tokens(4_096)
                    .retries(2)
                    .build()
                    .extract_with_usage(&prompt)
                    .await
            }
            AgentProtocol::ChatCompletions => {
                let client = openai::CompletionsClient::builder()
                    .api_key(&config.api_key)
                    .base_url(&config.api_base)
                    .build()
                    .context("创建 nature Rig Chat Completions client 失败")?;
                client
                    .extractor::<SemanticHints>(&config.model)
                    .preamble(inference_contract())
                    .max_tokens(4_096)
                    .retries(2)
                    .build()
                    .extract_with_usage(&prompt)
                    .await
            }
        }
        .with_context(|| {
            format!(
                "Rig 母语语义推导失败: model={}, protocol={}",
                config.model,
                config.protocol.engine()
            )
        })?;
        Ok(Some(ObservedSemanticHints {
            hints: response.data,
            metrics: InferenceMetrics {
                engine: config.protocol.engine().to_string(),
                mode: InferenceMode::Remote,
                model: Some(config.model.clone()),
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                total_tokens: response.usage.total_tokens,
                cached_input_tokens: response.usage.cached_input_tokens,
            },
        }))
    }
}

#[async_trait]
impl InferenceEngine for NatureInferenceAgent {
    async fn infer(
        &self,
        source_text: &str,
        previous_blueprint: Option<&Blueprint>,
        catalog: &CompilerCatalog,
    ) -> anyhow::Result<InferenceResult> {
        let fallback = MotherTongueInferenceEngine;
        let mut result = fallback
            .infer(source_text, previous_blueprint, catalog)
            .await?;
        let Some(observed) = self.infer_hints(source_text, catalog).await? else {
            return Ok(result);
        };
        apply_semantic_hints(&mut result.blueprint, previous_blueprint, observed.hints);
        result.metrics = observed.metrics;
        Ok(result)
    }
}

fn apply_semantic_hints(
    blueprint: &mut Blueprint,
    previous_blueprint: Option<&Blueprint>,
    hints: SemanticHints,
) {
    let hints = hints
        .elements
        .into_iter()
        .map(|hint| (hint.native_name.clone(), hint))
        .collect::<BTreeMap<_, _>>();
    let previous = previous_blueprint
        .map(descriptors_by_native_name)
        .unwrap_or_default();
    let current = descriptors_by_native_name(blueprint);
    let mut encoder = DescriptorEncoder::default();
    let mut replacements = BTreeMap::new();
    for (native_name, descriptor) in current {
        if let Some(previous_descriptor) = previous.get(&native_name) {
            encoder.reserve(previous_descriptor);
            replacements.insert(native_name, previous_descriptor.clone());
            continue;
        }
        let stem = hints
            .get(&native_name)
            .map(|hint| hint.english_stem.as_str())
            .unwrap_or(&descriptor.english_stem);
        replacements.insert(native_name.clone(), encoder.describe(&native_name, stem));
    }
    replace_descriptors(blueprint, &replacements);
    for definition in &mut blueprint.structs {
        for field in &mut definition.fields {
            if let Some(field_type) = hints
                .get(&field.descriptor.native_name)
                .and_then(|hint| hint.field_type)
            {
                field.field_type = field_type.into();
            }
        }
    }
    for hint in hints.values() {
        blueprint.inference_decisions.push(InferenceDecision {
            subject: hint.native_name.clone(),
            decision: format!("Rig 推导英文语义 {}", hint.english_stem),
            reused: false,
        });
    }
}

fn descriptors_by_native_name(blueprint: &Blueprint) -> BTreeMap<String, SemanticDescriptor> {
    let mut descriptors = BTreeMap::new();
    for requirement in &blueprint.requirements {
        insert_descriptor(&mut descriptors, &requirement.descriptor);
    }
    for definition in &blueprint.structs {
        insert_descriptor(&mut descriptors, &definition.descriptor);
        for field in &definition.fields {
            insert_descriptor(&mut descriptors, &field.descriptor);
        }
    }
    for function in &blueprint.functions {
        insert_descriptor(&mut descriptors, &function.descriptor);
        insert_descriptor(&mut descriptors, &function.input_model);
        insert_descriptor(&mut descriptors, &function.output_model);
    }
    for capability in &blueprint.capabilities {
        insert_descriptor(&mut descriptors, &capability.descriptor);
    }
    for binding in &blueprint.bindings {
        insert_descriptor(&mut descriptors, &binding.owner);
        insert_descriptor(&mut descriptors, &binding.field);
        insert_descriptor(&mut descriptors, &binding.source);
    }
    insert_descriptor(&mut descriptors, &blueprint.application.domain.descriptor);
    for model in &blueprint.application.domain.models {
        insert_descriptor(&mut descriptors, model);
    }
    for operation in &blueprint.application.operations {
        insert_descriptor(&mut descriptors, &operation.descriptor);
        insert_descriptor(&mut descriptors, &operation.model);
        for step in &operation.steps {
            if let OperationPlanStep::InvokeCapability { capability } = step {
                insert_descriptor(&mut descriptors, capability);
            }
        }
    }
    for interface in &blueprint.application.interfaces {
        insert_descriptor(&mut descriptors, &interface.descriptor);
        insert_descriptor(&mut descriptors, &interface.operation);
    }
    for view in &blueprint.application.views {
        insert_descriptor(&mut descriptors, &view.descriptor);
        insert_descriptor(&mut descriptors, &view.model);
        for field in &view.fields {
            insert_descriptor(&mut descriptors, field);
        }
        for action in &view.actions {
            insert_descriptor(&mut descriptors, &action.descriptor);
            insert_descriptor(&mut descriptors, &action.operation);
        }
    }
    insert_descriptor(
        &mut descriptors,
        &blueprint.application.navigation.descriptor,
    );
    insert_descriptor(
        &mut descriptors,
        &blueprint.application.navigation.default_view,
    );
    for entry in &blueprint.application.navigation.entries {
        insert_descriptor(&mut descriptors, &entry.descriptor);
        insert_descriptor(&mut descriptors, &entry.view);
        for permission in &entry.permissions {
            insert_descriptor(&mut descriptors, permission);
        }
    }
    for permission in &blueprint.application.permissions {
        insert_descriptor(&mut descriptors, &permission.descriptor);
        for operation in &permission.operations {
            insert_descriptor(&mut descriptors, operation);
        }
    }
    descriptors
}

fn insert_descriptor(
    target: &mut BTreeMap<String, SemanticDescriptor>,
    descriptor: &SemanticDescriptor,
) {
    target.insert(descriptor.native_name.clone(), descriptor.clone());
}

fn replace_descriptors(
    blueprint: &mut Blueprint,
    replacements: &BTreeMap<String, SemanticDescriptor>,
) {
    for requirement in &mut blueprint.requirements {
        replace_descriptor(&mut requirement.descriptor, replacements);
    }
    for definition in &mut blueprint.structs {
        replace_descriptor(&mut definition.descriptor, replacements);
        for field in &mut definition.fields {
            replace_descriptor(&mut field.descriptor, replacements);
        }
    }
    for function in &mut blueprint.functions {
        replace_descriptor(&mut function.descriptor, replacements);
        replace_descriptor(&mut function.input_model, replacements);
        replace_descriptor(&mut function.output_model, replacements);
    }
    for capability in &mut blueprint.capabilities {
        replace_descriptor(&mut capability.descriptor, replacements);
    }
    for binding in &mut blueprint.bindings {
        replace_descriptor(&mut binding.owner, replacements);
        replace_descriptor(&mut binding.field, replacements);
        replace_descriptor(&mut binding.source, replacements);
    }
    replace_descriptor(&mut blueprint.application.domain.descriptor, replacements);
    for model in &mut blueprint.application.domain.models {
        replace_descriptor(model, replacements);
    }
    for operation in &mut blueprint.application.operations {
        replace_descriptor(&mut operation.descriptor, replacements);
        replace_descriptor(&mut operation.model, replacements);
        for step in &mut operation.steps {
            if let OperationPlanStep::InvokeCapability { capability } = step {
                replace_descriptor(capability, replacements);
            }
        }
    }
    for interface in &mut blueprint.application.interfaces {
        replace_descriptor(&mut interface.descriptor, replacements);
        replace_descriptor(&mut interface.operation, replacements);
    }
    for view in &mut blueprint.application.views {
        replace_descriptor(&mut view.descriptor, replacements);
        replace_descriptor(&mut view.model, replacements);
        for field in &mut view.fields {
            replace_descriptor(field, replacements);
        }
        for action in &mut view.actions {
            replace_descriptor(&mut action.descriptor, replacements);
            replace_descriptor(&mut action.operation, replacements);
        }
    }
    replace_descriptor(
        &mut blueprint.application.navigation.descriptor,
        replacements,
    );
    replace_descriptor(
        &mut blueprint.application.navigation.default_view,
        replacements,
    );
    for entry in &mut blueprint.application.navigation.entries {
        replace_descriptor(&mut entry.descriptor, replacements);
        replace_descriptor(&mut entry.view, replacements);
        for permission in &mut entry.permissions {
            replace_descriptor(permission, replacements);
        }
    }
    for permission in &mut blueprint.application.permissions {
        replace_descriptor(&mut permission.descriptor, replacements);
        for operation in &mut permission.operations {
            replace_descriptor(operation, replacements);
        }
    }
    blueprint.application.refresh_derived_paths();
}

fn replace_descriptor(
    descriptor: &mut SemanticDescriptor,
    replacements: &BTreeMap<String, SemanticDescriptor>,
) {
    if let Some(replacement) = replacements.get(&descriptor.native_name) {
        *descriptor = replacement.clone();
    }
}

fn normalize_api_base(value: &str) -> anyhow::Result<String> {
    let value = value.trim().trim_end_matches('/');
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        bail!("nature Agent API base 必须以 http:// 或 https:// 开头");
    }
    if value.ends_with("/v1") {
        Ok(value.to_string())
    } else {
        Ok(format!("{value}/v1"))
    }
}

fn first_env<const N: usize>(names: [&str; N]) -> Option<String> {
    names
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}

fn inference_prompt(source_text: &str, catalog: &CompilerCatalog) -> String {
    let operation_capabilities = catalog
        .operation_capabilities
        .iter()
        .map(|capability| capability.descriptor.native_name.as_str())
        .collect::<Vec<_>>()
        .join("、");
    let view_components = catalog
        .view_components
        .iter()
        .map(|capability| capability.descriptor.native_name.as_str())
        .collect::<Vec<_>>()
        .join("、");
    format!(
        "{source_text}\n\n宿主可用的母语操作能力：{operation_capabilities}\n宿主可用的母语界面组件：{view_components}"
    )
}

fn inference_contract() -> &'static str {
    r#"
你只负责从中文需求中推导语义提示。

- native_name 必须逐字引用输入中已经出现的母语名称。
- english_stem 只表达英文领域语义，不得包含 Rust 大小写、路径、文件名或依赖。
- field_type 仅在元素确实是字段时填写。
- 不得返回 code、kind、表名、字典编码、Provider ID、Rust、SQL、Rhai、WASM 或文件路径。
- 不得补充输入中不存在的业务字段、函数或能力。
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_agent_uses_controlled_fallback() -> anyhow::Result<()> {
        let result = NatureInferenceAgent::without_remote()
            .infer(
                include_str!("../../../crates/generated/nature/blueprint-source.txt"),
                None,
                &CompilerCatalog::with_fixture_map(),
            )
            .await?;

        assert_eq!(
            result.blueprint.structs[0].descriptor.code,
            "environment_telemetry"
        );
        Ok(())
    }

    #[test]
    fn contract_forbids_generated_program_text() {
        let contract = inference_contract();
        assert!(contract.contains("不得返回 code"));
        assert!(contract.contains("Rust"));
        assert!(contract.contains("SQL"));
    }
}
