//! Blueprint 到 AIO 低代码部署清单的确定性 lowering。

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use az_aio_nature_generated::enums::{AdminMenuNodeKind, PageState};
use az_aio_platform::plugin::contract::{AdminMenuNode, AdminMenuSection, AdminMenuTree};
use az_engine::operation::{
    OperationDraft, OperationExecutorDefinition, OperationPlan, OperationPlanStep,
};
use az_engine::page::PageInput;
use az_engine::route::{DeploymentModelInput, RouteDefinitionInput};
use az_engine::{FieldInput, ModelInput};
use az_remote_ui::{
    ActionDefinition, ComponentIndex, ComponentNode, DataSourceDefinition, PAGE_SCHEMA_VERSION,
    PageDefinition, PropertyValue,
};
use nature_compiler::{
    Blueprint, DomainOperation, FieldDefinition, FieldType, HttpMethod,
    OperationPlanStep as BlueprintOperationPlanStep, ValidationRule, ViewDefinition, ViewLayout,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Blueprint 的完整 AIO 物化计划。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationDeployment {
    pub domain_code: String,
    pub models: Vec<DeploymentModelInput>,
    pub operations: Vec<OperationDraft>,
    pub pages: Vec<PageInput>,
    pub routes: Vec<RouteDefinitionInput>,
    pub menu: AdminMenuTree,
}

/// 使用当前 Rudi 组件目录生成 AIO 部署清单。
pub fn lower_application(
    blueprint: &Blueprint,
    components: &ComponentIndex,
) -> Result<ApplicationDeployment> {
    let model = blueprint
        .structs
        .first()
        .context("Blueprint 缺少领域模型")?;
    let deployment_model = DeploymentModelInput {
        model: ModelInput {
            name: model.descriptor.code.clone(),
            display_name: model.descriptor.native_name.clone(),
        },
        fields: model
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| lower_field(field, index))
            .collect::<Result<Vec<_>>>()?,
    };
    let operations = blueprint
        .application
        .operations
        .iter()
        .map(|operation| lower_operation(blueprint, operation))
        .collect::<Result<Vec<_>>>()?;
    let operation_keys = blueprint
        .application
        .operations
        .iter()
        .map(|operation| {
            (
                operation.descriptor.native_name.clone(),
                operation_key(blueprint, operation),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let pages = blueprint
        .application
        .views
        .iter()
        .map(|view| lower_page(blueprint, view, &operation_keys, components))
        .collect::<Result<Vec<_>>>()?;
    let routes = blueprint
        .application
        .interfaces
        .iter()
        .map(|interface| {
            let operation_key = operation_keys
                .get(&interface.operation.native_name)
                .with_context(|| {
                    format!(
                        "接口引用的 Operation 不存在: {}",
                        interface.operation.native_name
                    )
                })?;
            Ok(RouteDefinitionInput {
                method: http_method(interface.method).to_string(),
                path: interface.path.clone(),
                operation_key: operation_key.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ApplicationDeployment {
        domain_code: blueprint.application.domain.descriptor.code.clone(),
        models: vec![deployment_model],
        operations,
        pages,
        routes,
        menu: lower_menu(blueprint),
    })
}

fn lower_field(field: &FieldDefinition, index: usize) -> Result<FieldInput> {
    Ok(FieldInput {
        name: field.descriptor.code.clone(),
        display_name: field.descriptor.native_name.clone(),
        field_type: field_type(&field.field_type).to_string(),
        is_required: field.required,
        expression: None,
        dependency_json: None,
        domain_metadata_json: Some(serde_json::to_string(&field.domain_metadata)?),
        validation_json: Some(serde_json::to_string(&field.validations)?),
        order_index: i32::try_from(index + 1).unwrap_or(i32::MAX) * 10,
    })
}

fn lower_operation(blueprint: &Blueprint, operation: &DomainOperation) -> Result<OperationDraft> {
    let model = blueprint
        .structs
        .iter()
        .find(|model| model.descriptor == operation.model)
        .with_context(|| format!("Operation 模型不存在: {}", operation.model.native_name))?;
    let steps = operation.steps.iter().map(lower_operation_step).collect();
    Ok(OperationDraft {
        operation_key: operation_key(blueprint, operation),
        display_name: operation.descriptor.native_name.clone(),
        description: format!(
            "由 nature-compiler 生成：{}",
            operation.descriptor.native_name
        ),
        method: blueprint
            .application
            .interfaces
            .iter()
            .find(|interface| interface.operation == operation.descriptor)
            .map(|interface| http_method(interface.method).to_string())
            .unwrap_or_else(|| "POST".to_string()),
        executor: OperationExecutorDefinition::Plan(OperationPlan {
            model_name: model.descriptor.code.clone(),
            steps,
        }),
        input_schema: model_schema(&model.fields),
        output_schema: json!({"type": "object"}),
        capability_policy: json!({
            "allow": operation.steps.iter().filter_map(|step| match step {
                BlueprintOperationPlanStep::InvokeCapability { capability } => {
                    Some(capability.code.clone())
                }
                _ => None,
            }).collect::<Vec<_>>()
        }),
        timeout_ms: 3_000,
        generated_by_model: None,
    })
}

fn lower_operation_step(step: &BlueprintOperationPlanStep) -> OperationPlanStep {
    match step {
        BlueprintOperationPlanStep::ValidateInput => OperationPlanStep::ValidateInput,
        BlueprintOperationPlanStep::QueryRecords => OperationPlanStep::QueryRecords,
        BlueprintOperationPlanStep::LoadRecord => OperationPlanStep::LoadRecord,
        BlueprintOperationPlanStep::CreateRecord => OperationPlanStep::CreateRecord,
        BlueprintOperationPlanStep::UpdateRecord => OperationPlanStep::UpdateRecord,
        BlueprintOperationPlanStep::DeleteRecord => OperationPlanStep::DeleteRecord,
        BlueprintOperationPlanStep::InvokeCapability { capability } => {
            OperationPlanStep::InvokeCapability {
                capability: capability.code.clone(),
            }
        }
        BlueprintOperationPlanStep::ReturnResult => OperationPlanStep::ReturnResult,
    }
}

fn lower_page(
    blueprint: &Blueprint,
    view: &ViewDefinition,
    operation_keys: &BTreeMap<String, String>,
    components: &ComponentIndex,
) -> Result<PageInput> {
    let section = component_id(components, "section")?;
    let heading = component_id(components, "h1")?;
    let mut children = vec![ComponentNode {
        component: heading,
        id: Some(format!("{}_title", view.descriptor.code)),
        properties: BTreeMap::new(),
        content: Some(PropertyValue::text(&view.descriptor.native_name)),
        children: Vec::new(),
    }];
    match view.layout {
        ViewLayout::Table => children.push(table_node(view, components)?),
        ViewLayout::Detail | ViewLayout::Form => {
            children.extend(form_nodes(blueprint, view, components)?);
        }
    }
    children.extend(action_nodes(view, operation_keys, components)?);
    let data_sources = page_data_sources(blueprint, view, operation_keys);
    let actions = view
        .actions
        .iter()
        .filter_map(|action| {
            operation_keys
                .get(&action.operation.native_name)
                .map(|operation| ActionDefinition {
                    id: action.descriptor.code.clone(),
                    operation: operation.clone(),
                    input: BTreeMap::new(),
                    confirmation: Some(format!("确认执行{}？", action.descriptor.native_name)),
                })
        })
        .collect();
    Ok(PageInput {
        route: view.route.clone(),
        state: PageState::Published,
        definition: PageDefinition {
            schema_version: PAGE_SCHEMA_VERSION,
            key: view.descriptor.code.clone(),
            title: view.descriptor.native_name.clone(),
            root: ComponentNode {
                component: section,
                id: Some(format!("{}_root", view.descriptor.code)),
                properties: BTreeMap::new(),
                content: None,
                children,
            },
            data_sources,
            actions,
        },
    })
}

fn table_node(view: &ViewDefinition, components: &ComponentIndex) -> Result<ComponentNode> {
    let columns = view
        .fields
        .iter()
        .map(|field| format!("{}:{}", field.code, field.native_name))
        .collect::<Vec<_>>()
        .join(",");
    Ok(ComponentNode {
        component: component_id(components, "table")?,
        id: Some(format!("{}_table", view.descriptor.code)),
        properties: BTreeMap::from([
            ("source".to_string(), PropertyValue::text("records")),
            ("columns".to_string(), PropertyValue::text(columns)),
        ]),
        content: None,
        children: Vec::new(),
    })
}

fn form_nodes(
    blueprint: &Blueprint,
    view: &ViewDefinition,
    components: &ComponentIndex,
) -> Result<Vec<ComponentNode>> {
    let input_component = component_id(components, "input")?;
    let fields = blueprint
        .structs
        .iter()
        .flat_map(|model| &model.fields)
        .filter(|field| view.fields.iter().any(|item| item == &field.descriptor))
        .map(|field| ComponentNode {
            component: input_component.clone(),
            id: Some(format!(
                "{}_{}",
                view.descriptor.code, field.descriptor.code
            )),
            properties: BTreeMap::from([
                (
                    "label".to_string(),
                    PropertyValue::text(&field.descriptor.native_name),
                ),
                (
                    "name".to_string(),
                    PropertyValue::text(&field.descriptor.code),
                ),
                (
                    "type".to_string(),
                    PropertyValue::text(input_type(&field.field_type)),
                ),
                (
                    "required".to_string(),
                    PropertyValue::Literal {
                        value: Value::Bool(field.required),
                    },
                ),
            ]),
            content: None,
            children: Vec::new(),
        })
        .collect();
    Ok(fields)
}

fn action_nodes(
    view: &ViewDefinition,
    operation_keys: &BTreeMap<String, String>,
    components: &ComponentIndex,
) -> Result<Vec<ComponentNode>> {
    let button = component_id(components, "button")?;
    Ok(view
        .actions
        .iter()
        .filter(|action| operation_keys.contains_key(&action.operation.native_name))
        .map(|action| ComponentNode {
            component: button.clone(),
            id: Some(format!(
                "{}_{}",
                view.descriptor.code, action.descriptor.code
            )),
            properties: BTreeMap::from([
                (
                    "tx".to_string(),
                    PropertyValue::text(&action.descriptor.native_name),
                ),
                (
                    "act".to_string(),
                    PropertyValue::text(&action.descriptor.code),
                ),
            ]),
            content: None,
            children: Vec::new(),
        })
        .collect())
}

fn page_data_sources(
    blueprint: &Blueprint,
    view: &ViewDefinition,
    operation_keys: &BTreeMap<String, String>,
) -> Vec<DataSourceDefinition> {
    if view.layout != ViewLayout::Table {
        return Vec::new();
    }
    blueprint
        .application
        .operations
        .iter()
        .find(|operation| {
            operation.model == view.model
                && operation.intent == nature_compiler::OperationIntent::List
        })
        .and_then(|operation| operation_keys.get(&operation.descriptor.native_name))
        .map(|operation| DataSourceDefinition {
            id: "records".to_string(),
            operation: operation.clone(),
            parameters: BTreeMap::new(),
        })
        .into_iter()
        .collect()
}

fn lower_menu(blueprint: &Blueprint) -> AdminMenuTree {
    let application = &blueprint.application;
    let default_route = application
        .views
        .iter()
        .find(|view| view.descriptor == application.navigation.default_view)
        .map(|view| view.route.clone())
        .unwrap_or_default();
    let children = application
        .navigation
        .entries
        .iter()
        .filter_map(|entry| {
            application
                .views
                .iter()
                .find(|view| view.descriptor == entry.view)
                .map(|view| AdminMenuNode {
                    id: format!(
                        "{}.{}",
                        application.domain.descriptor.code, view.descriptor.code
                    ),
                    kind: AdminMenuNodeKind::Page,
                    label: entry.descriptor.native_name.clone(),
                    href: view.route.clone(),
                    icon: "□".to_string(),
                    order: entry.order,
                    active_patterns: vec![view.route.clone()],
                    permissions_any_of: entry
                        .permissions
                        .iter()
                        .map(|permission| permission.code.clone())
                        .collect(),
                    children: Vec::new(),
                })
        })
        .collect();
    AdminMenuTree {
        sections: vec![AdminMenuSection {
            domain_id: application.domain.descriptor.code.clone(),
            label: application.navigation.section_label.clone(),
            default_href: default_route.clone(),
            order: 500,
            menus: vec![AdminMenuNode {
                id: format!("{}.root", application.domain.descriptor.code),
                kind: AdminMenuNodeKind::Branch,
                label: application.navigation.descriptor.native_name.clone(),
                href: default_route,
                icon: "◇".to_string(),
                order: 10,
                active_patterns: application
                    .views
                    .iter()
                    .map(|view| view.route.clone())
                    .collect(),
                permissions_any_of: Vec::new(),
                children,
            }],
        }],
    }
}

fn component_id(components: &ComponentIndex, dsl_name: &str) -> Result<String> {
    components
        .resolve(dsl_name)
        .map(|component| component.canonical_id().to_string())
}

fn operation_key(blueprint: &Blueprint, operation: &DomainOperation) -> String {
    format!(
        "{}.{}",
        blueprint.application.domain.descriptor.code, operation.descriptor.code
    )
}

fn field_type(value: &FieldType) -> &'static str {
    match value {
        FieldType::String | FieldType::Password | FieldType::Email | FieldType::Dictionary => {
            "string"
        }
        FieldType::Integer => "integer",
        FieldType::Decimal => "decimal",
        FieldType::Boolean => "boolean",
        FieldType::Timestamp => "timestamp",
        FieldType::Json => "json",
    }
}

fn input_type(value: &FieldType) -> &'static str {
    match value {
        FieldType::Password => "password",
        FieldType::Email => "email",
        FieldType::Integer | FieldType::Decimal => "number",
        _ => "text",
    }
}

fn http_method(value: HttpMethod) -> &'static str {
    match value {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Delete => "DELETE",
    }
}

fn model_schema(fields: &[FieldDefinition]) -> Value {
    let properties = fields
        .iter()
        .map(|field| {
            let format = match &field.field_type {
                FieldType::Email => Some("email"),
                FieldType::Password => Some("password"),
                _ => None,
            };
            let mut schema = json!({"type": json_schema_type(&field.field_type)});
            if let Some(format) = format {
                schema["format"] = Value::String(format.to_string());
            }
            (field.descriptor.code.clone(), schema)
        })
        .collect::<serde_json::Map<_, _>>();
    let required = fields
        .iter()
        .filter(|field| field.validations.contains(&ValidationRule::Required))
        .map(|field| Value::String(field.descriptor.code.clone()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

fn json_schema_type(value: &FieldType) -> &'static str {
    match value {
        FieldType::Integer | FieldType::Timestamp => "integer",
        FieldType::Decimal => "number",
        FieldType::Boolean => "boolean",
        FieldType::Json => "object",
        FieldType::String | FieldType::Password | FieldType::Email | FieldType::Dictionary => {
            "string"
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nature_compiler::{CompileRequest, Compiler, CompilerCatalog, MotherTongueInferenceEngine};
    use rudi::Context as RudiContext;

    use super::*;

    #[tokio::test]
    async fn lowers_mother_tongue_blueprint_to_full_stack_deployment() -> Result<()> {
        let compiler = Compiler::new(
            Arc::new(MotherTongueInferenceEngine),
            CompilerCatalog::default(),
        );
        let artifacts = compiler
            .compile(CompileRequest {
                source_text: r#"领域：用户管理

需求：
1. 用户可以注册、登录和维护自己的资料
2. 管理员可以查询、修改和停用用户
3. 用户名和邮箱不能重复，密码不能在列表中展示

建模：用户
1. 用户名：文本，必填，唯一
2. 密码：密码，必填
3. 邮箱：文本，邮箱格式，唯一
4. 权限等级：字典，显示母语标签

操作：
1. 注册用户时校验用户名和邮箱，然后保存用户
2. 登录时校验密码并返回登录结果
3. 管理员可以按用户名和权限等级筛选用户
4. 停用用户前必须确认，停用后刷新用户列表

界面：用户列表
1. 使用表格展示用户名、邮箱和权限等级
2. 顶部提供新增用户操作
3. 支持按用户名和权限等级筛选

界面：用户资料
1. 使用表单管理用户信息
2. 密码只允许通过单独操作修改

导航：
1. 在“组织管理”下面显示“用户管理”
2. 用户列表作为默认页面

权限：
1. 用户只能管理自己的资料
2. 管理员可以管理全部用户
"#
                .to_string(),
                previous_blueprint: None,
            })
            .await?;
        let mut context = RudiContext::auto_register();
        let components = ComponentIndex::from_context(&mut context)?;
        let blueprint = artifacts.blueprint.context("编译结果缺少 Blueprint")?;
        let deployment = lower_application(&blueprint, &components)?;

        assert_eq!(deployment.models.len(), 1);
        assert!(!deployment.operations.is_empty());
        assert!(
            deployment
                .routes
                .iter()
                .all(|route| route.path.starts_with("/api/app/"))
        );
        assert!(deployment.pages.iter().any(|page| {
            !page.definition.data_sources.is_empty() && page.definition.title == "用户列表"
        }));
        assert!(!deployment.menu.sections[0].menus.is_empty());
        assert!(
            deployment.operations.iter().all(|operation| matches!(
                operation.executor,
                OperationExecutorDefinition::Plan(_)
            ))
        );
        Ok(())
    }
}
