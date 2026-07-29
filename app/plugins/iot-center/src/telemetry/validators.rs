pub fn validate_environment_telemetry(
    value: &super::structs::EnvironmentTelemetry,
) -> anyhow::Result<()> {
    if value.temperature < -40f64 || value.temperature > 125f64 {
        anyhow::bail!(concat!("温度", "超出允许范围 {} 到 {}"), -40f64, 125f64,);
    }
    if value.humidity < 0f64 || value.humidity > 100f64 {
        anyhow::bail!(concat!("湿度", "超出允许范围 {} 到 {}"), 0f64, 100f64,);
    }
    Ok(())
}
