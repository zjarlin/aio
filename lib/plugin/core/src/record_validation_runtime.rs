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
