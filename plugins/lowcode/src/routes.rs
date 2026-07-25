//! engine REST API 与 SSR action 路由。

use std::collections::BTreeMap;

use anyhow::{Context, anyhow, bail};
use axum::{
    Router,
    body::Bytes,
    extract::{RawQuery, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use az_aio_platform::core::api_error::{ApiError, ApiForm, ApiJson, ApiPath, parse_usize_param};
use az_engine::operation::{
    OperationDraft, OperationRequestContext, OperationRevisionInput, OperationTestInput,
};
use az_engine::page::PageInput;
use az_engine::{EngineStore, FieldInput, HookInput, ModelInput, PageParams};
use serde_json::{Map, Value, json};

#[derive(Clone)]
pub struct LowcodeApiState {
    pub store: EngineStore,
}

/// 构建 lowcode 插件的新 engine API 路由。
pub fn engine_router(state: LowcodeApiState) -> Router {
    Router::new()
        .route("/api/engine/models", get(list_models).post(create_model))
        .route(
            "/api/engine/models/{model_name}",
            get(get_model).put(update_model).delete(delete_model),
        )
        .route(
            "/api/engine/models/{model_name}/fields",
            get(list_fields).post(create_field),
        )
        .route(
            "/api/engine/models/{model_name}/fields/{field_id}",
            get(get_field).put(update_field).delete(delete_field),
        )
        .route(
            "/api/engine/models/{model_name}/hooks",
            get(list_hooks).post(create_hook),
        )
        .route(
            "/api/engine/models/{model_name}/hooks/{hook_id}",
            get(get_hook).put(update_hook).delete(delete_hook),
        )
        .route(
            "/api/engine/models/{model_name}/records",
            get(list_records).post(insert_record),
        )
        .route(
            "/api/engine/models/{model_name}/records/{record_id}",
            get(get_record).put(update_record).delete(delete_record),
        )
        .route(
            "/api/engine/operations",
            get(list_operations).post(create_operation),
        )
        .route("/api/engine/pages", get(list_pages).post(create_page))
        .route(
            "/api/engine/pages/{page_key}",
            get(get_page).put(update_page).delete(delete_page),
        )
        .route("/api/engine/operations/{operation_key}", get(get_operation))
        .route(
            "/api/engine/operations/{operation_key}/revisions",
            get(list_operation_revisions).post(create_operation_revision),
        )
        .route(
            "/api/engine/operations/{operation_key}/revisions/{revision_id}/publish",
            post(publish_operation),
        )
        .route(
            "/api/engine/operations/{operation_key}/revisions/{revision_id}/test",
            post(test_operation_revision),
        )
        .route(
            "/api/engine/operations/{operation_key}/disable",
            post(disable_operation),
        )
        .route(
            "/api/engine/invoke/{operation_key}",
            get(invoke_operation).post(invoke_operation),
        )
        .route("/api/engine/ui-action", post(ui_action))
        .with_state(state)
}

async fn list_pages(
    State(state): State<LowcodeApiState>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let page = match page_query_from_raw(raw_query.as_deref()) {
        Ok(page) => page,
        Err(error) => return ApiError::from(error).into_response(),
    };
    into_api_response(state.store.list_pages(page).await)
}

async fn create_page(
    State(state): State<LowcodeApiState>,
    ApiJson(input): ApiJson<PageInput>,
) -> Response {
    into_bad_request_response(state.store.create_page(input).await)
}

async fn get_page(
    State(state): State<LowcodeApiState>,
    ApiPath(page_key): ApiPath<String>,
) -> Response {
    into_api_response(state.store.get_page(&page_key).await)
}

async fn update_page(
    State(state): State<LowcodeApiState>,
    ApiPath(page_key): ApiPath<String>,
    ApiJson(input): ApiJson<PageInput>,
) -> Response {
    into_bad_request_response(state.store.update_page(&page_key, input).await)
}

async fn delete_page(
    State(state): State<LowcodeApiState>,
    ApiPath(page_key): ApiPath<String>,
) -> Response {
    into_api_response(
        async {
            state.store.delete_page(&page_key).await?;
            Ok(json!({ "deleted": true }))
        }
        .await,
    )
}

async fn list_operations(
    State(state): State<LowcodeApiState>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let page = match page_query_from_raw(raw_query.as_deref()) {
        Ok(page) => page,
        Err(error) => return ApiError::from(error).into_response(),
    };
    into_api_response(state.store.list_operations(page).await)
}

async fn create_operation(
    State(state): State<LowcodeApiState>,
    ApiJson(input): ApiJson<OperationDraft>,
) -> Response {
    into_bad_request_response(state.store.create_operation(input).await)
}

async fn get_operation(
    State(state): State<LowcodeApiState>,
    ApiPath(operation_key): ApiPath<String>,
) -> Response {
    into_api_response(state.store.get_operation(&operation_key).await)
}

async fn list_operation_revisions(
    State(state): State<LowcodeApiState>,
    ApiPath(operation_key): ApiPath<String>,
) -> Response {
    into_api_response(state.store.list_operation_revisions(&operation_key).await)
}

async fn create_operation_revision(
    State(state): State<LowcodeApiState>,
    ApiPath(operation_key): ApiPath<String>,
    ApiJson(input): ApiJson<OperationRevisionInput>,
) -> Response {
    into_bad_request_response(
        state
            .store
            .create_operation_revision(&operation_key, input)
            .await,
    )
}

async fn publish_operation(
    State(state): State<LowcodeApiState>,
    ApiPath((operation_key, revision_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(
        state
            .store
            .publish_operation(&operation_key, &revision_id)
            .await,
    )
}

async fn disable_operation(
    State(state): State<LowcodeApiState>,
    ApiPath(operation_key): ApiPath<String>,
) -> Response {
    into_api_response(state.store.disable_operation(&operation_key).await)
}

async fn test_operation_revision(
    State(state): State<LowcodeApiState>,
    ApiPath((operation_key, revision_id)): ApiPath<(String, String)>,
    ApiJson(input): ApiJson<OperationTestInput>,
) -> Response {
    into_bad_request_response(
        state
            .store
            .test_operation_revision(&operation_key, &revision_id, input)
            .await,
    )
}

async fn invoke_operation(
    State(state): State<LowcodeApiState>,
    method: Method,
    ApiPath(operation_key): ApiPath<String>,
    RawQuery(raw_query): RawQuery,
    body: Bytes,
) -> Response {
    let body = match operation_body(&body) {
        Ok(body) => body,
        Err(error) => return ApiError::bad_request(error.to_string()).into_response(),
    };
    let query = match operation_query_from_raw(raw_query.as_deref()) {
        Ok(query) => query,
        Err(error) => return ApiError::bad_request(error.to_string()).into_response(),
    };
    let path = BTreeMap::from([("operation_key".to_string(), operation_key.clone())]);
    let context = OperationRequestContext {
        operation_key,
        method: method.as_str().to_string(),
        path,
        query,
        body,
    };
    match state.store.invoke_operation(context).await {
        Ok(data) => az_aio_platform::core::api_error::ok_json(data).into_response(),
        Err(error) => {
            let message = error.to_string();
            if message.contains("尚未发布") || message.contains("没有活动 revision") {
                return ApiError::new(StatusCode::CONFLICT, message).into_response();
            }
            ApiError::from_anyhow(error).into_response()
        }
    }
}

fn operation_body(body: &[u8]) -> anyhow::Result<Value> {
    if body.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(body).context("动态 operation body 需要合法 JSON")
}

fn operation_query_from_raw(
    raw_query: Option<&str>,
) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let mut query = BTreeMap::<String, Vec<String>>::new();
    let raw_query = raw_query.unwrap_or_default();
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some(values) => values,
            None => (pair, ""),
        };
        let key = urlencoding::decode(raw_key)
            .with_context(|| format!("operation query 参数名解码失败: {raw_key}"))?
            .into_owned();
        let value = urlencoding::decode(raw_value)
            .with_context(|| format!("operation query 参数值解码失败: {raw_key}"))?
            .into_owned();
        query.entry(key).or_default().push(value);
    }
    Ok(query)
}

async fn list_models(
    State(state): State<LowcodeApiState>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let page = match page_query_from_raw(raw_query.as_deref()) {
        Ok(page) => page,
        Err(error) => return ApiError::from(error).into_response(),
    };
    into_api_response(state.store.list_models(page).await)
}

async fn create_model(
    State(state): State<LowcodeApiState>,
    ApiJson(input): ApiJson<ModelInput>,
) -> Response {
    into_api_response(state.store.create_model(input).await)
}

async fn get_model(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
) -> Response {
    into_api_response(state.store.get_model(&model_name).await)
}

async fn delete_model(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
) -> Response {
    into_api_response(
        async {
            state.store.delete_model(&model_name).await?;
            Ok(json!({ "deleted": true }))
        }
        .await,
    )
}

async fn update_model(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
    ApiJson(input): ApiJson<ModelInput>,
) -> Response {
    into_api_response(state.store.update_model(&model_name, input).await)
}

async fn list_fields(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
) -> Response {
    into_api_response(state.store.list_fields(&model_name).await)
}

async fn create_field(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
    ApiJson(input): ApiJson<FieldInput>,
) -> Response {
    into_api_response(state.store.create_field(&model_name, input).await)
}

async fn delete_field(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, field_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(
        async {
            state.store.delete_field(&field_id).await?;
            Ok(json!({ "deleted": true }))
        }
        .await,
    )
}

async fn get_field(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, field_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(state.store.get_field(&field_id).await)
}

async fn update_field(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, field_id)): ApiPath<(String, String)>,
    ApiJson(input): ApiJson<FieldInput>,
) -> Response {
    into_api_response(state.store.update_field(&field_id, input).await)
}

