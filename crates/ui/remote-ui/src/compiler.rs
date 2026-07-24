use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use crate::{
    ComponentIndex, ComponentNode, ComponentPropertyKind, ComponentShape, PAGE_SCHEMA_VERSION,
    PageDefinition, PropertyValue, UiNode, UiOp,
};

const DEFAULT_MAX_DEPTH: usize = 64;

/// 把声明式页面校验并编译为跨渲染器操作流。
pub struct PageCompiler<'a> {
    components: &'a ComponentIndex,
    max_depth: usize,
}

impl<'a> PageCompiler<'a> {
    #[must_use]
    pub const fn new(components: &'a ComponentIndex) -> Self {
        Self {
            components,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// 使用已经加载的数据快照编译页面。
    pub fn compile(&self, page: &PageDefinition, data: &Value) -> Result<Vec<UiOp>> {
        ensure!(
            page.schema_version == PAGE_SCHEMA_VERSION,
            "不支持的页面 schema_version: {}",
            page.schema_version
        );
        ensure!(!page.key.trim().is_empty(), "页面 key 不能为空");
        ensure!(!page.title.trim().is_empty(), "页面 title 不能为空");

        let _data_source_ids = unique_ids(
            page.data_sources.iter().map(|source| source.id.as_str()),
            "数据源",
        )?;
        let action_ids = unique_ids(page.actions.iter().map(|action| action.id.as_str()), "动作")?;
        for source in &page.data_sources {
            ensure!(
                !source.operation.trim().is_empty(),
                "数据源 operation 不能为空"
            );
        }
        for action in &page.actions {
            ensure!(
                !action.operation.trim().is_empty(),
                "动作 operation 不能为空"
            );
        }

        let mut state = CompileState {
            data,
            action_ids,
            node_ids: BTreeSet::new(),
            operations: Vec::new(),
        };
        self.compile_node(&page.root, 0, &mut state)?;
        Ok(state.operations)
    }

    fn compile_node(
        &self,
        node: &ComponentNode,
        depth: usize,
        state: &mut CompileState<'_>,
    ) -> Result<()> {
        ensure!(depth < self.max_depth, "页面组件树嵌套深度超过限制");
        let component = self.components.resolve_canonical(&node.component)?;
        if let Some(id) = node.id.as_deref() {
            ensure!(!id.trim().is_empty(), "组件 id 不能为空字符串");
            ensure!(state.node_ids.insert(id.to_string()), "组件 id 重复: {id}");
        }
        validate_shape(component.shape(), node)?;

        let mut attributes = BTreeMap::new();
        for (name, value) in &node.properties {
            let property = component
                .properties()
                .get(name)
                .with_context(|| format!("组件 {} 不支持属性 {name}", node.component))?;
            let resolved = resolve_value(value, state.data)?;
            validate_property(
                name,
                &resolved,
                property.kind,
                &property.choices,
                &state.action_ids,
            )?;
            attributes.insert(name.clone(), resolved);
        }
        for (name, property) in component.properties() {
            if property.required {
                ensure!(
                    node.properties.contains_key(name),
                    "组件 {} 缺少必填属性 {name}",
                    node.component
                );
            }
        }
        let content = node
            .content
            .as_ref()
            .map(|value| resolve_value(value, state.data))
            .transpose()?;
        let ui_node = UiNode {
            kind: component.canonical_id().to_string(),
            id: node.id.clone(),
            attributes,
            content,
        };

        if node.children.is_empty() {
            state.operations.push(UiOp::Leaf { node: ui_node });
            return Ok(());
        }

        state.operations.push(UiOp::Open { node: ui_node });
        for child in &node.children {
            self.compile_node(child, depth + 1, state)?;
        }
        state.operations.push(UiOp::Close {
            kind: component.canonical_id().to_string(),
        });
        Ok(())
    }
}

struct CompileState<'a> {
    data: &'a Value,
    action_ids: BTreeSet<String>,
    node_ids: BTreeSet<String>,
    operations: Vec<UiOp>,
}

fn unique_ids<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for value in values {
        ensure!(!value.trim().is_empty(), "{label} id 不能为空");
        ensure!(ids.insert(value.to_string()), "{label} id 重复: {value}");
    }
    Ok(ids)
}

fn validate_shape(shape: ComponentShape, node: &ComponentNode) -> Result<()> {
    match shape {
        ComponentShape::Leaf => {
            ensure!(
                node.children.is_empty(),
                "叶子组件不能包含子节点: {}",
                node.component
            );
        }
        ComponentShape::Container => {
            ensure!(
                node.content.is_none(),
                "容器组件不能直接设置 content: {}",
                node.component
            );
        }
        ComponentShape::Dual => {
            ensure!(
                node.content.is_none() || node.children.is_empty(),
                "双形态组件不能同时设置 content 和 children: {}",
                node.component
            );
        }
    }
    Ok(())
}

