//! 应用部署和动态领域路由的 PostgreSQL 边界。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toasty::stmt::{List, Query};

use crate::{EngineStore, FieldInput, MetaField, MetaModel, ModelInput, timestamp_ms};
use crate::{
    operation::{
        OperationDefinition, OperationDraft, OperationRevision, operation_revision_from_input,
        OperationInvocation, OperationRequestContext, validate_operation_draft,
    },
    page::{PageInput, PageRecord, validate_page_input},
};

/// 一次应用部署中的模型和字段。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentModelInput {
    pub model: ModelInput,
    pub fields: Vec<FieldInput>,
}

/// 动态领域路由输入。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteDefinitionInput {
    pub method: String,
    pub path: String,
    pub operation_key: String,
}

/// Engine 可原子物化的应用部署输入。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineApplicationDeployment {
    pub project_id: String,
    pub revision_id: String,
    pub artifact_hash: String,
    pub domain_code: String,
    pub manifest: Value,
    pub models: Vec<DeploymentModelInput>,
    pub operations: Vec<OperationDraft>,
    pub pages: Vec<PageInput>,
    pub routes: Vec<RouteDefinitionInput>,
}

/// 已发布应用 Revision 的活动记录。
#[derive(Clone, Debug, PartialEq, toasty::Model)]
#[table = "nature_application_deployments"]
pub struct ApplicationDeploymentRecord {
    #[key]
    pub id: String,
    #[index]
    pub project_id: String,
    #[unique]
    pub revision_id: String,
    pub artifact_hash: String,
    pub domain_code: String,
    pub state: String,
    pub manifest: toasty::Json<Value>,
    pub created_at_ms: i64,
    pub activated_at_ms: i64,
}

/// method/path 到 Operation Key 的动态映射。
#[derive(Clone, Debug, Eq, PartialEq, toasty::Model)]
#[table = "engine_route_definitions"]
pub struct RouteDefinitionRecord {
    #[key]
    pub id: String,
    #[index]
    pub deployment_id: String,
    pub method: String,
    pub path_template: String,
    pub operation_key: String,
    pub created_at_ms: i64,
}

impl EngineStore {
    /// 在一个 PostgreSQL 事务中物化并激活完整应用部署。
    pub async fn deploy_application(
        &self,
        input: EngineApplicationDeployment,
    ) -> anyhow::Result<ApplicationDeploymentRecord> {
        validate_deployment(&input)?;
        let now = timestamp_ms();
        let deployment_id = uuid::Uuid::new_v4().to_string();
        let mut db = self.db.lock().await;
        let mut transaction = db
            .transaction()
            .await
            .context("启动应用部署事务失败")?;
        let previous_deployments = Query::<List<ApplicationDeploymentRecord>>::filter(
            ApplicationDeploymentRecord::fields()
                .project_id()
                .eq(&input.project_id)
                .and(ApplicationDeploymentRecord::fields().state().eq("active")),
        )
        .exec(&mut transaction)
        .await
        .context("读取当前活动应用部署失败")?;
        let ownership = DeploymentOwnership::from_deployments(&previous_deployments);

        materialize_models(&mut transaction, &input.models, &ownership, now).await?;
        materialize_operations(&mut transaction, &input.operations, &ownership, now).await?;
        materialize_pages(&mut transaction, &input.pages, &ownership, now).await?;
        validate_route_conflicts(&mut transaction, &previous_deployments, &input.routes).await?;

        for deployment in previous_deployments {
            ApplicationDeploymentRecord::filter(
                ApplicationDeploymentRecord::fields().id().eq(&deployment.id),
            )
            .update()
            .state("inactive")
            .exec(&mut transaction)
            .await
            .context("停用上一版应用部署失败")?;
        }
        let deployment = ApplicationDeploymentRecord::create()
            .id(&deployment_id)
            .project_id(&input.project_id)
            .revision_id(&input.revision_id)
            .artifact_hash(&input.artifact_hash)
            .domain_code(&input.domain_code)
            .state("active")
            .manifest(input.manifest)
            .created_at_ms(now)
            .activated_at_ms(now)
            .exec(&mut transaction)
            .await
            .context("创建应用部署记录失败")?;
        for route in input.routes {
            RouteDefinitionRecord::create()
                .id(uuid::Uuid::new_v4().to_string())
                .deployment_id(&deployment_id)
                .method(&route.method)
                .path_template(&route.path)
                .operation_key(&route.operation_key)
                .created_at_ms(now)
                .exec(&mut transaction)
                .await
                .context("创建动态领域路由失败")?;
        }
        transaction.commit().await.context("提交应用部署事务失败")?;
        Ok(deployment)
    }

