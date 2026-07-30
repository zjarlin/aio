//! engine 公共 API。
//!
//! 本模块集中暴露低代码引擎的元数据模型、持久化 store、执行管道和 API 常量。

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use evalexpr::{ContextWithMutableVariables, HashMapContext, Value as EvalValue};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Row, postgres::PgPoolOptions, types::Json as SqlJson};
use toasty::stmt::{List, Query};

use crate::database::{timestamp_ms, verify_database_url};

/// engine Toasty 表名前缀。
pub const TABLE_NAME_PREFIX: &str = "engine_";

/// engine REST API 前缀。
pub const API_PREFIX: &str = "/api/engine";

/// 模型集合 API 路径。
pub const MODELS_PATH: &str = "/api/engine/models";

/// 字段集合 API 路径模板说明。
pub const FIELDS_PATH_TEMPLATE: &str = "/api/engine/models/{model_name}/fields";

/// 记录集合 API 路径模板说明。
pub const RECORDS_PATH_TEMPLATE: &str = "/api/engine/models/{model_name}/records";

/// 模型列表操作 ID。
pub const OP_MODELS_LIST: &str = "engine.models.list";

/// 模型创建操作 ID。
pub const OP_MODELS_CREATE: &str = "engine.models.create";

/// 模型更新操作 ID。
pub const OP_MODELS_UPDATE: &str = "engine.models.update";

/// 字段列表操作 ID。
pub const OP_FIELDS_LIST: &str = "engine.fields.list";

/// 字段创建操作 ID。
pub const OP_FIELDS_CREATE: &str = "engine.fields.create";

/// 字段更新操作 ID。
pub const OP_FIELDS_UPDATE: &str = "engine.fields.update";

/// 记录列表操作 ID。
pub const OP_RECORDS_LIST: &str = "engine.records.list";

/// 记录创建操作 ID。
pub const OP_RECORDS_CREATE: &str = "engine.records.create";

/// 记录更新操作 ID。
pub const OP_RECORDS_UPDATE: &str = "engine.records.update";

/// 元模型，描述一类动态业务记录。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, toasty::Model)]
#[table = "engine_meta_models"]
pub struct MetaModel {
    #[key]
    pub id: String,
    #[index]
    pub name: String,
    pub display_name: String,
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
            created_at_ms: now,
            updated_at_ms: now,
        };
        let mut db = self.db.lock().await;
        let created = MetaModel::create()
            .id(&model.id)
            .name(&model.name)
            .display_name(&model.display_name)
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
        let now = timestamp_ms();
        {
            let mut db = self.db.lock().await;
            MetaModel::filter(MetaModel::fields().name().eq(model_name))
                .update()
                .display_name(&input.display_name)
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

    /// 按 payload 字段精确筛选后分页查询原始记录。
    pub async fn list_raw_records_page_by_field(
        &self,
        model_name: &str,
        field_name: &str,
        field_value: &str,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecord>> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM engine_data_records
             WHERE model_name = $1 AND payload ->> $2 = $3",
        )
        .bind(model_name)
        .bind(field_name)
        .bind(field_value)
        .fetch_one(&self.pool)
        .await
        .context("统计筛选后的 engine 记录失败")?;
        let rows = sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records
             WHERE model_name = $1 AND payload ->> $2 = $3
             ORDER BY created_at_ms, id OFFSET $4 LIMIT $5",
        )
        .bind(model_name)
        .bind(field_name)
        .bind(field_value)
        .bind(page.o as i64)
        .bind(page.s as i64)
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
        let id = uuid::Uuid::new_v4().to_string();
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

