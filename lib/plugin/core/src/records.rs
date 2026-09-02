//! engine 公共 API。
//!
//! 本模块集中暴露低代码引擎的元数据模型、持久化 store 和执行管道。

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use evalexpr::{ContextWithMutableVariables, HashMapContext, Value as EvalValue};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, postgres::PgPoolOptions, types::Json as SqlJson};
use toasty::stmt::{List, Query};

use crate::database::{timestamp_ms, verify_database_url};

/// 元模型，描述一类动态业务记录。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "engine_meta_models"]
pub struct MetaModel {
    #[key]
    pub id: String,
    #[index]
    pub name: String,
    pub display_name: String,
    pub primary_key_generation: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 元字段，描述动态记录 payload 中的一个字段。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "engine_meta_fields"]
pub struct MetaField {
    #[key]
    pub id: String,
    #[index]
    pub model_name: String,
    pub name: String,
    pub display_name: String,
    pub field_type: String,
    pub is_required: bool,
    pub expression: Option<String>,
    pub dependency_json: Option<String>,
    pub domain_metadata_json: Option<String>,
    pub validation_json: Option<String>,
    pub order_index: i32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 动态业务记录，payload 使用 Toasty JSON 文档字段承载。
#[derive(Clone, Debug, PartialEq, toasty::Model)]
#[table = "engine_data_records"]
pub struct DataRecord {
    #[key]
    pub id: String,
    #[index]
    pub model_name: String,
    pub payload: toasty::Json<Value>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 计算字段的显式跨模型依赖。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComputedDependency {
    pub alias: String,
    pub source_model_name: String,
    pub local_field: String,
    pub source_payload_field: String,
}

/// 页式查询结果，遵循 `d/t/p` 约定。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageData<T> {
    pub d: Vec<T>,
    pub t: u64,
    pub p: PageParams,
}

/// 页式查询参数，遵循 `o/s` 约定。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageParams {
    pub o: usize,
    pub s: usize,
}