fn resolve_value(value: &PropertyValue, data: &Value) -> Result<String> {
    let value = match value {
        PropertyValue::Literal { value } => value,
        PropertyValue::Binding { path } => resolve_binding(data, path)?,
    };
    scalar_to_string(value)
}

fn resolve_binding<'a>(data: &'a Value, path: &str) -> Result<&'a Value> {
    ensure!(!path.trim().is_empty(), "数据绑定路径不能为空");
    let mut value = data;
    for segment in path.split('.') {
        value = value
            .get(segment)
            .with_context(|| format!("数据绑定路径不存在: {path}"))?;
    }
    Ok(value)
}

fn scalar_to_string(value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Null => Ok(String::new()),
        Value::Array(_) | Value::Object(_) => bail!("组件属性只接受标量值"),
    }
}

fn validate_property(
    name: &str,
    value: &str,
    kind: ComponentPropertyKind,
    choices: &[String],
    action_ids: &BTreeSet<String>,
) -> Result<()> {
    match kind {
        ComponentPropertyKind::Text => Ok(()),
        ComponentPropertyKind::Boolean => {
            ensure!(
                matches!(value, "true" | "false"),
                "属性 {name} 必须是布尔值"
            );
            Ok(())
        }
        ComponentPropertyKind::Number => {
            value
                .parse::<f64>()
                .with_context(|| format!("属性 {name} 必须是数字"))?;
            Ok(())
        }
        ComponentPropertyKind::Choice => {
            ensure!(
                choices.iter().any(|choice| choice == value),
                "属性 {name} 的选项无效: {value}"
            );
            Ok(())
        }
        ComponentPropertyKind::Action => {
            ensure!(action_ids.contains(value), "组件引用了未声明动作: {value}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rudi::Context as RudiContext;
    use serde_json::json;

    use super::*;
    use crate::{ActionDefinition, PropertyValue};

    fn components() -> Result<Arc<ComponentIndex>> {
        let mut context = RudiContext::auto_register();
        Ok(Arc::new(ComponentIndex::from_context(&mut context)?))
    }

    #[test]
    fn compiles_canonical_component_tree_and_binding() -> Result<()> {
        let components = components()?;
        let section = components.resolve("section")?.canonical_id().to_string();
        let button = components.resolve("button")?.canonical_id().to_string();
        let page = PageDefinition {
            schema_version: PAGE_SCHEMA_VERSION,
            key: "devices".to_string(),
            title: "设备".to_string(),
            root: ComponentNode {
                component: section,
                id: None,
                properties: BTreeMap::new(),
                content: None,
                children: vec![ComponentNode {
                    component: button,
                    id: Some("create-button".to_string()),
                    properties: BTreeMap::from([
                        (
                            "tx".to_string(),
                            PropertyValue::Binding {
                                path: "labels.create".to_string(),
                            },
                        ),
                        ("act".to_string(), PropertyValue::text("create")),
                    ]),
                    content: None,
                    children: Vec::new(),
                }],
            },
            data_sources: Vec::new(),
            actions: vec![ActionDefinition {
                id: "create".to_string(),
                operation: "devices.create".to_string(),
                input: BTreeMap::new(),
                confirmation: None,
            }],
        };
        let operations = PageCompiler::new(&components)
            .compile(&page, &json!({ "labels": { "create": "创建设备" } }))?;

        // 正式页面配置经校验后生成完整的容器操作流。
        assert_eq!(operations.len(), 3);
        let UiOp::Leaf { node } = &operations[1] else {
            bail!("第二个操作应为按钮叶子节点");
        };
        assert_eq!(node.attributes.get("tx"), Some(&"创建设备".to_string()));
        Ok(())
    }

    #[test]
    fn rejects_unknown_property_and_action() -> Result<()> {
        let components = components()?;
        let button = components.resolve("button")?.canonical_id().to_string();
        let page = PageDefinition {
            schema_version: PAGE_SCHEMA_VERSION,
            key: "invalid".to_string(),
            title: "无效页面".to_string(),
            root: ComponentNode {
                component: button,
                id: None,
                properties: BTreeMap::from([("act".to_string(), PropertyValue::text("missing"))]),
                content: None,
                children: Vec::new(),
            },
            data_sources: Vec::new(),
            actions: Vec::new(),
        };

        let error = PageCompiler::new(&components)
            .compile(&page, &Value::Null)
            .expect_err("未声明动作必须被拒绝");
        assert!(error.to_string().contains("未声明动作"));
        Ok(())
    }
}