impl EngineExecutor {
    /// 插入记录并执行完整写入管道。
    pub async fn insert_record(
        &self,
        model_name: &str,
        raw_payload: Value,
    ) -> anyhow::Result<DataRecordView> {
        let fields = self.store.list_fields(model_name).await?;
        let payload = value_to_object(raw_payload)?;
        validate_payload(&fields, &payload, false)?;
        let mut values = vec![Value::Object(payload)];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut values)
            .await?;
        let payload = value_to_object(values.remove(0))?;
        validate_payload(&fields, &payload, true)?;
        let record = self
            .store
            .persist_record(model_name, Value::Object(payload))
            .await?;
        Ok(record.into())
    }

    /// 更新记录并执行完整写入管道。
    pub async fn update_record(
        &self,
        model_name: &str,
        record_id: &str,
        raw_payload: Value,
    ) -> anyhow::Result<DataRecordView> {
        let existing = self
            .store
            .get_record(model_name, record_id)
            .await?
            .ok_or_else(|| anyhow!("记录不存在: {model_name}/{record_id}"))?;
        let fields = self.store.list_fields(model_name).await?;
        let mut payload = value_to_object(existing.payload.0)?;
        for (key, value) in value_to_object(raw_payload)? {
            payload.insert(key, value);
        }
        validate_payload(&fields, &payload, false)?;
        let mut values = vec![Value::Object(payload)];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut values)
            .await?;
        let payload = value_to_object(values.remove(0))?;
        validate_payload(&fields, &payload, true)?;
        let record = self
            .store
            .replace_record_payload(model_name, record_id, Value::Object(payload))
            .await?;
        Ok(record.into())
    }

    /// 查询单条记录并执行 computed 字段求值。
    pub async fn get_record(
        &self,
        model_name: &str,
        record_id: &str,
    ) -> anyhow::Result<DataRecordView> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        let record = self
            .store
            .get_record(model_name, record_id)
            .await?
            .ok_or_else(|| anyhow!("记录不存在: {model_name}/{record_id}"))?;
        let mut payloads = vec![record.payload.0.clone()];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut payloads)
            .await?;
        let payload = payloads.remove(0);
        Ok(DataRecordView {
            id: record.id,
            model_name: record.model_name,
            payload,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        })
    }

    /// 删除记录。
    pub async fn delete_record(&self, model_name: &str, record_id: &str) -> anyhow::Result<()> {
        self.store.delete_record(model_name, record_id).await
    }

    /// 查询记录并执行集合化计算字段。
    pub async fn list_records(
        &self,
        model_name: &str,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        let raw = self.store.list_raw_records_page(model_name, page).await?;
        self.evaluate_record_page(model_name, &fields, raw).await
    }

    /// 按模型字段精确筛选记录并执行集合化计算字段。
    pub async fn list_records_by_field(
        &self,
        model_name: &str,
        field_name: &str,
        field_value: &str,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        if !fields.iter().any(|field| field.name == field_name) {
            bail!("筛选字段不存在: {model_name}.{field_name}");
        }
        let raw = self
            .store
            .list_raw_records_page_by_field(model_name, field_name, field_value, page)
            .await?;
        self.evaluate_record_page(model_name, &fields, raw).await
    }

    async fn evaluate_record_page(
        &self,
        model_name: &str,
        fields: &[MetaField],
        raw: PageData<DataRecord>,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        let mut payloads = raw
            .d
            .iter()
            .map(|record| record.payload.0.clone())
            .collect::<Vec<_>>();
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut payloads)
            .await?;
        let rows = raw
            .d
            .into_iter()
            .zip(payloads)
            .map(|(record, payload)| DataRecordView {
                id: record.id,
                model_name: record.model_name,
                payload,
                created_at_ms: record.created_at_ms,
                updated_at_ms: record.updated_at_ms,
            })
            .collect::<Vec<_>>();
        Ok(PageData {
            d: rows,
            t: raw.t,
            p: raw.p,
        })
    }
}

impl BatchComputedEvaluator {
    /// 创建集合化计算字段求值器。
    pub fn new(store: RecordStore) -> Self {
        Self { store }
    }