async fn list_hooks(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
) -> Response {
    into_api_response(state.store.list_hooks(&model_name).await)
}

async fn create_hook(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
    ApiJson(input): ApiJson<HookInput>,
) -> Response {
    into_api_response(state.store.create_hook(&model_name, input).await)
}

async fn delete_hook(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, hook_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(
        async {
            state.store.delete_hook(&hook_id).await?;
            Ok(json!({ "deleted": true }))
        }
        .await,
    )
}

async fn get_hook(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, hook_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(state.store.get_hook(&hook_id).await)
}

async fn update_hook(
    State(state): State<LowcodeApiState>,
    ApiPath((_model_name, hook_id)): ApiPath<(String, String)>,
    ApiJson(input): ApiJson<HookInput>,
) -> Response {
    into_api_response(state.store.update_hook(&hook_id, input).await)
}

async fn list_records(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let page = match page_query_from_raw(raw_query.as_deref()) {
        Ok(page) => page,
        Err(error) => return ApiError::from(error).into_response(),
    };
    into_api_response(state.store.executor().list_records(&model_name, page).await)
}

async fn insert_record(
    State(state): State<LowcodeApiState>,
    ApiPath(model_name): ApiPath<String>,
    ApiJson(payload): ApiJson<Value>,
) -> Response {
    into_api_response(
        state
            .store
            .executor()
            .insert_record(&model_name, payload)
            .await,
    )
}

