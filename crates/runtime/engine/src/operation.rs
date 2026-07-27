//! 在线 operation 的契约、持久化与受控执行。

use std::{collections::BTreeMap, sync::Arc, time::Instant};

use anyhow::{Context, anyhow, bail};
use async_trait::async_trait;
use rhai::{Dynamic, Engine as RhaiEngine, Scope};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use toasty::stmt::{List, Query};

use crate::runtime::{EngineStore, PageData, PageParams, timestamp_ms};

/// operation 集合 API 路径。
pub const OPERATIONS_PATH: &str = "/api/engine/operations";

/// operation 统一调用路径模板。
pub const OPERATION_INVOKE_PATH_TEMPLATE: &str = "/api/engine/invoke/{operation_key}";

/// operation revision 集合 API 路径模板。
pub const OPERATION_REVISIONS_PATH_TEMPLATE: &str =
    "/api/engine/operations/{operation_key}/revisions";

/// operation revision 发布 API 路径模板。
pub const OPERATION_PUBLISH_PATH_TEMPLATE: &str =
    "/api/engine/operations/{operation_key}/revisions/{revision_id}/publish";

/// operation 列表操作 ID。
pub const OP_OPERATIONS_LIST: &str = "engine.operations.list";

/// operation 创建操作 ID。
pub const OP_OPERATIONS_CREATE: &str = "engine.operations.create";

/// operation 发布操作 ID。
pub const OP_OPERATIONS_PUBLISH: &str = "engine.operations.publish";

/// operation 调用操作 ID。
pub const OP_OPERATIONS_INVOKE: &str = "engine.operations.invoke";

const OPERATION_STATE_DRAFT: &str = "draft";
const OPERATION_STATE_PUBLISHED: &str = "published";
const OPERATION_STATE_DISABLED: &str = "disabled";
const EXECUTOR_RHAI: &str = "rhai";
const EXECUTOR_PLAN: &str = "plan";
const MAX_SOURCE_BYTES: usize = 64 * 1024;
const DEFAULT_TIMEOUT_MS: i64 = 3_000;
const MAX_TIMEOUT_MS: i64 = 30_000;

/// 在线 operation 的稳定身份。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "engine_operation_definitions"]
pub struct OperationDefinition {
    #[key]
    pub id: String,
    #[index]
    pub operation_key: String,
    pub display_name: String,
    pub description: String,
    pub method: String,
    pub state: String,
    pub active_revision_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// operation 的不可变执行版本。
#[derive(Clone, Debug, PartialEq, toasty::Model)]
#[table = "engine_operation_revisions"]
pub struct OperationRevision {
    #[key]
    pub id: String,
    #[index]
    pub operation_id: String,
    pub revision: i32,
    pub executor_kind: String,
    pub source_text: String,
    pub input_schema: toasty::Json<Value>,
    pub output_schema: toasty::Json<Value>,
    pub capability_policy: toasty::Json<Value>,
    pub timeout_ms: i64,
    pub generated_by_model: Option<String>,
    pub created_at_ms: i64,
}

/// operation 的一次执行审计记录。
#[derive(Clone, Debug, PartialEq, toasty::Model)]
#[table = "engine_operation_runs"]
pub struct OperationRun {
    #[key]
    pub id: String,
    #[index]
    pub operation_id: String,
    pub revision_id: String,
    pub request_context: toasty::Json<Value>,
    pub response: toasty::Json<Value>,
    pub status: String,
    pub diagnostics: Option<String>,
    pub duration_ms: i64,
    pub created_at_ms: i64,
}

/// 新建 operation 及首个 revision 的输入。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationDraft {
    pub operation_key: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub executor: OperationExecutorDefinition,
    #[serde(default = "default_object_schema")]
    pub input_schema: Value,
    #[serde(default = "default_object_schema")]
    pub output_schema: Value,
    #[serde(default = "default_capability_policy")]
    pub capability_policy: Value,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
    #[serde(default)]
    pub generated_by_model: Option<String>,
}

/// 为已有 operation 新建 revision 的输入。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationRevisionInput {
    pub executor: OperationExecutorDefinition,
    #[serde(default = "default_object_schema")]
    pub input_schema: Value,
    #[serde(default = "default_object_schema")]
    pub output_schema: Value,
    #[serde(default = "default_capability_policy")]
    pub capability_policy: Value,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
    #[serde(default)]
    pub generated_by_model: Option<String>,
}

/// operation 与其新建 revision 的组合结果。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationBundle {
    pub definition: OperationDefinition,
    pub revision: OperationRevisionView,
}

