#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentTelemetry {
    pub temperature: f64,
    pub humidity: f64,
}
impl EnvironmentTelemetry {
    pub const fn encode() -> &'static str {
        "environment_telemetry"
    }
}
impl super::descriptors::Encode for EnvironmentTelemetry {
    fn encode(&self) -> &'static str {
        Self::encode()
    }
}