async fn get_record(
    State(state): State<LowcodeApiState>,
    ApiPath((model_name, record_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(
        state
            .store
            .executor()
            .get_record(&model_name, &record_id)
            .await,
    )
}

async fn update_record(
    State(state): State<LowcodeApiState>,
    ApiPath((model_name, record_id)): ApiPath<(String, String)>,
    ApiJson(payload): ApiJson<Value>,
) -> Response {
    into_api_response(
        state
            .store
            .executor()
            .update_record(&model_name, &record_id, payload)
            .await,
    )
}

async fn delete_record(
    State(state): State<LowcodeApiState>,
    ApiPath((model_name, record_id)): ApiPath<(String, String)>,
) -> Response {
    into_api_response(
        async {
            state
                .store
                .executor()
                .delete_record(&model_name, &record_id)
                .await?;
            Ok(json!({ "deleted": true }))
        }
        .await,
    )
}

async fn ui_action(
    State(state): State<LowcodeApiState>,
    ApiForm(form): ApiForm<BTreeMap<String, String>>,
) -> Response {
    let operation_action = form
        .get("action")
        .is_some_and(|action| action.contains("operation"));
    let redirect = match apply_ui_action(&state, form).await {
        Ok(route) => route,
        Err(error) => {
            let route = if operation_action {
                format!(
                    "/lowcode?tab=operations&error={}",
                    urlencoding::encode(&error.to_string())
                )
            } else {
                format!("/lowcode?error={}", urlencoding::encode(&error.to_string()))
            };
            format!("/?route={}", urlencoding::encode(&route))
        }
    };
    Redirect::to(&redirect).into_response()
}

async fn apply_ui_action(
    state: &LowcodeApiState,
    form: BTreeMap<String, String>,
) -> anyhow::Result<String> {
    let store = &state.store;
    let action = form_value(&form, "action");
    match action.as_deref() {
        Some("create_model") => {
            let name = required_form_value(&form, "name")?;
            let display_name = match form_value(&form, "display_name") {
                Some(value) => value,
                None => name.clone(),
            };
            store
                .create_model(ModelInput {
                    name: name.clone(),
                    display_name,
                })
                .await?;
            Ok(lowcode_route(Some(&name), "fields"))
        }
        Some("delete_model") => {
            let model_name = required_form_value(&form, "model_name")?;
            store.delete_model(&model_name).await?;
            Ok(lowcode_route(None, "fields"))
        }
        Some("update_model") => {
            let model_name = required_form_value(&form, "model_name")?;
            let name = match form_value(&form, "name") {
                Some(value) => value,
                None => model_name.clone(),
            };
            store
                .update_model(
                    &model_name,
                    ModelInput {
                        name,
                        display_name: required_form_value(&form, "display_name")?,
                    },
                )
                .await?;
            Ok(lowcode_route(Some(&model_name), "fields"))
        }
        Some("create_field") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .create_field(&model_name, field_input_from_form(&form)?)
                .await?;
            Ok(lowcode_route(Some(&model_name), "fields"))
        }
        Some("update_field") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .update_field(
                    &required_form_value(&form, "field_id")?,
                    field_input_from_form(&form)?,
                )
                .await?;
            Ok(lowcode_route(Some(&model_name), "fields"))
        }
        Some("delete_field") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .delete_field(&required_form_value(&form, "field_id")?)
                .await?;
            Ok(lowcode_route(Some(&model_name), "fields"))
        }
        Some("create_hook") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .create_hook(&model_name, hook_input_from_form(&form, true)?)
                .await?;
            Ok(lowcode_route(Some(&model_name), "hooks"))
        }
        Some("update_hook") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .update_hook(
                    &required_form_value(&form, "hook_id")?,
                    hook_input_from_form(&form, false)?,
                )
                .await?;
            Ok(lowcode_route(Some(&model_name), "hooks"))
        }
        Some("delete_hook") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .delete_hook(&required_form_value(&form, "hook_id")?)
                .await?;
            Ok(lowcode_route(Some(&model_name), "hooks"))
        }
        Some("create_record") => {
            let model_name = required_form_value(&form, "model_name")?;
            let fields = store.list_fields(&model_name).await?;
            let payload = payload_from_form(&fields, &form, EmptyFieldPolicy::Omit)?;
            store
                .executor()
                .insert_record(&model_name, Value::Object(payload))
                .await?;
            Ok(lowcode_route(Some(&model_name), "records"))
        }
        Some("update_record") => {
            let model_name = required_form_value(&form, "model_name")?;
            let record_id = required_form_value(&form, "record_id")?;
            let fields = store.list_fields(&model_name).await?;
            let payload = payload_from_form(&fields, &form, EmptyFieldPolicy::Null)?;
            store
                .executor()
                .update_record(&model_name, &record_id, Value::Object(payload))
                .await?;
            Ok(lowcode_route(Some(&model_name), "records"))
        }
        Some("delete_record") => {
            let model_name = required_form_value(&form, "model_name")?;
            store
                .executor()
                .delete_record(&model_name, &required_form_value(&form, "record_id")?)
                .await?;
            Ok(lowcode_route(Some(&model_name), "records"))
        }
        Some("create_operation") => {
            let operation = store
                .create_operation(operation_draft_from_form(&form)?)
                .await?;
            Ok(operation_route(Some(&operation.definition.operation_key)))
        }
        Some("create_operation_revision") => {
            let operation_key = required_form_value(&form, "operation_key")?;
            store
                .create_operation_revision(&operation_key, operation_revision_from_form(&form)?)
                .await?;
            Ok(operation_route(Some(&operation_key)))
        }
        Some("publish_operation") => {
            let operation_key = required_form_value(&form, "operation_key")?;
            store
                .publish_operation(&operation_key, &required_form_value(&form, "revision_id")?)
                .await?;
            Ok(operation_route(Some(&operation_key)))
        }
        Some("disable_operation") => {
            let operation_key = required_form_value(&form, "operation_key")?;
            store.disable_operation(&operation_key).await?;
            Ok(operation_route(Some(&operation_key)))
        }
        Some("test_operation") => {
            let operation_key = required_form_value(&form, "operation_key")?;
            let revision_id = required_form_value(&form, "revision_id")?;
            let result = store
                .test_operation_revision(
                    &operation_key,
                    &revision_id,
                    operation_test_input_from_form(&form)?,
                )
                .await?;
            let result =
                serde_json::to_string(&result).context("序列化 operation 试运行结果失败")?;
            Ok(operation_result_route(&operation_key, &result))
        }
        Some(other) => Err(anyhow!("未知 engine UI action: {other}")),
        None => Err(anyhow!("缺少 engine UI action")),
    }
}