impl Default for PageParams {
    fn default() -> Self {
        Self { o: 0, s: 50 }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordFilterOperator {
    Equals,
    Contains,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFilter {
    pub field: String,
    pub operator: RecordFilterOperator,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordSortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSort {
    pub field: String,
    pub direction: RecordSortDirection,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordCriteria {
    pub all: Vec<RecordFilter>,
    pub any: Vec<RecordFilter>,
    pub sort: Option<RecordSort>,
}

/// 对外返回的记录视图。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataRecordView {
    pub id: String,
    pub model_name: String,
    pub payload: Value,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl From<DataRecord> for DataRecordView {
    fn from(record: DataRecord) -> Self {
        Self {
            id: record.id,
            model_name: record.model_name,
            payload: record.payload.0,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

/// 新建模型输入。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelInput {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub primary_key_generation: RecordIdGeneration,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordIdGeneration {
    #[default]
    Uuid,
    AutoIncrement,
}

impl RecordIdGeneration {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uuid => "uuid",
            Self::AutoIncrement => "auto_increment",
        }
    }
}

/// 新建或更新字段输入。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldInput {
    pub name: String,
    pub display_name: String,
    pub field_type: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub dependency_json: Option<String>,
    #[serde(default)]
    pub domain_metadata_json: Option<String>,
    #[serde(default)]
    pub validation_json: Option<String>,
    #[serde(default)]
    pub order_index: i32,
}

/// engine 持久化 store。
#[derive(Clone)]
pub struct RecordStore {
    pub(crate) db: Arc<tokio::sync::Mutex<toasty::Db>>,
    pool: PgPool,
}

/// engine 执行器。
#[derive(Clone)]
pub struct EngineExecutor {
    store: RecordStore,
}

/// 集合化计算字段求值器。
#[derive(Clone)]
pub struct BatchComputedEvaluator {
    store: RecordStore,
}

impl RecordStore {
    /// 连接已完成 SQLx 迁移的 PostgreSQL 并验证 engine schema。
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let database_url = verify_database_url(database_url)?;
        let db = toasty::Db::builder()
            .models(engine_models())
            .connect(database_url)
            .await
            .context("连接 engine PostgreSQL 失败")?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("连接 engine SQLx PostgreSQL 失败")?;
        verify_existing_schema(&db, &pool).await?;
        Ok(Self::new(db, pool))
    }

    /// 包装已经完成配置的 Toasty 数据库。
    pub fn new(db: toasty::Db, pool: PgPool) -> Self {
        Self {
            db: Arc::new(tokio::sync::Mutex::new(db)),
            pool,
        }
    }

    /// 复用应用级 Toasty 执行器单例。
    pub fn from_shared_db(db: Arc<tokio::sync::Mutex<toasty::Db>>, pool: PgPool) -> Self {
        Self { db, pool }
    }

    /// 创建执行器。
    pub fn executor(&self) -> EngineExecutor {
        EngineExecutor {
            store: self.clone(),
        }
    }

    /// 新建模型。
    pub async fn create_model(&self, input: ModelInput) -> anyhow::Result<MetaModel> {
        validate_identifier(&input.name, "模型名")?;
        let now = timestamp_ms();
        let model = MetaModel {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            display_name: input.display_name,
            primary_key_generation: input.primary_key_generation.as_str().to_owned(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        let mut db = self.db.lock().await;
        let created = MetaModel::create()
            .id(&model.id)
            .name(&model.name)
            .display_name(&model.display_name)
            .primary_key_generation(&model.primary_key_generation)
            .created_at_ms(model.created_at_ms)
            .updated_at_ms(model.updated_at_ms)
            .exec(&mut *db)
            .await
            .context("创建 engine 模型失败")?;
        Ok(created)
    }

    /// 查询模型列表。
    pub async fn list_models(&self, page: PageParams) -> anyhow::Result<PageData<MetaModel>> {
        let mut db = self.db.lock().await;
        let total = Query::<List<MetaModel>>::all()
            .count()
            .exec(&mut *db)
            .await
            .context("统计 engine 模型失败")?;
        let mut query = Query::<List<MetaModel>>::all();
        query.limit(page.s);
        query.offset(page.o);
        let rows = query.exec(&mut *db).await.context("查询 engine 模型失败")?;
        Ok(PageData {
            d: rows,
            t: total,
            p: page,
        })
    }

    /// 按模型名查询模型。
    pub async fn get_model(&self, model_name: &str) -> anyhow::Result<Option<MetaModel>> {
        let mut db = self.db.lock().await;
        Query::<List<MetaModel>>::filter(MetaModel::fields().name().eq(model_name))
            .first()
            .exec(&mut *db)
            .await
            .context("查询 engine 模型失败")
    }

    /// 更新模型显示信息，模型名作为路由身份不在此处重命名。
    pub async fn update_model(
        &self,
        model_name: &str,
        input: ModelInput,
    ) -> anyhow::Result<MetaModel> {
        validate_identifier(&input.name, "模型名")?;
        if input.name != model_name {
            bail!(
                "模型名不支持通过 update 改名: {model_name} -> {}",
                input.name
            );
        }
        let existing = self.ensure_model(model_name).await?;
        if existing.primary_key_generation != input.primary_key_generation.as_str() {
            let has_records = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM engine_data_records WHERE model_name = $1)",
            )
            .bind(model_name)
            .fetch_one(&self.pool)
            .await
            .context("检查 engine 模型历史记录失败")?;
            if has_records {
                bail!("模型 {model_name} 已存在业务记录，不能切换主键生成策略");
            }
        }
        let now = timestamp_ms();
        {
            let mut db = self.db.lock().await;
            MetaModel::filter(MetaModel::fields().name().eq(model_name))
                .update()
                .display_name(&input.display_name)
                .primary_key_generation(input.primary_key_generation.as_str())
                .updated_at_ms(now)
                .exec(&mut *db)
                .await
                .context("更新 engine 模型失败")?;
        }
        self.get_model(model_name)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {model_name}"))
    }

    /// 删除模型以及其字段和记录。
    pub async fn delete_model(&self, model_name: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM engine_data_records WHERE model_name = $1")
            .bind(model_name)
            .execute(&self.pool)
            .await
            .context("删除 engine 模型记录失败")?;
        let mut db = self.db.lock().await;
        MetaField::filter(MetaField::fields().model_name().eq(model_name))
            .delete()
            .exec(&mut *db)
            .await
            .context("删除 engine 模型字段失败")?;
        MetaModel::filter(MetaModel::fields().name().eq(model_name))
            .delete()
            .exec(&mut *db)
            .await
            .context("删除 engine 模型失败")?;
        Ok(())
    }

    /// 创建字段。
    pub async fn create_field(
        &self,
        model_name: &str,
        input: FieldInput,
    ) -> anyhow::Result<MetaField> {
        self.ensure_model(model_name).await?;
        validate_identifier(&input.name, "字段名")?;
        validate_field_type(&input.field_type)?;
        validate_dependency_json(input.dependency_json.as_deref())?;
        validate_optional_json(input.domain_metadata_json.as_deref(), "领域元数据")?;
        validate_optional_json(input.validation_json.as_deref(), "校验定义")?;
        let now = timestamp_ms();
        let field = MetaField {
            id: uuid::Uuid::new_v4().to_string(),
            model_name: model_name.to_string(),
            name: input.name,
            display_name: input.display_name,
            field_type: normalize_field_type(&input.field_type),
            is_required: input.is_required,
            expression: empty_to_none(input.expression),
            dependency_json: empty_to_none(input.dependency_json),
            domain_metadata_json: empty_to_none(input.domain_metadata_json),
            validation_json: empty_to_none(input.validation_json),
            order_index: input.order_index,
            created_at_ms: now,
            updated_at_ms: now,
        };
        let mut db = self.db.lock().await;
        let created = MetaField::create()
            .id(&field.id)
            .model_name(&field.model_name)
            .name(&field.name)
            .display_name(&field.display_name)
            .field_type(&field.field_type)
            .is_required(field.is_required)
            .expression(&field.expression)
            .dependency_json(&field.dependency_json)
            .domain_metadata_json(&field.domain_metadata_json)
            .validation_json(&field.validation_json)
            .order_index(field.order_index)
            .created_at_ms(field.created_at_ms)
            .updated_at_ms(field.updated_at_ms)
            .exec(&mut *db)
            .await
            .context("创建 engine 字段失败")?;
        Ok(created)
    }

    /// 查询模型字段。
    pub async fn list_fields(&self, model_name: &str) -> anyhow::Result<Vec<MetaField>> {
        let mut db = self.db.lock().await;
        let mut fields =
            Query::<List<MetaField>>::filter(MetaField::fields().model_name().eq(model_name))
                .exec(&mut *db)
                .await
                .context("查询 engine 字段失败")?;
        fields.sort_by(|left, right| {
            left.order_index
                .cmp(&right.order_index)
                .then(left.name.cmp(&right.name))
        });
        Ok(fields)
    }

    /// 按字段 ID 查询字段。
    pub async fn get_field(&self, field_id: &str) -> anyhow::Result<Option<MetaField>> {
        let mut db = self.db.lock().await;
        Query::<List<MetaField>>::filter(MetaField::fields().id().eq(field_id))
            .first()
            .exec(&mut *db)
            .await
            .context("查询 engine 字段失败")
    }

    /// 更新字段定义。
    pub async fn update_field(
        &self,
        field_id: &str,
        input: FieldInput,
    ) -> anyhow::Result<MetaField> {
        validate_identifier(&input.name, "字段名")?;
        validate_field_type(&input.field_type)?;
        validate_dependency_json(input.dependency_json.as_deref())?;
        validate_optional_json(input.domain_metadata_json.as_deref(), "领域元数据")?;
        validate_optional_json(input.validation_json.as_deref(), "校验定义")?;
        let now = timestamp_ms();
        let expression = empty_to_none(input.expression);
        let dependency_json = empty_to_none(input.dependency_json);
        let domain_metadata_json = empty_to_none(input.domain_metadata_json);
        let validation_json = empty_to_none(input.validation_json);
        {
            let mut db = self.db.lock().await;
            MetaField::filter(MetaField::fields().id().eq(field_id))
                .update()
                .name(&input.name)
                .display_name(&input.display_name)
                .field_type(normalize_field_type(&input.field_type))
                .is_required(input.is_required)
                .expression(&expression)
                .dependency_json(&dependency_json)
                .domain_metadata_json(&domain_metadata_json)
                .validation_json(&validation_json)
                .order_index(input.order_index)
                .updated_at_ms(now)
                .exec(&mut *db)
                .await
                .context("更新 engine 字段失败")?;
        }
        self.get_field(field_id)
            .await?
            .ok_or_else(|| anyhow!("字段不存在: {field_id}"))
    }

    /// 删除字段。
    pub async fn delete_field(&self, field_id: &str) -> anyhow::Result<()> {
        let mut db = self.db.lock().await;
        MetaField::filter(MetaField::fields().id().eq(field_id))
            .delete()
            .exec(&mut *db)
            .await
            .context("删除 engine 字段失败")?;
        Ok(())
    }

    /// 按模型和记录 ID 查询记录。
    pub async fn get_record(
        &self,
        model_name: &str,
        record_id: &str,
    ) -> anyhow::Result<Option<DataRecord>> {
        sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records WHERE model_name = $1 AND id = $2",
        )
        .bind(model_name)
        .bind(record_id)
        .fetch_optional(&self.pool)
        .await
        .context("查询 engine 记录失败")?
        .as_ref()
        .map(data_record_from_row)
        .transpose()
    }

    /// 查询模型原始记录，不执行计算字段。
    pub async fn list_raw_records(&self, model_name: &str) -> anyhow::Result<Vec<DataRecord>> {
        let rows = sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records WHERE model_name = $1 ORDER BY created_at_ms, id",
        )
        .bind(model_name)
        .fetch_all(&self.pool)
        .await
        .context("查询 engine 原始记录失败")?;
        rows.iter().map(data_record_from_row).collect()
    }

    /// 分页查询模型原始记录，不执行计算字段。
    pub async fn list_raw_records_page(
        &self,
        model_name: &str,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecord>> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM engine_data_records WHERE model_name = $1",
        )
        .bind(model_name)
        .fetch_one(&self.pool)
        .await
        .context("统计 engine 记录失败")?;
        let rows = sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records WHERE model_name = $1
             ORDER BY created_at_ms, id OFFSET $2 LIMIT $3",
        )
        .bind(model_name)
        .bind(page.o as i64)
        .bind(page.s as i64)
        .fetch_all(&self.pool)
        .await
        .context("分页查询 engine 记录失败")?;
        let records = rows
            .iter()
            .map(data_record_from_row)
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(PageData {
            d: records,
            t: total as u64,
            p: page,
        })
    }

    /// 按结构化条件筛选、排序并分页查询原始记录。
    pub async fn list_raw_records_page_with_criteria(
        &self,
        model_name: &str,
        criteria: &RecordCriteria,
        sort_field_type: Option<&str>,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecord>> {
        let mut count = QueryBuilder::<Postgres>::new(
            "SELECT COUNT(*) FROM engine_data_records WHERE model_name = ",
        );
        count.push_bind(model_name.to_owned());
        push_record_criteria_predicate(&mut count, criteria);
        let total = count
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .context("统计筛选后的 engine 记录失败")?;

        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms \
             FROM engine_data_records WHERE model_name = ",
        );
        query.push_bind(model_name.to_owned());
        push_record_criteria_predicate(&mut query, criteria);
        push_record_sort(&mut query, criteria.sort.as_ref(), sort_field_type);
        query.push(" OFFSET ");
        query.push_bind(page.o as i64);
        query.push(" LIMIT ");
        query.push_bind(page.s as i64);
        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await
            .context("分页查询筛选后的 engine 记录失败")?;
        let records = rows
            .iter()
            .map(data_record_from_row)
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(PageData {
            d: records,
            t: total as u64,
            p: page,
        })
    }

    async fn ensure_model(&self, model_name: &str) -> anyhow::Result<MetaModel> {
        self.get_model(model_name)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {model_name}"))
    }

    async fn persist_record(&self, model_name: &str, payload: Value) -> anyhow::Result<DataRecord> {
        let now = timestamp_ms();
        let model = self.ensure_model(model_name).await?;
        let id = match model.primary_key_generation.as_str() {
            "uuid" => uuid::Uuid::new_v4().to_string(),
            "auto_increment" => {
                sqlx::query_scalar::<_, i64>("SELECT nextval('engine_data_record_auto_id_seq')")
                    .fetch_one(&self.pool)
                    .await
                    .context("分配 engine 记录自增主键失败")?
                    .to_string()
            }
            value => bail!("模型主键生成策略无效: {value}"),
        };
        sqlx::query(
            "INSERT INTO engine_data_records
             (id, model_name, payload, created_at_ms, updated_at_ms)
             VALUES ($1, $2, $3, $4, $4)",
        )
        .bind(&id)
        .bind(model_name)
        .bind(SqlJson(&payload))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("创建 engine 记录失败")?;
        Ok(DataRecord {
            id,
            model_name: model_name.to_owned(),
            payload: toasty::Json(payload),
            created_at_ms: now,
            updated_at_ms: now,
        })
    }

    async fn replace_record_payload(
        &self,
        model_name: &str,
        record_id: &str,
        payload: Value,
    ) -> anyhow::Result<DataRecord> {
        let now = timestamp_ms();
        sqlx::query(
            "UPDATE engine_data_records SET payload = $1, updated_at_ms = $2
             WHERE model_name = $3 AND id = $4",
        )
        .bind(SqlJson(&payload))
        .bind(now)
        .bind(model_name)
        .bind(record_id)
        .execute(&self.pool)
        .await
        .context("更新 engine 记录失败")?;
        self.get_record(model_name, record_id)
            .await?
            .ok_or_else(|| anyhow!("记录不存在: {model_name}/{record_id}"))
    }

    async fn delete_record(&self, model_name: &str, record_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM engine_data_records WHERE model_name = $1 AND id = $2")
            .bind(model_name)
            .bind(record_id)
            .execute(&self.pool)
            .await
            .context("删除 engine 记录失败")?;
        Ok(())
    }
}

include!("record_query_sql.rs");

include!("record_engine.rs");

include!("record_validation_runtime.rs");

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