/// 对外返回的 revision 视图。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationRevisionView {
    pub id: String,
    pub operation_id: String,
    pub revision: i32,
    pub executor: OperationExecutorDefinition,
    pub input_schema: Value,
    pub output_schema: Value,
    pub capability_policy: Value,
    pub timeout_ms: i64,
    pub generated_by_model: Option<String>,
    pub created_at_ms: i64,
}

impl TryFrom<OperationRevision> for OperationRevisionView {
    type Error = anyhow::Error;

    fn try_from(revision: OperationRevision) -> anyhow::Result<Self> {
        Ok(Self {
            id: revision.id,
            operation_id: revision.operation_id,
            revision: revision.revision,
            executor: deserialize_executor(&revision.executor_kind, &revision.source_text)?,
            input_schema: revision.input_schema.0,
            output_schema: revision.output_schema.0,
            capability_policy: revision.capability_policy.0,
            timeout_ms: revision.timeout_ms,
            generated_by_model: revision.generated_by_model,
            created_at_ms: revision.created_at_ms,
        })
    }
}

/// 可持久化的受控 Operation 执行定义。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationExecutorDefinition {
    Plan(OperationPlan),
    Rhai { source_text: String },
}

/// 不包含脚本源码的异步领域操作计划。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationPlan {
    pub model_name: String,
    pub steps: Vec<OperationPlanStep>,
}

/// Engine 可审计执行的领域步骤。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPlanStep {
    ValidateInput,
    QueryRecords,
    LoadRecord,
    CreateRecord,
    UpdateRecord,
    DeleteRecord,
    InvokeCapability { capability: String },
    ReturnResult,
}

/// 可由 Rudi 注册的领域运行能力描述。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationCapabilityDefinition {
    pub code: String,
    pub native_name: String,
    pub aliases: Vec<String>,
    pub input_schema: Value,
    pub output_schema: Value,
}

/// 领域插件实现的受控 Operation 能力。
#[async_trait]
pub trait OperationCapabilityProvider: Send + Sync {
    fn definition(&self) -> OperationCapabilityDefinition;

    async fn execute(
        &self,
        input: Value,
        context: &OperationRequestContext,
    ) -> anyhow::Result<Value>;
}

pub type DynOperationCapabilityProvider = Arc<dyn OperationCapabilityProvider>;

/// 运行期 Operation 能力目录。
#[derive(Clone, Default)]
pub struct OperationCapabilityCatalog {
    providers: Vec<DynOperationCapabilityProvider>,
}

impl OperationCapabilityCatalog {
    pub fn new(providers: Vec<DynOperationCapabilityProvider>) -> Self {
        Self { providers }
    }

    pub fn definitions(&self) -> Vec<OperationCapabilityDefinition> {
        self.providers
            .iter()
            .map(|provider| provider.definition())
            .collect()
    }

    async fn execute(
        &self,
        code: &str,
        input: Value,
        context: &OperationRequestContext,
    ) -> anyhow::Result<Value> {
        let matched = self
            .providers
            .iter()
            .filter(|provider| provider.definition().code == code)
            .collect::<Vec<_>>();
        match matched.as_slice() {
            [provider] => provider.execute(input, context).await,
            [] => bail!("Operation capability 未注册: {code}"),
            _ => bail!("Operation capability 注册重复: {code}"),
        }
    }
}

/// 注入动态脚本的统一请求上下文。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationRequestContext {
    pub operation_key: String,
    pub method: String,
    #[serde(default)]
    pub path: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_body")]
    pub body: Value,
}

/// 指定 revision 试运行时使用的请求数据。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OperationTestInput {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_body")]
    pub body: Value,
}

/// operation 执行结果及其版本信息。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationInvocation {
    pub operation_key: String,
    pub revision: i32,
    pub data: Value,
    pub duration_ms: i64,
}