fn operation_draft_from_form(form: &BTreeMap<String, String>) -> anyhow::Result<OperationDraft> {
    Ok(OperationDraft {
        operation_key: required_form_value(form, "operation_key")?,
        display_name: required_form_value(form, "display_name")?,
        description: form_value(form, "description").unwrap_or_default(),
        method: required_form_value(form, "method")?,
        executor_kind: "rhai".to_string(),
        source_text: required_form_value(form, "source_text")?,
        input_schema: form_json_object(form, "input_schema")?,
        output_schema: form_json_object(form, "output_schema")?,
        capability_policy: json!({ "allow": [] }),
        timeout_ms: form_i64_default(form, "timeout_ms", 3_000)?,
        generated_by_model: None,
    })
}

fn operation_revision_from_form(
    form: &BTreeMap<String, String>,
) -> anyhow::Result<OperationRevisionInput> {
    Ok(OperationRevisionInput {
        executor_kind: "rhai".to_string(),
        source_text: required_form_value(form, "source_text")?,
        input_schema: form_json_object(form, "input_schema")?,
        output_schema: form_json_object(form, "output_schema")?,
        capability_policy: json!({ "allow": [] }),
        timeout_ms: form_i64_default(form, "timeout_ms", 3_000)?,
        generated_by_model: None,
    })
}

