/// 生成类型和值的稳定代码身份。
pub trait Encode {
    fn encode(&self) -> &'static str;
}
/// 字段、结构和函数共享的只读语义描述。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Descriptor {
    pub code: &'static str,
    pub label: &'static str,
    pub unit: Option<&'static str>,
}
pub struct EnvironmentTelemetryTemperatureField;
impl EnvironmentTelemetryTemperatureField {
    pub const DESCRIPTOR: Descriptor = Descriptor {
        code: "temperature",
        label: "温度",
        unit: Some("摄氏度"),
    };
    pub const fn encode() -> &'static str {
        "temperature"
    }
}
impl Encode for EnvironmentTelemetryTemperatureField {
    fn encode(&self) -> &'static str {
        Self::encode()
    }
}
pub struct EnvironmentTelemetryHumidityField;
impl EnvironmentTelemetryHumidityField {
    pub const DESCRIPTOR: Descriptor = Descriptor {
        code: "humidity",
        label: "湿度",
        unit: Some("百分比"),
    };
    pub const fn encode() -> &'static str {
        "humidity"
    }
}
impl Encode for EnvironmentTelemetryHumidityField {
    fn encode(&self) -> &'static str {
        Self::encode()
    }
}