    /// 对一批 payload 执行 computed 字段求值。
    pub async fn evaluate(
        &self,
        model_name: &str,
        fields: &[MetaField],
        records: &mut [Value],
    ) -> anyhow::Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let computed_fields = fields
            .iter()
            .filter(|field| field.field_type == "computed")
            .collect::<Vec<_>>();
        if computed_fields.is_empty() {
            return Ok(());
        }
        let dependencies = self
            .load_dependency_cache(model_name, &computed_fields, records)
            .await?;
        for (record_index, record) in records.iter_mut().enumerate() {
            let payload = record
                .as_object_mut()
                .ok_or_else(|| anyhow!("计算字段只能处理 JSON 对象记录"))?;
            for field in &computed_fields {
                let Some(expression) = field.expression.as_deref() else {
                    continue;
                };
                let deps = match dependencies
                    .get(&field.id)
                    .and_then(|items| items.get(record_index))
                {
                    Some(value) => value.clone(),
                    None => HashMap::new(),
                };
                let value = evaluate_expression(expression, payload, &deps)
                    .with_context(|| format!("计算字段失败: {}.{}", model_name, field.name))?;
                payload.insert(field.name.clone(), eval_value_to_json(value));
            }
        }
        Ok(())
    }

    async fn load_dependency_cache(
        &self,
        _model_name: &str,
        computed_fields: &[&MetaField],
        records: &[Value],
    ) -> anyhow::Result<HashMap<String, Vec<HashMap<String, Value>>>> {
        let mut output = HashMap::new();
        for field in computed_fields {
            let deps = parse_dependencies(field.dependency_json.as_deref())?;
            if deps.is_empty() {
                continue;
            }
            let mut aliases_by_record = vec![HashMap::new(); records.len()];
            for dep in deps {
                let cache = self.load_one_dependency(&dep, records).await?;
                for (record_index, record) in records.iter().enumerate() {
                    let Some(object) = record.as_object() else {
                        continue;
                    };
                    let Some(record_id) = object.get(&dep.local_field).and_then(Value::as_str)
                    else {
                        continue;
                    };
                    if let Some(source_payload) = cache.get(record_id)
                        && let Some(value) = source_payload.get(&dep.source_payload_field)
                    {
                        aliases_by_record[record_index].insert(dep.alias.clone(), value.clone());
                    }
                }
            }
            output.insert(field.id.clone(), aliases_by_record);
        }
        Ok(output)
    }

    async fn load_one_dependency(
        &self,
        dependency: &ComputedDependency,
        records: &[Value],
    ) -> anyhow::Result<HashMap<String, Map<String, Value>>> {
        let ids = records
            .iter()
            .filter_map(Value::as_object)
            .filter_map(|object| object.get(&dependency.local_field))
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let wanted = ids.into_iter().collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records WHERE model_name = $1 AND id = ANY($2)",
        )
        .bind(&dependency.source_model_name)
        .bind(&wanted)
        .fetch_all(&self.store.pool)
        .await
        .with_context(|| {
            format!(
                "批量加载 computed 依赖失败: {}",
                dependency.source_model_name
            )
        })?;
        let mut cache = HashMap::new();
        for row in &rows {
            let row = data_record_from_row(row)?;
            let payload = value_to_object(row.payload.0)?;
            cache.insert(row.id, payload);
        }
        Ok(cache)
    }
}

/// 创建 API 成功响应。
pub fn ok_response<T: Serialize>(data: T) -> Value {
    json!({ "code": 200, "data": data })
}

/// 创建 API 错误响应。
pub fn error_response(code: u16, message: impl Into<String>) -> Value {
    json!({ "code": code, "msg": message.into() })
}

/// 返回动态数据运行时的 Toasty 模型集合。
pub fn engine_models() -> toasty::ModelSet {
    toasty::models!(MetaModel, MetaField, DataRecord)
}

/// 通过真实 Toasty 查询确认三张动态数据表都可读。
async fn verify_existing_schema(db: &toasty::Db, pool: &PgPool) -> anyhow::Result<()> {
    let mut db = db.clone();
    let mut models = Query::<List<MetaModel>>::all();
    models.limit(1);
    models
        .exec(&mut db)
        .await
        .context("校验 engine_meta_models 表失败")?;

    let mut fields = Query::<List<MetaField>>::all();
    fields.limit(1);
    fields
        .exec(&mut db)
        .await
        .context("校验 engine_meta_fields 表失败")?;

    sqlx::query_scalar::<_, SqlJson<Value>>("SELECT payload FROM engine_data_records LIMIT 1")
        .fetch_optional(pool)
        .await
        .context("校验 engine_data_records 表失败")?;

    Ok(())
}

fn data_record_from_row(row: &sqlx::postgres::PgRow) -> anyhow::Result<DataRecord> {
    let payload: SqlJson<Value> = row.try_get("payload")?;
    Ok(DataRecord {
        id: row.try_get("id")?,
        model_name: row.try_get("model_name")?,
        payload: toasty::Json(payload.0),
        created_at_ms: row.try_get("created_at_ms")?,
        updated_at_ms: row.try_get("updated_at_ms")?,
    })
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let item = item.trim().to_string();
        if item.is_empty() { None } else { Some(item) }
    })
}

fn validate_identifier(value: &str, label: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        bail!("{label}不能为空");
    }
    let valid = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !valid {
        bail!("{label}只能包含 ASCII 字母、数字和下划线: {value}");
    }
    Ok(())
}

fn validate_field_type(value: &str) -> anyhow::Result<()> {
    match normalize_field_type(value).as_str() {
        "string" | "int" | "decimal" | "boolean" | "datetime" | "json" | "computed" => Ok(()),
        _ => bail!("不支持的字段类型: {value}"),
    }
}