fn operation_test_input_from_form(
    form: &BTreeMap<String, String>,
) -> anyhow::Result<OperationTestInput> {
    let query = match form_value(form, "query_json") {
        Some(value) => operation_query_from_json(&value)?,
        None => BTreeMap::new(),
    };
    let body = match form_value(form, "body_json") {
        Some(value) => serde_json::from_str(&value).context("operation body_json 需要合法 JSON")?,
        None => json!({}),
    };
    Ok(OperationTestInput {
        method: None,
        path: BTreeMap::new(),
        query,
        body,
    })
}

fn operation_query_from_json(value: &str) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let object = serde_json::from_str::<Value>(value)
        .context("operation query_json 需要合法 JSON")?
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("operation query_json 必须是 JSON object"))?;
    let mut query = BTreeMap::new();
    for (key, value) in object {
        let values = match value {
            Value::String(value) => vec![value],
            Value::Array(values) => values
                .into_iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| anyhow!("operation query_json 数组只能包含字符串: {key}"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            _ => bail!("operation query_json 字段只能是字符串或字符串数组: {key}"),
        };
        query.insert(key, values);
    }
    Ok(query)
}

fn form_json_object(form: &BTreeMap<String, String>, key: &str) -> anyhow::Result<Value> {
    let value = match form_value(form, key) {
        Some(value) => value,
        None => r#"{"type":"object"}"#.to_string(),
    };
    let value = serde_json::from_str::<Value>(&value)
        .with_context(|| format!("表单字段 {key} 需要合法 JSON"))?;
    if !value.is_object() {
        bail!("表单字段 {key} 必须是 JSON object");
    }
    Ok(value)
}

fn field_input_from_form(form: &BTreeMap<String, String>) -> anyhow::Result<FieldInput> {
    Ok(FieldInput {
        name: required_form_value(form, "name")?,
        display_name: required_form_value(form, "display_name")?,
        field_type: required_form_value(form, "field_type")?,
        is_required: form_bool(form, "is_required"),
        expression: form_value(form, "expression"),
        dependency_json: form_value(form, "dependency_json"),
        domain_metadata_json: form_value(form, "domain_metadata_json"),
        validation_json: form_value(form, "validation_json"),
        order_index: form_i32(form, "order_index")?,
    })
}

