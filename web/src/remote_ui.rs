use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Extension, Json,
    extract::{Path, Query},
    http::StatusCode,
    response::{
        Html,
        sse::{Event, KeepAlive, Sse},
    },
};
use az_aio_nature_generated::enums::PageState;
use az_engine::{EngineStore, operation::OperationRequestContext, page::PageInput};
use az_remote_ui::{ComponentIndex, PageCompiler, PageDefinition, PropertyValue, UiOp};
use futures_util::{Stream, stream};
use serde::Deserialize;
use serde_json::Value;

type WebResult<T> = Result<T, (StatusCode, String)>;

const DEFAULT_PAGE_KEY: &str = "device-overview";
const DEFAULT_PAGE_ROUTE: &str = "/remote-ui?page=device-overview";
const DEFAULT_PAGE_JSON: &str = include_str!("../assets/device-overview.page.json");

#[derive(Clone)]
pub struct RemoteUiRuntime {
    components: Arc<ComponentIndex>,
    store: Option<EngineStore>,
}

impl RemoteUiRuntime {
    #[must_use]
    pub fn new(components: Arc<ComponentIndex>, store: Option<EngineStore>) -> Self {
        Self { components, store }
    }

    async fn load_page(&self, page_key: &str) -> Result<LoadedPage> {
        if let Some(store) = &self.store {
            if let Some(page) = store.get_page(page_key).await? {
                return Ok(LoadedPage {
                    definition: page.definition,
                    source: "PostgreSQL",
                });
            }
            if page_key == DEFAULT_PAGE_KEY {
                let definition = default_page_definition()?;
                let page = store
                    .create_page(PageInput {
                        route: DEFAULT_PAGE_ROUTE.to_string(),
                        state: PageState::Published,
                        definition,
                    })
                    .await
                    .context("初始化 Remote UI 演示页面失败")?;
                return Ok(LoadedPage {
                    definition: page.definition,
                    source: "PostgreSQL",
                });
            }
            anyhow::bail!("页面不存在: {page_key}");
        }

        anyhow::ensure!(
            page_key == DEFAULT_PAGE_KEY,
            "未配置 PostgreSQL 时只提供开发态演示页面"
        );
        Ok(LoadedPage {
            definition: default_page_definition()?,
            source: "开发态内置定义",
        })
    }

    async fn load_data(&self, page: &PageDefinition) -> Result<Value> {
        let Some(store) = &self.store else {
            return Ok(Value::Object(Default::default()));
        };
        let mut data = serde_json::Map::new();
        for source in &page.data_sources {
            let operation = store
                .get_operation(&source.operation)
                .await?
                .with_context(|| format!("页面数据源 operation 不存在: {}", source.operation))?;
            let query = source
                .parameters
                .iter()
                .map(|(name, value)| {
                    Ok((
                        name.clone(),
                        vec![property_value(value, &Value::Null)?.to_string()],
                    ))
                })
                .collect::<Result<_>>()?;
            let invocation = store
                .invoke_operation(OperationRequestContext {
                    operation_key: source.operation.clone(),
                    method: operation.method,
                    path: Default::default(),
                    query,
                    body: Value::Null,
                })
                .await
                .with_context(|| format!("加载页面数据源失败: {}", source.id))?;
            data.insert(source.id.clone(), invocation.data);
        }
        Ok(Value::Object(data))
    }
}

struct LoadedPage {
    definition: PageDefinition,
    source: &'static str,
}

#[derive(Debug, Default, Deserialize)]
pub struct PageQuery {
    page: Option<String>,
}

pub async fn page(
    Extension(runtime): Extension<RemoteUiRuntime>,
    Query(query): Query<PageQuery>,
) -> WebResult<Html<String>> {
    let page_key = query.page.as_deref().unwrap_or(DEFAULT_PAGE_KEY);
    let loaded = runtime.load_page(page_key).await.map_err(internal_error)?;
    let data = runtime
        .load_data(&loaded.definition)
        .await
        .map_err(internal_error)?;
    PageCompiler::new(&runtime.components)
        .compile(&loaded.definition, &data)
        .context("校验页面定义失败")
        .map_err(internal_error)?;

    let catalog = runtime.components.browser_catalog();
    let catalog_json = serde_json::to_string(&catalog).map_err(internal_error)?;
    let catalog_json = escape_script_json(&catalog_json);
    let title = escape_html(&loaded.definition.title);
    let page_key = escape_html(&loaded.definition.key);
    let source = escape_html(loaded.source);
    let stream_url = format!("/api/remote-ui/pages/{page_key}/stream");
    let data_url = format!("/api/remote-ui/pages/{page_key}/data");
    let action_url = format!("/api/remote-ui/pages/{page_key}/actions");
    let actions_json = escape_script_json(
        &serde_json::to_string(&loaded.definition.actions).map_err(internal_error)?,
    );
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title} · Remote UI</title>
  <link rel="stylesheet" href="/assets/app.css?v=remote-ui-v3">
  <link rel="stylesheet" href="/assets/remote-ui.css?v=remote-ui-v3">
