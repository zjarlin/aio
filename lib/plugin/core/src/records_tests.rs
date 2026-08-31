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
fn escapes_literal_contains_patterns() {
    assert_eq!(contains_pattern(r"50%_off\now"), r"%50\%\_off\\now%");
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
