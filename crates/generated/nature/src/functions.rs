pub struct ProcessTelemetryFunction;
impl ProcessTelemetryFunction {
    pub const fn encode() -> &'static str {
        "process_telemetry"
    }
}
impl crate::descriptors::Encode for ProcessTelemetryFunction {
    fn encode(&self) -> &'static str {
        Self::encode()
    }
}
pub fn process_telemetry(
    source: &std::collections::BTreeMap<String, serde_json::Value>,
) -> anyhow::Result<crate::structs::EnvironmentTelemetry> {
    crate::bindings::decode_environment_telemetry(source)
}
