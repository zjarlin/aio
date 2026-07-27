#[allow(dead_code)]
fn read_decimal(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> anyhow::Result<f64> {
    let value = source
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("缺少原始字段 {key}"))?;
    if let Some(number) = value.as_f64() {
        return Ok(number);
    }
    if let Some(text) = value.as_str() {
        return text
            .parse::<f64>()
            .map_err(|error| anyhow::anyhow!("原始字段 {key} 不是小数: {error}"));
    }
    anyhow::bail!("原始字段 {key} 不是小数")
}
#[allow(dead_code)]
fn read_integer(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> anyhow::Result<i64> {
    source
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("原始字段 {key} 不是整数"))
}
#[allow(dead_code)]
fn read_boolean(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> anyhow::Result<bool> {
    source
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| anyhow::anyhow!("原始字段 {key} 不是布尔值"))
}
#[allow(dead_code)]
fn read_string(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> anyhow::Result<String> {
    source
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("原始字段 {key} 不是文本"))
}
pub fn decode_environment_telemetry(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
) -> anyhow::Result<crate::structs::EnvironmentTelemetry> {
    let value = crate::structs::EnvironmentTelemetry {
        temperature: (read_decimal(source, "raw_temperature")?) / 10f64,
        humidity: (read_decimal(source, "raw_humidity")?) / 10f64,
    };
    crate::validators::validate_environment_telemetry(&value)?;
    Ok(value)
}