fn normalize_field_type(value: &str) -> String {
    match value {
        "String" => "string",
        "Integer" => "int",
        "Float" => "decimal",
        "Boolean" => "boolean",
        "DateTime" => "datetime",
        "Json" => "json",
        "Computed" => "computed",
        other => other,
    }
    .to_ascii_lowercase()
}

fn validate_dependency_json(value: Option<&str>) -> anyhow::Result<()> {
    let _ = parse_dependencies(value)?;
    Ok(())
}

fn validate_optional_json(value: Option<&str>, label: &str) -> anyhow::Result<()> {
    let Some(value) = value.map(str::trim).filter(|item| !item.is_empty()) else {
        return Ok(());
    };
    serde_json::from_str::<Value>(value).with_context(|| format!("解析{label}失败"))?;
    Ok(())
}

fn parse_dependencies(value: Option<&str>) -> anyhow::Result<Vec<ComputedDependency>> {
    let Some(value) = value.map(str::trim).filter(|item| !item.is_empty()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str(value).context("解析 computed dependency_json 失败")
}

fn validate_payload(
    fields: &[MetaField],
    payload: &Map<String, Value>,
    include_computed: bool,
) -> anyhow::Result<()> {
    for field in fields {
        if field.field_type == "computed" && !include_computed {
            continue;
        }
        if field.is_required && !payload.contains_key(&field.name) {
            bail!("缺少必填字段: {}", field.name);
        }
        if let Some(value) = payload.get(&field.name) {
            validate_json_type(field, value)?;
            validate_field_constraints(field, value)?;
        }
    }
    Ok(())
}

fn validate_field_constraints(field: &MetaField, value: &Value) -> anyhow::Result<()> {
    let Some(definition) = field
        .validation_json
        .as_deref()
        .map(str::trim)
        .filter(|item| !item.is_empty())
    else {
        return Ok(());
    };
    let definition = serde_json::from_str::<Value>(definition)
        .with_context(|| format!("解析字段 {} 的校验定义失败", field.name))?;
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if let Some(min_length) = definition.get("min_length").and_then(Value::as_u64)
            && length < min_length
        {
            bail!("字段 {} 长度不能小于 {min_length}", field.name);
        }
        if let Some(max_length) = definition.get("max_length").and_then(Value::as_u64)
            && length > max_length
        {
            bail!("字段 {} 长度不能大于 {max_length}", field.name);
        }
        if let Some(pattern) = definition.get("pattern").and_then(Value::as_str) {
            let pattern = Regex::new(pattern)
                .with_context(|| format!("字段 {} 的正则表达式无效", field.name))?;
            if !pattern.is_match(text) {
                bail!("字段 {} 不符合格式要求", field.name);
            }
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = definition.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            bail!("字段 {} 小于最小值 {minimum}", field.name);
        }
        if let Some(maximum) = definition.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            bail!("字段 {} 大于最大值 {maximum}", field.name);
        }
    }
    Ok(())
}

fn validate_json_type(field: &MetaField, value: &Value) -> anyhow::Result<()> {
    if value.is_null() {
        if field.is_required {
            bail!("字段不能为空: {}", field.name);
        }
        return Ok(());
    }
    if field.is_required && value.as_str().is_some_and(str::is_empty) {
        bail!("字段不能为空: {}", field.name);
    }
    let matched = match field.field_type.as_str() {
        "string" => value.is_string(),
        "int" | "datetime" => value.is_i64() || value.is_u64(),
        "decimal" => value.is_number(),
        "boolean" => value.is_boolean(),
        "json" | "computed" => true,
        _ => false,
    };
    if !matched {
        bail!("字段类型不匹配: {}", field.name);
    }
    Ok(())
}

fn value_to_object(value: Value) -> anyhow::Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("payload 必须是 JSON 对象"))
}

fn evaluate_expression(
    expression: &str,
    payload: &Map<String, Value>,
    dependencies: &HashMap<String, Value>,
) -> anyhow::Result<EvalValue> {
    let mut context = HashMapContext::new();
    for (key, value) in payload {
        set_eval_value(&mut context, key, value)?;
    }
    for (key, value) in dependencies {
        set_eval_value(&mut context, key, value)?;
    }
    evalexpr::eval_with_context_mut(expression, &mut context)
        .map_err(|error| anyhow!("表达式求值失败: {error}"))
}