fn hook_input_from_form(
    form: &BTreeMap<String, String>,
    default_active: bool,
) -> anyhow::Result<HookInput> {
    let is_active = if default_active {
        form_bool_default_true(form, "is_active")
    } else {
        form_bool(form, "is_active")
    };
    Ok(HookInput {
        trigger_event: required_form_value(form, "trigger_event")?,
        script_content: required_form_value(form, "script_content")?,
        is_active,
        order_index: form_i32(form, "order_index")?,
    })
}

fn payload_from_form(
    fields: &[az_engine::MetaField],
    form: &BTreeMap<String, String>,
    empty_policy: EmptyFieldPolicy,
) -> anyhow::Result<Map<String, Value>> {
    let mut payload = Map::new();
    for field in fields {
        if field.field_type == "computed" {
            continue;
        }
        let key = format!("payload_{}", field.name);
        if field.field_type == "boolean" {
            payload.insert(field.name.clone(), Value::Bool(form_bool(form, &key)));
            continue;
        }
        let Some(raw) = form_value(form, &key) else {
            continue;
        };
        if raw.is_empty() && !field.is_required {
            if empty_policy == EmptyFieldPolicy::Null {
                payload.insert(field.name.clone(), Value::Null);
            }
            continue;
        }
        let value = match field.field_type.as_str() {
            "string" => Value::String(raw),
            "int" | "datetime" => Value::Number(
                raw.parse::<i64>()
                    .with_context(|| format!("字段 {} 需要整数", field.name))?
                    .into(),
            ),
            "decimal" => json!(
                raw.parse::<f64>()
                    .with_context(|| format!("字段 {} 需要数字", field.name))?
            ),
            "json" => serde_json::from_str(&raw)
                .with_context(|| format!("字段 {} 需要合法 JSON", field.name))?,
            _ => Value::String(raw),
        };
        payload.insert(field.name.clone(), value);
    }
    Ok(payload)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EmptyFieldPolicy {
    Omit,
    Null,
}

fn into_api_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    az_aio_platform::core::api_error::into_api_response(result)
}

fn into_bad_request_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(data) => az_aio_platform::core::api_error::ok_json(data).into_response(),
        Err(error) => ApiError::bad_request(error.to_string()).into_response(),
    }
}

fn page_query_from_raw(raw_query: Option<&str>) -> anyhow::Result<PageParams> {
    let mut page = PageParams::default();
    for pair in raw_query.unwrap_or_default().split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = urlencoding::decode(raw_key)
            .with_context(|| format!("分页参数名解码失败: {raw_key}"))?;
        let value = urlencoding::decode(raw_value)
            .with_context(|| format!("分页参数值解码失败: {raw_key}"))?;
        apply_page_query_pair(&mut page, &key, &value)?;
    }
    Ok(page)
}

fn apply_page_query_pair(page: &mut PageParams, key: &str, value: &str) -> anyhow::Result<()> {
    match key {
        "o" => {
            page.o = parse_usize_param("o", value)?;
        }
        "s" => {
            page.s = parse_usize_param("s", value)?;
        }
        _ => {}
    }
    Ok(())
}

fn required_form_value(form: &BTreeMap<String, String>, key: &str) -> anyhow::Result<String> {
    form_value(form, key).ok_or_else(|| anyhow!("缺少表单字段: {key}"))
}