impl EngineStore {
    /// 创建 operation，并写入不可变的首个 revision。
    pub async fn create_operation(&self, draft: OperationDraft) -> anyhow::Result<OperationBundle> {
        validate_operation_draft(&draft)?;
        if self.get_operation(&draft.operation_key).await?.is_some() {
            bail!("operation 已存在: {}", draft.operation_key);
        }

        let now = timestamp_ms();
        let definition = OperationDefinition {
            id: uuid::Uuid::new_v4().to_string(),
            operation_key: draft.operation_key,
            display_name: draft.display_name,
            description: draft.description,
            method: normalize_method(&draft.method)?,
            state: OPERATION_STATE_DRAFT.to_string(),
            active_revision_id: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        let revision = operation_revision_from_input(
            &definition.id,
            1,
            OperationRevisionInput {
                executor: draft.executor,
                input_schema: draft.input_schema,
                output_schema: draft.output_schema,
                capability_policy: draft.capability_policy,
                timeout_ms: draft.timeout_ms,
                generated_by_model: draft.generated_by_model,
            },
        )?;

        let mut db = self.db.lock().await;
        let mut transaction = db
            .transaction()
            .await
            .context("启动 engine operation 创建事务失败")?;
        let created_definition = OperationDefinition::create()
            .id(&definition.id)
            .operation_key(&definition.operation_key)
            .display_name(&definition.display_name)
            .description(&definition.description)
            .method(&definition.method)
            .state(&definition.state)
            .active_revision_id(&definition.active_revision_id)
            .created_at_ms(definition.created_at_ms)
            .updated_at_ms(definition.updated_at_ms)
            .exec(&mut transaction)
            .await
            .context("创建 engine operation 失败")?;
        let created_revision = OperationRevision::create()
            .id(&revision.id)
            .operation_id(&revision.operation_id)
            .revision(revision.revision)
            .executor_kind(&revision.executor_kind)
            .source_text(&revision.source_text)
            .input_schema(revision.input_schema.0.clone())
            .output_schema(revision.output_schema.0.clone())
            .capability_policy(revision.capability_policy.0.clone())
            .timeout_ms(revision.timeout_ms)
            .generated_by_model(&revision.generated_by_model)
            .created_at_ms(revision.created_at_ms)
            .exec(&mut transaction)
            .await
            .context("创建 engine operation 首个 revision 失败")?;
        transaction
            .commit()
            .await
            .context("提交 engine operation 创建事务失败")?;

        Ok(OperationBundle {
            definition: created_definition,
            revision: OperationRevisionView::try_from(created_revision)?,
        })
    }

    /// 分页查询 operation 定义。
    pub async fn list_operations(
        &self,
        page: PageParams,
    ) -> anyhow::Result<PageData<OperationDefinition>> {
        let mut db = self.db.lock().await;
        let total = Query::<List<OperationDefinition>>::all()
            .count()
            .exec(&mut *db)
            .await
            .context("统计 engine operation 失败")?;
        let mut query = Query::<List<OperationDefinition>>::all();
        query.limit(page.s);
        query.offset(page.o);
        let mut rows = query
            .exec(&mut *db)
            .await
            .context("查询 engine operation 失败")?;
        rows.sort_by(|left, right| left.operation_key.cmp(&right.operation_key));
        Ok(PageData { d: rows, t: total, p: page })
    }

    /// 按 operation key 查询稳定定义。
    pub async fn get_operation(
        &self,
        operation_key: &str,
    ) -> anyhow::Result<Option<OperationDefinition>> {
        let mut db = self.db.lock().await;
        Query::<List<OperationDefinition>>::filter(
            OperationDefinition::fields()
                .operation_key()
                .eq(operation_key),
        )
        .first()
        .exec(&mut *db)
        .await
        .context("查询 engine operation 失败")
    }

    /// 为已有 operation 创建新的不可变 revision。
    pub async fn create_operation_revision(
        &self,
        operation_key: &str,
        input: OperationRevisionInput,
    ) -> anyhow::Result<OperationRevisionView> {
        let definition = self
            .get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {operation_key}"))?;
        let revision = self
            .create_operation_revision_by_id(&definition.id, input)
            .await?;
        OperationRevisionView::try_from(revision)
    }

    /// 查询 operation 的全部 revision，按版本倒序返回。
    pub async fn list_operation_revisions(
        &self,
        operation_key: &str,
    ) -> anyhow::Result<Vec<OperationRevisionView>> {
        let definition = self
            .get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {operation_key}"))?;
        let mut db = self.db.lock().await;
        let mut revisions = Query::<List<OperationRevision>>::filter(
            OperationRevision::fields()
                .operation_id()
                .eq(&definition.id),
        )
        .exec(&mut *db)
        .await
        .context("查询 engine operation revision 失败")?;
        revisions.sort_by_key(|item| std::cmp::Reverse(item.revision));
        revisions
            .into_iter()
            .map(OperationRevisionView::try_from)
            .collect()
    }

    /// 发布指定 revision，使其成为统一网关的活动版本。
    pub async fn publish_operation(
        &self,
        operation_key: &str,
        revision_id: &str,
    ) -> anyhow::Result<OperationDefinition> {
        let definition = self
            .get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {operation_key}"))?;
        let revision = self
            .get_operation_revision(revision_id)
            .await?
            .ok_or_else(|| anyhow!("operation revision 不存在: {revision_id}"))?;
        if revision.operation_id != definition.id {
            bail!("revision 不属于 operation: {operation_key}/{revision_id}");
        }

        let now = timestamp_ms();
        let active_revision_id = Some(revision.id);
        {
            let mut db = self.db.lock().await;
            OperationDefinition::filter(OperationDefinition::fields().id().eq(&definition.id))
                .update()
                .state(OPERATION_STATE_PUBLISHED)
                .active_revision_id(&active_revision_id)
                .updated_at_ms(now)
                .exec(&mut *db)
                .await
                .context("发布 engine operation 失败")?;
        }
        self.get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("发布后的 operation 不存在: {operation_key}"))
    }

