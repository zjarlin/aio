use std::{collections::BTreeMap, sync::Arc};

pub(crate) use az_aio_nature_generated::enums::{
    ComponentBehavior, ComponentPropertyKind, ComponentShape,
};
use serde::{Deserialize, Serialize};

/// 单个组件属性的编辑和校验约束。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentPropertySpec {
    pub kind: ComponentPropertyKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

/// 组件事件向动作系统暴露的载荷字段。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentEventSpec {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub payload: BTreeMap<String, ComponentPropertyKind>,
}

/// 浏览器渲染器使用的 HTML 和样式映射。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub html_tag: String,
    pub class_name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variants: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_variant: Option<String>,
    #[serde(default, skip_serializing_if = "ComponentBehavior::is_generic")]
    pub behavior: ComponentBehavior,
}

/// 组件实例提供给查询索引的完整定义。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub shape: ComponentShape,
    pub spec: ComponentSpec,
}

/// 发送给低代码编辑器和浏览器 renderer 的组件完整元数据。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentCatalogEntry {
    pub shape: ComponentShape,
    pub spec: ComponentSpec,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, ComponentPropertySpec>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, ComponentEventSpec>,
}

/// Rudi 管理的远程组件实例。
pub trait RemoteComponent: Send + Sync {
    fn definition(&self) -> ComponentDefinition;

    /// 提供给自然编译器的母语名称和别名。
    fn semantic_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn properties(&self) -> BTreeMap<String, ComponentPropertySpec> {
        BTreeMap::new()
    }

    fn events(&self) -> BTreeMap<String, ComponentEventSpec> {
        BTreeMap::new()
    }
}

pub type DynRemoteComponent = Arc<dyn RemoteComponent>;

pub(crate) fn bind_component<T>(component: T) -> DynRemoteComponent
where
    T: RemoteComponent + 'static,
{
    Arc::new(component)
}

pub(crate) fn spec(html_tag: &str, class_name: &str) -> ComponentSpec {
    ComponentSpec {
        html_tag: html_tag.to_string(),
        class_name: class_name.to_string(),
        variants: BTreeMap::new(),
        default_variant: None,
        behavior: ComponentBehavior::Generic,
    }
}

pub(crate) fn spec_with_behavior(
    html_tag: &str,
    class_name: &str,
    behavior: ComponentBehavior,
) -> ComponentSpec {
    ComponentSpec {
        html_tag: html_tag.to_string(),
        class_name: class_name.to_string(),
        variants: BTreeMap::new(),
        default_variant: None,
        behavior,
    }
}

pub(crate) fn spec_with_variants(
    html_tag: &str,
    class_name: &str,
    variants: &[(&str, &str)],
    default_variant: &str,
    behavior: ComponentBehavior,
) -> ComponentSpec {
    let variants = variants
        .iter()
        .map(|(name, class_name)| (name.to_string(), class_name.to_string()))
        .collect();
    ComponentSpec {
        html_tag: html_tag.to_string(),
        class_name: class_name.to_string(),
        variants,
        default_variant: Some(default_variant.to_string()),
        behavior,
    }
}

pub(crate) const fn property(kind: ComponentPropertyKind, required: bool) -> ComponentPropertySpec {
    ComponentPropertySpec {
        kind,
        required,
        choices: Vec::new(),
    }
}

pub(crate) fn properties(
    values: &[(&str, ComponentPropertySpec)],
) -> BTreeMap<String, ComponentPropertySpec> {
    values
        .iter()
        .map(|(name, property)| ((*name).to_string(), property.clone()))
        .collect()
}

pub(crate) fn choice_property(choices: &[&str]) -> ComponentPropertySpec {
    ComponentPropertySpec {
        kind: ComponentPropertyKind::Choice,
        required: false,
        choices: choices.iter().map(|choice| (*choice).to_string()).collect(),
    }
}

pub(crate) fn event(payload: &[(&str, ComponentPropertyKind)]) -> ComponentEventSpec {
    ComponentEventSpec {
        payload: payload
            .iter()
            .map(|(name, kind)| ((*name).to_string(), *kind))
            .collect(),
    }
}

pub(crate) fn events(
    values: &[(&str, ComponentEventSpec)],
) -> BTreeMap<String, ComponentEventSpec> {
    values
        .iter()
        .map(|(name, event)| ((*name).to_string(), event.clone()))
        .collect()
}
