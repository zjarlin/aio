use serde::{Deserialize, Serialize};

az_dict_macros::dict_enum!(
    name = IotOnlineStatus,
    dict = "iot_online_status",
    spec = include_str!("../specs/iot_online_status.json"),
    raw_type = &'static str
);

impl Serialize for IotOnlineStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.raw_value())
    }
}

impl<'de> Deserialize<'de> for IotOnlineStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as Deserialize>::deserialize(deserializer)?;
        let raw_value = Self::items()
            .iter()
            .find(|item| item.raw_value == value)
            .map(|item| item.raw_value);
        raw_value
            .and_then(Self::from_raw)
            .ok_or_else(|| serde::de::Error::custom(format!("未知的设备在线状态：{value}")))
    }
}