</head>
<body class="remote-ui-body">
  <header class="remote-ui-topbar border-b bg-background">
    <div class="min-w-0">
      <h1 class="remote-ui-brand">{title}</h1>
      <p class="remote-ui-context text-xs text-muted-foreground">{page_key} · {source}</p>
    </div>
    <div class="remote-ui-toolbar">
      <span id="remote-ui-status" class="remote-ui-status remote-ui-status--idle">待连接</span>
      <button id="remote-ui-replay" class="remote-ui-replay" type="button">重新播放</button>
    </div>
  </header>
  <main class="remote-ui-workbench">
    <section class="remote-ui-stage" aria-label="Remote UI preview">
      <div id="remote-ui-root" class="remote-ui-root" data-stream-url="{stream_url}" data-data-url="{data_url}" data-action-url="{action_url}"></div>
    </section>
    <aside class="remote-ui-events" aria-label="Component events">
      <header class="remote-ui-events-header">
        <h2>组件事件</h2>
        <span id="remote-ui-event-count" class="remote-ui-event-count">0</span>
      </header>
      <ol id="remote-ui-event-log" class="remote-ui-event-log"></ol>
    </aside>
  </main>
  <script id="remote-ui-catalog" type="application/json">{catalog_json}</script>
  <script id="remote-ui-actions" type="application/json">{actions_json}</script>
  <script type="module" src="/assets/remote-ui.js?v=remote-ui-v3"></script>
</body>
</html>"#
    );
    Ok(Html(html))
}

pub async fn page_stream(
    Extension(runtime): Extension<RemoteUiRuntime>,
    Path(page_key): Path<String>,
) -> WebResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let loaded = runtime.load_page(&page_key).await.map_err(internal_error)?;
    let data = runtime
        .load_data(&loaded.definition)
        .await
        .map_err(internal_error)?;
    let operations = PageCompiler::new(&runtime.components)
        .compile(&loaded.definition, &data)
        .context("编译声明式页面失败")
        .map_err(internal_error)?;
    let mut events = operations
        .into_iter()
        .map(serialize_operation)
        .collect::<Result<VecDeque<_>, _>>()
        .map_err(internal_error)?;
    events.push_back(("done".to_string(), "{}".to_string()));

    let event_stream = stream::unfold(events, |mut events| async move {
        let (event_name, payload) = events.pop_front()?;
        tokio::time::sleep(Duration::from_millis(40)).await;
        let event = Event::default().event(event_name).data(payload);
        Some((Ok(event), events))
    });

    Ok(Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    ))
}

pub async fn page_data(
    Extension(runtime): Extension<RemoteUiRuntime>,
    Path(page_key): Path<String>,
) -> WebResult<Json<Value>> {
    let loaded = runtime.load_page(&page_key).await.map_err(internal_error)?;
    runtime
        .load_data(&loaded.definition)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn invoke_action(
    Extension(runtime): Extension<RemoteUiRuntime>,
    Path((page_key, action_id)): Path<(String, String)>,
    Json(input): Json<Value>,
) -> WebResult<Json<Value>> {
    let store = runtime
        .store
        .as_ref()
        .ok_or_else(|| internal_error("未配置 PostgreSQL，不能执行页面动作"))?;
    let loaded = runtime.load_page(&page_key).await.map_err(internal_error)?;
    let action = loaded
        .definition
        .actions
        .iter()
        .find(|action| action.id == action_id)
        .with_context(|| format!("页面动作不存在: {action_id}"))
        .map_err(internal_error)?;
    let operation = store
        .get_operation(&action.operation)
        .await
        .map_err(internal_error)?
        .with_context(|| format!("页面动作 operation 不存在: {}", action.operation))
        .map_err(internal_error)?;
    let mut body = input.as_object().cloned().unwrap_or_default();
    for (name, value) in &action.input {
        body.insert(
            name.clone(),
            property_value(value, &input).map_err(internal_error)?,
        );
    }
    let invocation = store
        .invoke_operation(OperationRequestContext {
            operation_key: action.operation.clone(),
            method: operation.method,
            path: Default::default(),
            query: Default::default(),
            body: Value::Object(body),
        })
        .await
        .map_err(internal_error)?;
    Ok(Json(invocation.data))
}

/// 低代码编辑器读取的 Rudi 组件能力目录。
pub async fn component_catalog(
    Extension(runtime): Extension<RemoteUiRuntime>,
) -> Json<std::collections::BTreeMap<String, az_remote_ui::ComponentCatalogEntry>> {
    Json(runtime.components.browser_catalog())
}

fn default_page_definition() -> Result<PageDefinition> {
    serde_json::from_str(DEFAULT_PAGE_JSON).context("解析开发态 Remote UI 页面定义失败")
}

fn serialize_operation(operation: UiOp) -> serde_json::Result<(String, String)> {
    let payload = serde_json::to_string(&operation)?;
    Ok(("message".to_string(), payload))
}

fn property_value(property: &PropertyValue, data: &Value) -> Result<Value> {
    match property {
        PropertyValue::Literal { value } => Ok(value.clone()),
        PropertyValue::Binding { path } => {
            let pointer = format!("/{}", path.replace('.', "/"));
            data.pointer(&pointer)
                .cloned()
                .with_context(|| format!("数据绑定不存在: {path}"))
        }
    }
}

fn escape_script_json(value: &str) -> String {
    value
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Remote UI 生成失败: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use rudi::Context as RudiContext;

    use super::*;

    #[test]
    fn bundled_page_is_valid_declarative_configuration() -> Result<()> {
        let mut context = RudiContext::auto_register();
        let components = ComponentIndex::from_context(&mut context)?;
        let page = default_page_definition()?;
        let operations = PageCompiler::new(&components).compile(&page, &Value::Null)?;

        // 演示页面必须完全由声明式配置编译，不再依赖 DSL 字符串常量。
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, UiOp::Open { .. }))
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, UiOp::Leaf { .. }))
        );
        assert_eq!(page.key, DEFAULT_PAGE_KEY);
        Ok(())
    }
}
