use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 当前声明式页面配置版本。
pub const PAGE_SCHEMA_VERSION: u32 = 1;

/// 可持久化、可版本化的声明式页面定义。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageDefinition {
    pub schema_version: u32,
    pub key: String,
    pub title: String,
    pub root: ComponentNode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_sources: Vec<DataSourceDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDefinition>,
}

/// 页面树中的组件实例。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentNode {
    /// Rudi provider 的编译时模块路径。
    pub component: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<PropertyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ComponentNode>,
}

/// 页面属性只能使用字面量或只读数据路径，不执行任意表达式。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum PropertyValue {
    Literal { value: Value },
    Binding { path: String },
}

impl PropertyValue {
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Literal {
            value: Value::String(value.into()),
        }
    }

    #[must_use]
    pub fn number(value: impl Into<serde_json::Number>) -> Self {
        Self::Literal {
            value: Value::Number(value.into()),
        }
    }
}

/// 由 engine operation 提供数据的只读数据源。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataSourceDefinition {
    pub id: String,
    pub operation: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, PropertyValue>,
}

/// 组件事件可以触发的受控 engine operation。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub operation: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub input: BTreeMap<String, PropertyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<String>,
}