    /// 禁用 operation，保留其 revision 和执行审计。
    pub async fn disable_operation(
        &self,
        operation_key: &str,
    ) -> anyhow::Result<OperationDefinition> {
        let definition = self
            .get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {operation_key}"))?;
        let now = timestamp_ms();
        {
            let mut db = self.db.lock().await;
            OperationDefinition::filter(OperationDefinition::fields().id().eq(&definition.id))
                .update()
                .state(OPERATION_STATE_DISABLED)
                .updated_at_ms(now)
                .exec(&mut *db)
                .await
                .context("禁用 engine operation 失败")?;
        }
        self.get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("禁用后的 operation 不存在: {operation_key}"))
    }

    /// 调用已发布 operation，并写入执行审计。
    pub async fn invoke_operation(
        &self,
        context: OperationRequestContext,
    ) -> anyhow::Result<OperationInvocation> {
        let definition = self
            .get_operation(&context.operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {}", context.operation_key))?;
        if definition.state != OPERATION_STATE_PUBLISHED {
            bail!("operation 尚未发布: {}", context.operation_key);
        }
        if definition.method != context.method {
            bail!(
                "operation method 不匹配: 需要 {}, 收到 {}",
                definition.method,
                context.method
            );
        }
        let revision_id = definition
            .active_revision_id
            .as_deref()
            .ok_or_else(|| anyhow!("operation 没有活动 revision: {}", context.operation_key))?;
        let revision = self
            .get_operation_revision(revision_id)
            .await?
            .ok_or_else(|| anyhow!("活动 operation revision 不存在: {revision_id}"))?;

        self.execute_and_audit(&definition, revision, context).await
    }

    /// 试运行指定 revision，不要求 operation 已发布。
    pub async fn test_operation_revision(
        &self,
        operation_key: &str,
        revision_id: &str,
        input: OperationTestInput,
    ) -> anyhow::Result<OperationInvocation> {
        let definition = self
            .get_operation(operation_key)
            .await?
            .ok_or_else(|| anyhow!("operation 不存在: {operation_key}"))?;
        let revision = self
            .get_operation_revision(revision_id)
            .await?
            .ok_or_else(|| anyhow!("operation revision 不存在: {revision_id}"))?;
        if revision.operation_id != definition.id {
            bail!("revision 不属于 operation: {operation_key}/{revision_id}");
        }
        let method = match input.method {
            Some(value) => normalize_method(&value)?,
            None => definition.method.clone(),
        };
        let mut path = input.path;
        path.insert("operation_key".to_string(), operation_key.to_string());
        let context = OperationRequestContext {
            operation_key: operation_key.to_string(),
            method,
            path,
            query: input.query,
            body: input.body,
        };
        self.execute_and_audit(&definition, revision, context).await
    }

    async fn create_operation_revision_by_id(
        &self,
        operation_id: &str,
        input: OperationRevisionInput,
    ) -> anyhow::Result<OperationRevision> {
        validate_revision_input(&input)?;
        let next_revision = self.next_operation_revision(operation_id).await?;
        let revision = operation_revision_from_input(operation_id, next_revision, input)?;
        let mut db = self.db.lock().await;
        OperationRevision::create()
            .id(&revision.id)
            .operation_id(&revision.operation_id)
            .revision(revision.revision)
            .executor_kind(&revision.executor_kind)
            .source_text(&revision.source_text)
            .input_schema(revision.input_schema.0.clone())
            .output_schema(revision.output_schema.0.clone())
            .capability_policy(revision.capability_policy.0.clone())
            .timeout_ms(revision.timeout_ms)
            .generated_by_model(&revision.generated_by_model)
            .created_at_ms(revision.created_at_ms)
            .exec(&mut *db)
            .await
            .context("创建 engine operation revision 失败")
    }

    async fn next_operation_revision(&self, operation_id: &str) -> anyhow::Result<i32> {
        let mut db = self.db.lock().await;
        let revisions = Query::<List<OperationRevision>>::filter(
            OperationRevision::fields()
                .operation_id()
                .eq(operation_id),
        )
        .exec(&mut *db)
        .await
        .context("读取 engine operation revision 序号失败")?;
        let latest = revisions
            .into_iter()
            .map(|revision| revision.revision)
            .max();
        Ok(match latest {
            Some(revision) => revision + 1,
            None => 1,
        })
    }

    async fn get_operation_revision(
        &self,
        revision_id: &str,
    ) -> anyhow::Result<Option<OperationRevision>> {
        let mut db = self.db.lock().await;
        Query::<List<OperationRevision>>::filter(
            OperationRevision::fields().id().eq(revision_id),
        )
        .first()
        .exec(&mut *db)
        .await
        .context("查询 engine operation revision 失败")
    }

    async fn execute_and_audit(
        &self,
        definition: &OperationDefinition,
        revision: OperationRevision,
        context: OperationRequestContext,
    ) -> anyhow::Result<OperationInvocation> {
        let started = Instant::now();
        let executor = deserialize_executor(&revision.executor_kind, &revision.source_text)?;
        let execution = match executor {
            OperationExecutorDefinition::Plan(plan) => self.execute_plan(&plan, &context).await,
            OperationExecutorDefinition::Rhai { source_text } => {
                let execution_revision = revision.clone();
                let execution_context = context.clone();
                tokio::task::spawn_blocking(move || {
                    execute_rhai(&execution_revision, &execution_context, &source_text)
                })
                .await
                .context("等待 engine Rhai operation 执行任务失败")?
            }
        };
        let duration_ms = started.elapsed().as_millis() as i64;

        match execution {
            Ok(data) => {
                self.record_operation_run(
                    definition,
                    &revision,
                    &context,
                    &data,
                    "succeeded",
                    None,
                    duration_ms,
                )
                .await?;
                Ok(OperationInvocation {
                    operation_key: definition.operation_key.clone(),
                    revision: revision.revision,
                    data,
                    duration_ms,
                })
            }
            Err(error) => {
                let diagnostics = format!("{error:#}");
                self.record_operation_run(
                    definition,
                    &revision,
                    &context,
                    &Value::Null,
                    "failed",
                    Some(&diagnostics),
                    duration_ms,
                )
                .await?;
                Err(error)
            }
        }
    }

    async fn execute_plan(
        &self,
        plan: &OperationPlan,
        context: &OperationRequestContext,
    ) -> anyhow::Result<Value> {
        let mut result = Value::Null;
        for step in &plan.steps {
            result = match step {
                OperationPlanStep::ValidateInput => {
                    if !context.body.is_object() {
                        bail!("OperationPlan body 必须是 JSON object");
                    }
                    result
                }
                OperationPlanStep::QueryRecords => {
                    let page = page_params_from_query(&context.query)?;
                    serde_json::to_value(
                        self.executor().list_records(&plan.model_name, page).await?,
                    )
                    .context("序列化领域查询结果失败")?
                }
                OperationPlanStep::LoadRecord => {
                    let record_id = required_record_id(context)?;
                    serde_json::to_value(
                        self.executor()
                            .get_record(&plan.model_name, record_id)
                            .await?,
                    )
                    .context("序列化领域记录失败")?
                }
                OperationPlanStep::CreateRecord => serde_json::to_value(
                    self.executor()
                        .insert_record(&plan.model_name, context.body.clone())
                        .await?,
                )
                .context("序列化领域创建结果失败")?,
                OperationPlanStep::UpdateRecord => {
                    let record_id = required_record_id(context)?;
                    serde_json::to_value(
                        self.executor()
                            .update_record(&plan.model_name, record_id, context.body.clone())
                            .await?,
                    )
                    .context("序列化领域更新结果失败")?
                }
                OperationPlanStep::DeleteRecord => {
                    let record_id = required_record_id(context)?;
                    self.executor()
                        .delete_record(&plan.model_name, record_id)
                        .await?;
                    json!({"id": record_id, "deleted": true})
                }
                OperationPlanStep::InvokeCapability { capability } => {
                    self.operation_capabilities
                        .execute(capability, context.body.clone(), context)
                        .await?
                }
                OperationPlanStep::ReturnResult => result,
            };
        }
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_operation_run(
        &self,
        definition: &OperationDefinition,
        revision: &OperationRevision,
        context: &OperationRequestContext,
        response: &Value,
        status: &str,
        diagnostics: Option<&str>,
        duration_ms: i64,
    ) -> anyhow::Result<OperationRun> {
        let request_context = serde_json::to_value(context).context("序列化 operation 请求上下文失败")?;
        let diagnostics = diagnostics.map(str::to_string);
        let id = uuid::Uuid::new_v4().to_string();
        let mut db = self.db.lock().await;
        OperationRun::create()
            .id(&id)
            .operation_id(&definition.id)
            .revision_id(&revision.id)
            .request_context(request_context)
            .response(response.clone())
            .status(status)
            .diagnostics(&diagnostics)
            .duration_ms(duration_ms)
            .created_at_ms(timestamp_ms())
            .exec(&mut *db)
            .await
            .context("写入 engine operation 执行审计失败")
    }
}

/// 通过受限 Rhai 运行指定 revision。
pub fn execute_revision(
    revision: &OperationRevision,
    context: &OperationRequestContext,
) -> anyhow::Result<Value> {
    let executor = deserialize_executor(&revision.executor_kind, &revision.source_text)?;
    let OperationExecutorDefinition::Rhai { source_text } = executor else {
        bail!("同步执行入口只支持 Rhai operation");
    };
    execute_rhai(revision, context, &source_text)
}

fn execute_rhai(
    revision: &OperationRevision,
    context: &OperationRequestContext,
    source_text: &str,
) -> anyhow::Result<Value> {
    let engine = restricted_rhai_engine(revision.timeout_ms)?;
    let mut scope = Scope::new();
    let context_value = serde_json::to_value(context).context("序列化 operation 请求上下文失败")?;
    let request = serde_json::from_value::<Dynamic>(context_value)
        .context("注入 operation request 失败")?;
    let body = serde_json::from_value::<Dynamic>(context.body.clone())
        .context("注入 operation body 失败")?;
    let query_value = serde_json::to_value(&context.query).context("序列化 operation query 失败")?;
    let query = serde_json::from_value::<Dynamic>(query_value)
        .context("注入 operation query 失败")?;

    scope.push_dynamic("request", request);
    scope.push_dynamic("body", body);
    scope.push_dynamic("query", query);
    scope.push("operation_key", context.operation_key.clone());
    scope.push("method", context.method.clone());

    let result = engine
        .eval_with_scope::<Dynamic>(&mut scope, source_text)
        .map_err(|error| anyhow!("Rhai operation 执行失败: {error}"))?;
    serde_json::to_value(result).context("导出 Rhai operation 返回值失败")
}

pub(crate) fn operation_revision_from_input(
    operation_id: &str,
    revision: i32,
    input: OperationRevisionInput,
) -> anyhow::Result<OperationRevision> {
    let (executor_kind, source_text) = serialize_executor(&input.executor)?;
    Ok(OperationRevision {
        id: uuid::Uuid::new_v4().to_string(),
        operation_id: operation_id.to_string(),
        revision,
        executor_kind,
        source_text,
        input_schema: toasty::Json(input.input_schema),
        output_schema: toasty::Json(input.output_schema),
        capability_policy: toasty::Json(input.capability_policy),
        timeout_ms: input.timeout_ms,
        generated_by_model: input.generated_by_model,
        created_at_ms: timestamp_ms(),
    })
}

pub(crate) fn validate_operation_draft(draft: &OperationDraft) -> anyhow::Result<()> {
    validate_operation_key(&draft.operation_key)?;
    if draft.display_name.trim().is_empty() {
        bail!("operation 显示名不能为空");
    }
    let _ = normalize_method(&draft.method)?;
    validate_revision_input(&OperationRevisionInput {
        executor: draft.executor.clone(),
        input_schema: draft.input_schema.clone(),
        output_schema: draft.output_schema.clone(),
        capability_policy: draft.capability_policy.clone(),
        timeout_ms: draft.timeout_ms,
        generated_by_model: draft.generated_by_model.clone(),
    })
}

fn validate_revision_input(input: &OperationRevisionInput) -> anyhow::Result<()> {
    if !(1..=MAX_TIMEOUT_MS).contains(&input.timeout_ms) {
        bail!("operation timeout_ms 必须在 1..={MAX_TIMEOUT_MS} 范围内");
    }
    validate_schema_object(&input.input_schema, "input_schema")?;
    validate_schema_object(&input.output_schema, "output_schema")?;
    validate_schema_object(&input.capability_policy, "capability_policy")?;
    match &input.executor {
        OperationExecutorDefinition::Plan(plan) => validate_operation_plan(plan)?,
        OperationExecutorDefinition::Rhai { source_text } => {
            if source_text.trim().is_empty() {
                bail!("Rhai operation source_text 不能为空");
            }
            if source_text.len() > MAX_SOURCE_BYTES {
                bail!("Rhai operation source_text 不能超过 {MAX_SOURCE_BYTES} 字节");
            }
            let engine = restricted_rhai_engine(input.timeout_ms)?;
            let _ = engine
                .compile(source_text)
                .map_err(|error| anyhow!("Rhai operation 编译失败: {error}"))?;
        }
    }
    Ok(())
}

fn validate_operation_plan(plan: &OperationPlan) -> anyhow::Result<()> {
    if plan.model_name.trim().is_empty() {
        bail!("OperationPlan model_name 不能为空");
    }
    if plan.steps.is_empty() {
        bail!("OperationPlan 至少需要一个步骤");
    }
    if !matches!(plan.steps.last(), Some(OperationPlanStep::ReturnResult)) {
        bail!("OperationPlan 最后一步必须返回结果");
    }
    Ok(())
}

fn required_record_id(context: &OperationRequestContext) -> anyhow::Result<&str> {
    context
        .path
        .get("id")
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("领域操作缺少记录 id"))
}