    /// 返回全部活动部署，供菜单和路由运行时构建快照。
    pub async fn active_application_deployments(
        &self,
    ) -> anyhow::Result<Vec<ApplicationDeploymentRecord>> {
        let mut db = self.db.lock().await;
        Query::<List<ApplicationDeploymentRecord>>::filter(
            ApplicationDeploymentRecord::fields().state().eq("active"),
        )
        .exec(&mut *db)
        .await
        .map_err(Into::into)
    }

    /// 返回活动部署拥有的全部路由。
    pub async fn active_route_definitions(&self) -> anyhow::Result<Vec<RouteDefinitionRecord>> {
        let deployments = self.active_application_deployments().await?;
        let deployment_ids = deployments
            .into_iter()
            .map(|deployment| deployment.id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut db = self.db.lock().await;
        let routes = Query::<List<RouteDefinitionRecord>>::all()
            .exec(&mut *db)
            .await?;
        Ok(routes
            .into_iter()
            .filter(|route| deployment_ids.contains(&route.deployment_id))
            .collect())
    }

    /// 解析并调用活动应用的领域路由。
    pub async fn invoke_application_route(
        &self,
        method: &str,
        path: &str,
        query: BTreeMap<String, Vec<String>>,
        body: Value,
    ) -> anyhow::Result<OperationInvocation> {
        let routes = self.active_route_definitions().await?;
        let mut router = matchit::Router::new();
        for route in routes.iter().filter(|route| route.method == method) {
            router
                .insert(&route.path_template, route)
                .with_context(|| format!("构建动态领域路由失败: {}", route.path_template))?;
        }
        let matched = router
            .at(path)
            .with_context(|| format!("没有匹配的活动领域路由: {method} {path}"))?;
        let path = matched
            .params
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        self.invoke_operation(OperationRequestContext {
            operation_key: matched.value.operation_key.clone(),
            method: method.to_string(),
            path,
            query,
            body,
        })
        .await
    }
}

#[derive(Default)]
struct DeploymentOwnership {
    models: BTreeSet<String>,
    operations: BTreeSet<String>,
    pages: BTreeSet<String>,
}

impl DeploymentOwnership {
    fn from_deployments(deployments: &[ApplicationDeploymentRecord]) -> Self {
        let mut ownership = Self::default();
        for deployment in deployments {
            let manifest = &deployment.manifest.0;
            collect_manifest_codes(manifest, "models", "model", "name", &mut ownership.models);
            collect_manifest_codes(
                manifest,
                "operations",
                "",
                "operationKey",
                &mut ownership.operations,
            );
            collect_manifest_codes(
                manifest,
                "pages",
                "definition",
                "key",
                &mut ownership.pages,
            );
        }
        ownership
    }
}

fn collect_manifest_codes(
    manifest: &Value,
    collection: &str,
    nested: &str,
    key: &str,
    target: &mut BTreeSet<String>,
) {
    let Some(items) = manifest.get(collection).and_then(Value::as_array) else {
        return;
    };
    for item in items {
        let value = if nested.is_empty() {
            item.get(key)
        } else {
            item.get(nested).and_then(|value| value.get(key))
        };
        if let Some(value) = value.and_then(Value::as_str) {
            target.insert(value.to_string());
        }
    }
}

async fn materialize_models(
    transaction: &mut toasty::Transaction<'_>,
    models: &[DeploymentModelInput],
    ownership: &DeploymentOwnership,
    now: i64,
) -> anyhow::Result<()> {
    for deployment_model in models {
        let model_name = &deployment_model.model.name;
        let existing = Query::<List<MetaModel>>::filter(MetaModel::fields().name().eq(model_name))
            .first()
            .exec(&mut *transaction)
            .await
            .context("查询部署模型失败")?;
        match existing {
            Some(model) if ownership.models.contains(model_name) => {
                MetaModel::filter(MetaModel::fields().id().eq(&model.id))
                    .update()
                    .display_name(&deployment_model.model.display_name)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("更新部署模型失败")?;
            }
            Some(_) => bail!("部署模型与非 Nature 对象冲突: {model_name}"),
            None => {
                MetaModel::create()
                    .id(uuid::Uuid::new_v4().to_string())
                    .name(model_name)
                    .display_name(&deployment_model.model.display_name)
                    .created_at_ms(now)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("创建部署模型失败")?;
            }
        }
        materialize_fields(transaction, deployment_model, now).await?;
    }
    Ok(())
}

async fn materialize_fields(
    transaction: &mut toasty::Transaction<'_>,
    deployment_model: &DeploymentModelInput,
    now: i64,
) -> anyhow::Result<()> {
    let existing = Query::<List<MetaField>>::filter(
        MetaField::fields()
            .model_name()
            .eq(&deployment_model.model.name),
    )
    .exec(&mut *transaction)
    .await
    .context("查询部署字段失败")?;
    let desired = deployment_model
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    for field in existing.iter().filter(|field| !desired.contains(field.name.as_str())) {
        MetaField::filter(MetaField::fields().id().eq(&field.id))
            .delete()
            .exec(&mut *transaction)
            .await
            .context("删除已移除的部署字段失败")?;
    }
    for input in &deployment_model.fields {
        match existing.iter().find(|field| field.name == input.name) {
            Some(field) => {
                MetaField::filter(MetaField::fields().id().eq(&field.id))
                    .update()
                    .display_name(&input.display_name)
                    .field_type(&input.field_type)
                    .is_required(input.is_required)
                    .expression(&input.expression)
                    .dependency_json(&input.dependency_json)
                    .domain_metadata_json(&input.domain_metadata_json)
                    .validation_json(&input.validation_json)
                    .order_index(input.order_index)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("更新部署字段失败")?;
            }
            None => {
                MetaField::create()
                    .id(uuid::Uuid::new_v4().to_string())
                    .model_name(&deployment_model.model.name)
                    .name(&input.name)
                    .display_name(&input.display_name)
                    .field_type(&input.field_type)
                    .is_required(input.is_required)
                    .expression(&input.expression)
                    .dependency_json(&input.dependency_json)
                    .domain_metadata_json(&input.domain_metadata_json)
                    .validation_json(&input.validation_json)
                    .order_index(input.order_index)
                    .created_at_ms(now)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("创建部署字段失败")?;
            }
        }
    }
    Ok(())
}

async fn materialize_operations(
    transaction: &mut toasty::Transaction<'_>,
    operations: &[OperationDraft],
    ownership: &DeploymentOwnership,
    now: i64,
) -> anyhow::Result<()> {
    for draft in operations {
        validate_operation_draft(draft)?;
        let existing = Query::<List<OperationDefinition>>::filter(
            OperationDefinition::fields()
                .operation_key()
                .eq(&draft.operation_key),
        )
        .first()
        .exec(&mut *transaction)
        .await
        .context("查询部署 Operation 失败")?;
        let definition_id = match existing {
            Some(definition) if ownership.operations.contains(&draft.operation_key) => {
                definition.id
            }
            Some(_) => bail!("部署 Operation 与非 Nature 对象冲突: {}", draft.operation_key),
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                OperationDefinition::create()
                    .id(&id)
                    .operation_key(&draft.operation_key)
                    .display_name(&draft.display_name)
                    .description(&draft.description)
                    .method(&draft.method)
                    .state("draft")
                    .active_revision_id(&None::<String>)
                    .created_at_ms(now)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("创建部署 Operation 失败")?;
                id
            }
        };
        let revisions = Query::<List<OperationRevision>>::filter(
            OperationRevision::fields().operation_id().eq(&definition_id),
        )
        .exec(&mut *transaction)
        .await
        .context("查询部署 Operation revision 失败")?;
        let next_revision = revisions
            .iter()
            .map(|revision| revision.revision)
            .max()
            .unwrap_or(0)
            + 1;
        let revision = operation_revision_from_input(
            &definition_id,
            next_revision,
            crate::operation::OperationRevisionInput {
                executor: draft.executor.clone(),
                input_schema: draft.input_schema.clone(),
                output_schema: draft.output_schema.clone(),
                capability_policy: draft.capability_policy.clone(),
                timeout_ms: draft.timeout_ms,
                generated_by_model: draft.generated_by_model.clone(),
            },
        )?;
        OperationRevision::create()
            .id(&revision.id)
            .operation_id(&revision.operation_id)
            .revision(revision.revision)
            .executor_kind(&revision.executor_kind)
            .source_text(&revision.source_text)
            .input_schema(revision.input_schema.0)
            .output_schema(revision.output_schema.0)
            .capability_policy(revision.capability_policy.0)
            .timeout_ms(revision.timeout_ms)
            .generated_by_model(&revision.generated_by_model)
            .created_at_ms(revision.created_at_ms)
            .exec(&mut *transaction)
            .await
            .context("创建部署 Operation revision 失败")?;
        OperationDefinition::filter(OperationDefinition::fields().id().eq(&definition_id))
            .update()
            .display_name(&draft.display_name)
            .description(&draft.description)
            .method(&draft.method)
            .state("published")
            .active_revision_id(Some(revision.id))
            .updated_at_ms(now)
            .exec(&mut *transaction)
            .await
            .context("激活部署 Operation revision 失败")?;
    }
    Ok(())
}

