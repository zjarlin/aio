//! 声明式页面的 PostgreSQL 持久化边界。

use anyhow::{Context, Result, bail, ensure};
use az_remote_ui::{PAGE_SCHEMA_VERSION, PageDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toasty::stmt::{List, Query};

use crate::{EngineStore, PageData, PageParams, timestamp_ms};

/// 页面集合 REST API 路径。
pub const PAGES_PATH: &str = "/api/engine/pages";

/// 页面详情 REST API 路径模板。
pub const PAGE_PATH_TEMPLATE: &str = "/api/engine/pages/{page_key}";

pub const OP_PAGES_LIST: &str = "engine.pages.list";
pub const OP_PAGES_CREATE: &str = "engine.pages.create";
pub const OP_PAGES_GET: &str = "engine.pages.get";
pub const OP_PAGES_UPDATE: &str = "engine.pages.update";
pub const OP_PAGES_DELETE: &str = "engine.pages.delete";

/// 正式页面记录。
#[derive(Clone, Debug, PartialEq, toasty::Model)]
#[table = "engine_page_definitions"]
pub struct PageRecord {
    #[key]
    pub id: String,
    #[unique]
    pub page_key: String,
    #[unique]
    pub route: String,
    pub state: String,
    pub definition: toasty::Json<Value>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// 页面发布状态。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageState {
    #[default]
    Draft,
    Published,
    Disabled,
}

impl PageState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for PageState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "disabled" => Ok(Self::Disabled),
            _ => bail!("未知页面状态: {value}"),
        }
    }
}

/// 创建或替换页面的输入契约。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageInput {
    pub route: String,
    #[serde(default)]
    pub state: PageState,
    pub definition: PageDefinition,
}

/// 对外返回的页面视图。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageView {
    pub id: String,
    pub page_key: String,
    pub route: String,
    pub state: PageState,
    pub definition: PageDefinition,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl TryFrom<PageRecord> for PageView {
    type Error = anyhow::Error;

    fn try_from(record: PageRecord) -> Result<Self> {
        let definition = serde_json::from_value(record.definition.0)
            .context("反序列化 engine 页面定义失败")?;
        Ok(Self {
            id: record.id,
            page_key: record.page_key,
            route: record.route,
            state: PageState::try_from(record.state.as_str())?,
            definition,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        })
    }
}

impl EngineStore {
    /// 创建一条正式页面定义。
    pub async fn create_page(&self, input: PageInput) -> Result<PageView> {
        validate_page_input(&input)?;
        if self.get_page(&input.definition.key).await?.is_some() {
            bail!("页面已存在: {}", input.definition.key);
        }

        let now = timestamp_ms();
        let definition = serde_json::to_value(&input.definition).context("序列化页面定义失败")?;
        let mut db = self.db.lock().await;
        let record = PageRecord::create()
            .id(uuid::Uuid::new_v4().to_string())
            .page_key(&input.definition.key)
            .route(&input.route)
            .state(input.state.as_str().to_string())
            .definition(definition)
            .created_at_ms(now)
            .updated_at_ms(now)
            .exec(&mut *db)
            .await
            .context("创建 engine 页面失败")?;
        PageView::try_from(record)
    }

    /// 查询页面列表。
    pub async fn list_pages(&self, page: PageParams) -> Result<PageData<PageView>> {
        let mut db = self.db.lock().await;
        let total = Query::<List<PageRecord>>::all()
            .count()
            .exec(&mut *db)
            .await
            .context("统计 engine 页面失败")?;
        let mut query = Query::<List<PageRecord>>::all();
        query.limit(page.s);
        query.offset(page.o);
        let records = query.exec(&mut *db).await.context("查询 engine 页面失败")?;
        let rows = records
            .into_iter()
            .map(PageView::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(PageData {
            d: rows,
            t: total,
            p: page,
        })
    }

    /// 按稳定 page key 查询页面。
    pub async fn get_page(&self, page_key: &str) -> Result<Option<PageView>> {
        let mut db = self.db.lock().await;
        let record = Query::<List<PageRecord>>::filter(PageRecord::fields().page_key().eq(page_key))
            .first()
            .exec(&mut *db)
            .await
            .context("查询 engine 页面失败")?;
        record.map(PageView::try_from).transpose()
    }

    /// 整体替换页面定义，page key 不允许通过更新改名。
    pub async fn update_page(&self, page_key: &str, input: PageInput) -> Result<PageView> {
        validate_page_input(&input)?;
        ensure!(
            input.definition.key == page_key,
            "页面 key 不支持通过 update 改名: {page_key} -> {}",
            input.definition.key
        );
        ensure!(self.get_page(page_key).await?.is_some(), "页面不存在: {page_key}");

        let definition = serde_json::to_value(&input.definition).context("序列化页面定义失败")?;
        let now = timestamp_ms();
        {
            let mut db = self.db.lock().await;
            PageRecord::filter(PageRecord::fields().page_key().eq(page_key))
                .update()
                .route(&input.route)
                .state(input.state.as_str().to_string())
                .definition(definition)
                .updated_at_ms(now)
                .exec(&mut *db)
                .await
                .context("更新 engine 页面失败")?;
        }
        self.get_page(page_key)
            .await?
            .with_context(|| format!("读取更新后的 engine 页面失败: {page_key}"))
    }

    /// 删除页面定义。
    pub async fn delete_page(&self, page_key: &str) -> Result<()> {
        let mut db = self.db.lock().await;
        PageRecord::filter(PageRecord::fields().page_key().eq(page_key))
            .delete()
            .exec(&mut *db)
            .await
            .context("删除 engine 页面失败")?;
        Ok(())
    }
}

fn validate_page_input(input: &PageInput) -> Result<()> {
    validate_page_key(&input.definition.key)?;
    ensure!(
        input.definition.schema_version == PAGE_SCHEMA_VERSION,
        "不支持的页面 schema_version: {}",
        input.definition.schema_version
    );
    ensure!(!input.definition.title.trim().is_empty(), "页面 title 不能为空");
    ensure!(input.route.starts_with('/'), "页面 route 必须以 / 开始");
    ensure!(input.route.len() > 1, "页面 route 不能只包含 /");
    Ok(())
}

fn validate_page_key(value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "页面 key 不能为空");
    let valid = value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    ensure!(valid, "页面 key 只能包含 ASCII 字母、数字、连字符和下划线");
    Ok(())
}

/// 通过真实查询确认页面表可用。
pub(crate) async fn verify_page_schema(db: &mut toasty::Db) -> Result<()> {
    let mut pages = Query::<List<PageRecord>>::all();
    pages.limit(1);
    pages
        .exec(&mut *db)
        .await
        .context("校验 engine_page_definitions 表失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use az_remote_ui::{ComponentNode, PropertyValue};

    use super::*;

    #[test]
    fn validates_page_identity_and_schema_version() {
        let input = PageInput {
            route: "/remote-ui/device-overview".to_string(),
            state: PageState::Published,
            definition: PageDefinition {
                schema_version: PAGE_SCHEMA_VERSION,
                key: "device-overview".to_string(),
                title: "设备概览".to_string(),
                root: ComponentNode {
                    component: "az_remote_ui::components::section".to_string(),
                    id: None,
                    properties: BTreeMap::new(),
                    content: Some(PropertyValue::text("invalid container content")),
                    children: Vec::new(),
                },
                data_sources: Vec::new(),
                actions: Vec::new(),
            },
        };

        // Store 只验证持久化身份；组件树能力由当前 Rudi catalog 编译时校验。
        assert!(validate_page_input(&input).is_ok());
    }
}