fn page_params_from_query(
    query: &BTreeMap<String, Vec<String>>,
) -> anyhow::Result<PageParams> {
    let offset = query_number(query, "o")?.unwrap_or(0);
    let size = query_number(query, "s")?.unwrap_or(50);
    if size == 0 || size > 200 {
        bail!("分页参数 s 必须在 1..=200 范围内");
    }
    Ok(PageParams { o: offset, s: size })
}

fn query_number(
    query: &BTreeMap<String, Vec<String>>,
    key: &str,
) -> anyhow::Result<Option<usize>> {
    let Some(value) = query.get(key).and_then(|values| values.first()) else {
        return Ok(None);
    };
    value
        .parse::<usize>()
        .map(Some)
        .with_context(|| format!("分页参数 {key} 不是非负整数"))
}

fn serialize_executor(
    executor: &OperationExecutorDefinition,
) -> anyhow::Result<(String, String)> {
    match executor {
        OperationExecutorDefinition::Plan(plan) => Ok((
            EXECUTOR_PLAN.to_string(),
            serde_json::to_string(plan).context("序列化 OperationPlan 失败")?,
        )),
        OperationExecutorDefinition::Rhai { source_text } => {
            Ok((EXECUTOR_RHAI.to_string(), source_text.clone()))
        }
    }
}

