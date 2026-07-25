use std::collections::BTreeMap;
#[test]
fn fixture_map_decodes_and_validates_the_domain_value() -> anyhow::Result<()> {
    let mut source = BTreeMap::new();
    source.insert("temp_x10".to_string(), serde_json::json!(253f64));
    source.insert("humidity_x10".to_string(), serde_json::json!(612f64));
    let decoded = az_aio_nature_generated::bindings::decode_environment_telemetry(&source)?;
    assert!((decoded.temperature - 25.3f64).abs() < f64::EPSILON);
    assert!((decoded.humidity - 61.2f64).abs() < f64::EPSILON);
    assert_eq!(
        az_aio_nature_generated::descriptors::EnvironmentTelemetryTemperatureField::encode(),
        "temperature",
    );
    Ok(())
}
#[test]
fn missing_fixture_field_is_rejected() {
    let source = BTreeMap::new();
    let result = az_aio_nature_generated::bindings::decode_environment_telemetry(&source);
    assert!(result.is_err());
    let message = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(message.contains("temp_x10"));
}
#[test]
fn fixture_type_error_is_rejected() {
    let mut source = BTreeMap::new();
    source.insert("temp_x10".to_string(), serde_json::json!(253f64));
    source.insert("humidity_x10".to_string(), serde_json::json!(612f64));
    source.insert("temp_x10".to_string(), serde_json::json!("不是有效数值"));
    let result = az_aio_nature_generated::bindings::decode_environment_telemetry(&source);
    assert!(result.is_err());
}
#[test]
fn out_of_range_fixture_value_is_rejected() {
    let mut source = BTreeMap::new();
    source.insert("temp_x10".to_string(), serde_json::json!(253f64));
    source.insert("humidity_x10".to_string(), serde_json::json!(612f64));
    source.insert("temp_x10".to_string(), serde_json::json!(1260f64));
    let result = az_aio_nature_generated::bindings::decode_environment_telemetry(&source);
    assert!(result.is_err());
}