async fn materialize_pages(
    transaction: &mut toasty::Transaction<'_>,
    pages: &[PageInput],
    ownership: &DeploymentOwnership,
    now: i64,
) -> anyhow::Result<()> {
    for input in pages {
        validate_page_input(input)?;
        let page_key = &input.definition.key;
        let definition = serde_json::to_value(&input.definition).context("序列化部署页面失败")?;
        let existing = Query::<List<PageRecord>>::filter(PageRecord::fields().page_key().eq(page_key))
            .first()
            .exec(&mut *transaction)
            .await
            .context("查询部署页面失败")?;
        match existing {
            Some(page) if ownership.pages.contains(page_key) => {
                PageRecord::filter(PageRecord::fields().id().eq(&page.id))
                    .update()
                    .route(&input.route)
                    .state(input.state.as_str())
                    .definition(definition)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("更新部署页面失败")?;
            }
            Some(_) => bail!("部署页面与非 Nature 对象冲突: {page_key}"),
            None => {
                PageRecord::create()
                    .id(uuid::Uuid::new_v4().to_string())
                    .page_key(page_key)
                    .route(&input.route)
                    .state(input.state.as_str())
                    .definition(definition)
                    .created_at_ms(now)
                    .updated_at_ms(now)
                    .exec(&mut *transaction)
                    .await
                    .context("创建部署页面失败")?;
            }
        }
    }
    Ok(())
}