fn deserialize_executor(
    executor_kind: &str,
    source_text: &str,
) -> anyhow::Result<OperationExecutorDefinition> {
    match executor_kind {
        EXECUTOR_PLAN => serde_json::from_str(source_text)
            .map(OperationExecutorDefinition::Plan)
            .context("解析 OperationPlan 失败"),
        EXECUTOR_RHAI => Ok(OperationExecutorDefinition::Rhai {
            source_text: source_text.to_string(),
        }),
        other => bail!("不支持的 operation executor: {other}"),
    }
}

fn validate_operation_key(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        bail!("operation_key 不能为空");
    }
    let valid = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
    if !valid {
        bail!("operation_key 只能包含 ASCII 字母、数字、点、横线和下划线: {value}");
    }
    Ok(())
}

fn normalize_method(value: &str) -> anyhow::Result<String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok("GET".to_string()),
        "POST" => Ok("POST".to_string()),
        other => bail!("首版 operation 只支持 GET/POST，收到: {other}"),
    }
}

fn validate_schema_object(value: &Value, label: &str) -> anyhow::Result<()> {
    if value.is_object() {
        return Ok(());
    }
    bail!("operation {label} 必须是 JSON object")
}

fn restricted_rhai_engine(timeout_ms: i64) -> anyhow::Result<RhaiEngine> {
    if !(1..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
        bail!("operation timeout_ms 必须在 1..={MAX_TIMEOUT_MS} 范围内");
    }
    let started = Instant::now();
    let timeout_ms = timeout_ms as u128;
    let mut engine = RhaiEngine::new();
    engine.set_max_operations(1_000_000);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_variables(256);
    engine.set_max_functions(128);
    engine.set_max_modules(16);
    engine.set_max_string_size(1024 * 1024);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);
    engine.disable_symbol("eval");
    engine.on_progress(move |_| {
        if started.elapsed().as_millis() >= timeout_ms {
            Some("operation 执行超时".into())
        } else {
            None
        }
    });
    Ok(engine)
}

