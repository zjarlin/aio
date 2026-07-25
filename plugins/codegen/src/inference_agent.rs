//! Rig Agent 的强类型母语语义提示适配器。

use std::{collections::BTreeMap, env};

use anyhow::{Context, bail};
use async_trait::async_trait;
use nature_compiler::{
    Blueprint, DescriptorEncoder, FieldType, InferenceDecision, InferenceEngine, InferenceResult,
    MotherTongueInferenceEngine, SemanticDescriptor,
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
        Ok(Self {
            config: Some(AgentConfig {
                api_key,
                api_base,
                model,
            }),
        })
    }

    #[cfg(test)]
    fn without_remote() -> Self {
        Self { config: None }
    }

    async fn infer_hints(&self, source_text: &str) -> anyhow::Result<Option<SemanticHints>> {
        let Some(config) = self.config.as_ref() else {
            return Ok(None);
        };
        let client = openai::Client::builder()
            .api_key(&config.api_key)
            .base_url(&config.api_base)
            .build()
            .context("创建 nature Rig client 失败")?;
        let extractor = client
            .extractor::<SemanticHints>(&config.model)
            .preamble(inference_contract())
            .max_tokens(4_096)
            .retries(2)
            .build();
        extractor
            .extract(source_text)
            .await
            .context("Rig 母语语义推导失败")
            .map(Some)
    }
}

#[async_trait]
impl InferenceEngine for NatureInferenceAgent {
    async fn infer(
        &self,
        source_text: &str,
        previous_blueprint: Option<&Blueprint>,
    ) -> anyhow::Result<InferenceResult> {
        let fallback = MotherTongueInferenceEngine;
        let mut result = fallback.infer(source_text, previous_blueprint).await?;
        let Some(hints) = self.infer_hints(source_text).await? else {
            return Ok(result);
        };
        apply_semantic_hints(&mut result.blueprint, previous_blueprint, hints);
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