async fn validate_route_conflicts(
    transaction: &mut toasty::Transaction<'_>,
    previous: &[ApplicationDeploymentRecord],
    routes: &[RouteDefinitionInput],
) -> anyhow::Result<()> {
    let previous_ids = previous
        .iter()
        .map(|deployment| deployment.id.as_str())
        .collect::<BTreeSet<_>>();
    let active_deployments = Query::<List<ApplicationDeploymentRecord>>::filter(
        ApplicationDeploymentRecord::fields().state().eq("active"),
    )
    .exec(&mut *transaction)
    .await
    .context("查询活动路由部署失败")?;
    let active_ids = active_deployments
        .iter()
        .filter(|deployment| !previous_ids.contains(deployment.id.as_str()))
        .map(|deployment| deployment.id.as_str())
        .collect::<BTreeSet<_>>();
    let existing_routes = Query::<List<RouteDefinitionRecord>>::all()
        .exec(&mut *transaction)
        .await
        .context("查询活动领域路由失败")?;
    for route in routes {
        if existing_routes.iter().any(|existing| {
            active_ids.contains(existing.deployment_id.as_str())
                && existing.method == route.method
                && existing.path_template == route.path
        }) {
            bail!("领域路由与其他活动应用冲突: {} {}", route.method, route.path);
        }
    }
    Ok(())
}