fn default_method() -> String {
    "POST".to_string()
}

fn default_object_schema() -> Value {
    json!({ "type": "object" })
}

fn default_capability_policy() -> Value {
    json!({ "allow": [] })
}

fn default_timeout_ms() -> i64 {
    DEFAULT_TIMEOUT_MS
}

fn default_body() -> Value {
    json!({})
}

/// 通过真实查询确认 operation 三张表均可用。
pub(crate) async fn verify_operation_schema(db: &mut toasty::Db) -> anyhow::Result<()> {
    let mut definitions = Query::<List<OperationDefinition>>::all();
    definitions.limit(1);
    definitions
        .exec(&mut *db)
        .await
        .context("校验 engine_operation_definitions 表失败")?;

    let mut revisions = Query::<List<OperationRevision>>::all();
    revisions.limit(1);
    revisions
        .exec(&mut *db)
        .await
        .context("校验 engine_operation_revisions 表失败")?;

    let mut runs = Query::<List<OperationRun>>::all();
    runs.limit(1);
    runs.exec(&mut *db)
        .await
        .context("校验 engine_operation_runs 表失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(source_text: &str) -> OperationRevision {
        OperationRevision {
            id: "revision-1".to_string(),
            operation_id: "operation-1".to_string(),
            revision: 1,
            executor_kind: EXECUTOR_RHAI.to_string(),
            source_text: source_text.to_string(),
            input_schema: toasty::Json(default_object_schema()),
            output_schema: toasty::Json(default_object_schema()),
            capability_policy: toasty::Json(default_capability_policy()),
            timeout_ms: 500,
            generated_by_model: None,
            created_at_ms: 0,
        }
    }

    fn request_context() -> OperationRequestContext {
        OperationRequestContext {
            operation_key: "device.power".to_string(),
            method: "POST".to_string(),
            path: BTreeMap::from([("operation_key".to_string(), "device.power".to_string())]),
            query: BTreeMap::from([(
                "startTime".to_string(),
                vec!["2026-07-23".to_string()],
            )]),
            body: json!({ "deviceId": "961000001008" }),
        }
    }

    #[test]
    fn rhai_operation_reads_namespaced_request_context() {
        let revision = revision(
            r#"#{ device_id: body.deviceId, start_time: query.startTime[0], key: operation_key }"#,
        );

        let output = execute_revision(&revision, &request_context());
        let output = match output {
            Ok(value) => value,
            Err(error) => panic!("Rhai operation 应执行成功: {error:#}"),
        };

        // 三类请求参数必须保持命名空间，并可由动态脚本组合返回。
        assert_eq!(output["device_id"], "961000001008");
        assert_eq!(output["start_time"], "2026-07-23");
        assert_eq!(output["key"], "device.power");
    }

    #[test]
    fn rejects_invalid_operation_source_before_persistence() {
        let input = OperationRevisionInput {
            executor: OperationExecutorDefinition::Rhai {
                source_text: "let = ;".to_string(),
            },
            input_schema: default_object_schema(),
            output_schema: default_object_schema(),
            capability_policy: default_capability_policy(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            generated_by_model: None,
        };

        let error = match validate_revision_input(&input) {
            Ok(()) => panic!("非法 Rhai 不应通过 revision 校验"),
            Err(error) => error,
        };

        // 在线生成的脚本必须在写库前暴露准确的编译错误。
        assert!(error.to_string().contains("Rhai operation 编译失败"));
    }

    #[test]
    fn rejects_operation_plan_without_explicit_return() {
        let draft = OperationDraft {
            operation_key: "device.power".to_string(),
            display_name: "设备电力".to_string(),
            description: String::new(),
            method: "POST".to_string(),
            executor: OperationExecutorDefinition::Plan(OperationPlan {
                model_name: "device".to_string(),
                steps: vec![OperationPlanStep::QueryRecords],
            }),
            input_schema: default_object_schema(),
            output_schema: default_object_schema(),
            capability_policy: default_capability_policy(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            generated_by_model: None,
        };

        // 类型协议不再允许 shell，且计划必须显式返回结果。
        assert!(validate_operation_draft(&draft).is_err());
    }
}