fn form_value(form: &BTreeMap<String, String>, key: &str) -> Option<String> {
    form.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn form_i32(form: &BTreeMap<String, String>, key: &str) -> anyhow::Result<i32> {
    let Some(value) = form_value(form, key) else {
        return Ok(0);
    };
    value
        .parse::<i32>()
        .with_context(|| format!("表单字段 {key} 需要整数"))
}

fn form_i64_default(
    form: &BTreeMap<String, String>,
    key: &str,
    default: i64,
) -> anyhow::Result<i64> {
    let Some(value) = form_value(form, key) else {
        return Ok(default);
    };
    value
        .parse::<i64>()
        .with_context(|| format!("表单字段 {key} 需要整数"))
}

fn form_bool(form: &BTreeMap<String, String>, key: &str) -> bool {
    matches!(
        form.get(key).map(String::as_str),
        Some("1" | "true" | "on" | "yes")
    )
}

fn form_bool_default_true(form: &BTreeMap<String, String>, key: &str) -> bool {
    !matches!(
        form.get(key).map(String::as_str),
        Some("0" | "false" | "off" | "no")
    )
}

fn lowcode_route(model_name: Option<&str>, tab: &str) -> String {
    let route = match model_name {
        Some(model_name) => format!(
            "/lowcode?model={}&tab={tab}",
            urlencoding::encode(model_name)
        ),
        None => format!("/lowcode?tab={tab}"),
    };
    format!("/?route={}", urlencoding::encode(&route))
}

fn operation_route(operation_key: Option<&str>) -> String {
    let route = match operation_key {
        Some(operation_key) => format!(
            "/lowcode?tab=operations&operation={}",
            urlencoding::encode(operation_key)
        ),
        None => "/lowcode?tab=operations".to_string(),
    };
    format!("/?route={}", urlencoding::encode(&route))
}

fn operation_result_route(operation_key: &str, result: &str) -> String {
    let route = format!(
        "/lowcode?tab=operations&operation={}&result={}",
        urlencoding::encode(operation_key),
        urlencoding::encode(result)
    );
    format!("/?route={}", urlencoding::encode(&route))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_route_uses_new_lowcode_path() {
        let route = lowcode_route(Some("order"), "records");

        // SSR action 必须回到编码后的 route=/lowcode?... 路由。
        assert_eq!(route, "/?route=%2Flowcode%3Fmodel%3Dorder%26tab%3Drecords");
    }

    #[test]
    fn page_query_defaults_to_contract_page_size() {
        let page = page_query_from_raw(Some("o=20&s=50")).unwrap();

        // 列表接口继续使用 o/s 入参并回传 p。
        assert_eq!(page, PageParams { o: 20, s: 50 });
    }

    #[test]
    fn field_form_builds_update_input() {
        let form = BTreeMap::from([
            ("name".to_string(), "amount".to_string()),
            ("display_name".to_string(), "金额".to_string()),
            ("field_type".to_string(), "decimal".to_string()),
            ("is_required".to_string(), "1".to_string()),
            ("order_index".to_string(), "7".to_string()),
        ]);

        let input = field_input_from_form(&form).unwrap();

        // UI action 的字段编辑表单必须能落到真实 update_field 入参。
        assert_eq!(input.name, "amount");
        assert_eq!(input.field_type, "decimal");
        assert!(input.is_required);
        assert_eq!(input.order_index, 7);
    }

    #[test]
    fn hook_form_builds_inactive_update_input() {
        let form = BTreeMap::from([
            ("trigger_event".to_string(), "after_update".to_string()),
            ("script_content".to_string(), "let x = 1;".to_string()),
            ("order_index".to_string(), "3".to_string()),
        ]);

        let input = hook_input_from_form(&form, false).unwrap();

        // 未勾选启用时，更新钩子必须能真实写回 inactive。
        assert_eq!(input.trigger_event, "after_update");
        assert!(!input.is_active);
        assert_eq!(input.order_index, 3);
    }

    #[test]
    fn operation_query_preserves_repeated_values() {
        let query = match operation_query_from_raw(Some("tag=a&tag=b&empty=")) {
            Ok(value) => value,
            Err(error) => panic!("合法 query 应完成解析: {error}"),
        };

        // 动态接口不能把同名 query 参数压缩成单个字符串。
        assert_eq!(
            query.get("tag"),
            Some(&vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(query.get("empty"), Some(&vec![String::new()]));
    }

    #[test]
    fn operation_test_form_accepts_string_or_string_array_query() {
        let form = BTreeMap::from([
            (
                "query_json".to_string(),
                r#"{"tag":["a","b"],"state":"ready"}"#.to_string(),
            ),
            ("body_json".to_string(), r#"{"deviceId":"d-1"}"#.to_string()),
        ]);

        let input = match operation_test_input_from_form(&form) {
            Ok(value) => value,
            Err(error) => panic!("合法试运行表单应完成解析: {error}"),
        };

        // Admin 试运行输入应与统一网关的多值 query 上下文保持一致。
        assert_eq!(input.query["tag"], ["a", "b"]);
        assert_eq!(input.query["state"], ["ready"]);
        assert_eq!(input.body["deviceId"], "d-1");
    }
}