fn validate_deployment(input: &EngineApplicationDeployment) -> anyhow::Result<()> {
    if input.project_id.trim().is_empty() || input.revision_id.trim().is_empty() {
        bail!("应用部署缺少 project_id 或 revision_id");
    }
    let mut routes = BTreeSet::new();
    for route in &input.routes {
        if !route.path.starts_with("/api/app/") {
            bail!("领域路由必须位于 /api/app/ 下: {}", route.path);
        }
        if !routes.insert((route.method.as_str(), route.path.as_str())) {
            bail!("应用部署包含重复路由: {} {}", route.method, route.path);
        }
        if !input
            .operations
            .iter()
            .any(|operation| operation.operation_key == route.operation_key)
        {
            bail!("领域路由引用了不存在的 Operation: {}", route.operation_key);
        }
    }
    for page in &input.pages {
        validate_page_input(page)?;
    }
    for operation in &input.operations {
        validate_operation_draft(operation)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::operation::{OperationExecutorDefinition, OperationPlan};

    fn deployment(route: RouteDefinitionInput) -> EngineApplicationDeployment {
        EngineApplicationDeployment {
            project_id: "project".to_string(),
            revision_id: "revision".to_string(),
            artifact_hash: "hash".to_string(),
            domain_code: "user_management".to_string(),
            manifest: json!({}),
            models: Vec::new(),
            operations: vec![OperationDraft {
                operation_key: "user_management.list_users".to_string(),
                display_name: "查询用户".to_string(),
                description: String::new(),
                method: "GET".to_string(),
                executor: OperationExecutorDefinition::Plan(OperationPlan {
                    model_name: "user".to_string(),
                    steps: vec![
                        crate::operation::OperationPlanStep::QueryRecords,
                        crate::operation::OperationPlanStep::ReturnResult,
                    ],
                }),
                input_schema: json!({"type": "object"}),
                output_schema: json!({"type": "object"}),
                capability_policy: json!({"allowed": []}),
                timeout_ms: 1_000,
                generated_by_model: Some("nature-compiler".to_string()),
            }],
            pages: Vec::new(),
            routes: vec![route],
        }
    }

    #[test]
    fn deployment_rejects_routes_outside_application_namespace() {
        let input = deployment(RouteDefinitionInput {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            operation_key: "user_management.list_users".to_string(),
        });
        assert!(validate_deployment(&input).is_err());
    }

    #[test]
    fn deployment_rejects_dangling_operation_route() {
        let input = deployment(RouteDefinitionInput {
            method: "GET".to_string(),
            path: "/api/app/user-management/users".to_string(),
            operation_key: "user_management.missing".to_string(),
        });
        assert!(validate_deployment(&input).is_err());
    }
}