fn set_eval_value(context: &mut HashMapContext, key: &str, value: &Value) -> anyhow::Result<()> {
    let eval_value = match value {
        Value::Bool(value) => EvalValue::Boolean(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                EvalValue::Int(value)
            } else if let Some(value) = value.as_f64() {
                EvalValue::Float(value)
            } else {
                return Ok(());
            }
        }
        Value::String(value) => EvalValue::String(value.clone()),
        _ => return Ok(()),
    };
    context
        .set_value(key.to_string(), eval_value)
        .map_err(|error| anyhow!("注入表达式变量失败: {key}: {error}"))
}

fn eval_value_to_json(value: EvalValue) -> Value {
    match value {
        EvalValue::String(value) => Value::String(value),
        EvalValue::Float(value) => match serde_json::Number::from_f64(value) {
            Some(number) => Value::Number(number),
            None => Value::Null,
        },
        EvalValue::Int(value) => Value::Number(value.into()),
        EvalValue::Boolean(value) => Value::Bool(value),
        EvalValue::Tuple(values) => {
            Value::Array(values.into_iter().map(eval_value_to_json).collect())
        }
        EvalValue::Empty => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn computed_field(name: &str, expression: &str) -> MetaField {
        MetaField {
            id: name.to_string(),
            model_name: "order".to_string(),
            name: name.to_string(),
            display_name: name.to_string(),
            field_type: "computed".to_string(),
            is_required: false,
            expression: Some(expression.to_string()),
            dependency_json: None,
            domain_metadata_json: None,
            validation_json: None,
            order_index: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    #[test]
    fn validates_required_fields() {
        let field = MetaField {
            id: "amount".to_string(),
            model_name: "order".to_string(),
            name: "amount".to_string(),
            display_name: "金额".to_string(),
            field_type: "int".to_string(),
            is_required: true,
            expression: None,
            dependency_json: None,
            domain_metadata_json: None,
            validation_json: None,
            order_index: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        let payload = Map::new();

        // 必填字段缺失时应在落库前阻断。
        assert!(validate_payload(&[field], &payload, false).is_err());
    }

    #[test]
    fn validates_string_length_and_pattern() {
        let field = MetaField {
            id: "code".to_string(),
            model_name: "asset".to_string(),
            name: "code".to_string(),
            display_name: "编码".to_string(),
            field_type: "string".to_string(),
            is_required: true,
            expression: None,
            dependency_json: None,
            domain_metadata_json: None,
            validation_json: Some(
                json!({"min_length": 3, "max_length": 8, "pattern": "^[A-Z]+$"}).to_string(),
            ),
            order_index: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        // 长度和格式规则必须与数值上下界一样在统一写入管线生效。
        assert!(validate_field_constraints(&field, &json!("ABC")).is_ok());
        assert!(validate_field_constraints(&field, &json!("ab")).is_err());
        assert!(validate_field_constraints(&field, &json!("abcdefghi")).is_err());
    }

    #[test]
    fn validates_postgresql_database_url_with_built_ins() {
        let url = match verify_database_url("  postgresql://engine  ") {
            Ok(url) => url,
            Err(error) => panic!("PostgreSQL DATABASE_URL 应通过校验: {error}"),
        };

        // 校验链应返回去除首尾空白后的正式连接串。
        assert_eq!(url, "postgresql://engine");
        // 非 PostgreSQL 协议必须被 starts_with_any 内置阻断。
        assert!(verify_database_url("mysql://engine").is_err());
    }

    #[test]
    fn evaluates_row_computed_expression() {
        let mut payload = Map::new();
        payload.insert("amount".to_string(), json!(100));
        let value = match evaluate_expression("amount * 2", &payload, &HashMap::new()) {
            Ok(value) => value,
            Err(error) => panic!("表达式应求值成功: {error}"),
        };

        // 本行字段应直接注入表达式上下文。
        assert_eq!(eval_value_to_json(value), json!(200));
    }

    #[test]
    fn parses_explicit_dependencies() {
        let dependencies = match parse_dependencies(Some(
            r#"[{"alias":"user_vip_level","source_model_name":"user","local_field":"user_id","source_payload_field":"vip_level"}]"#,
        )) {
            Ok(value) => value,
            Err(error) => panic!("依赖配置应解析成功: {error}"),
        };

        // 跨模型计算依赖必须显式配置，避免运行时猜表达式变量。
        assert_eq!(dependencies[0].alias, "user_vip_level");
    }

    #[test]
    fn computed_field_helper_uses_new_field_type() {
        let field = computed_field("bonus", "amount * 2");

        // computed 是新引擎字段类型，不再使用旧渲染配置。
        assert_eq!(field.field_type, "computed");
    }
}
