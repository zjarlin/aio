use anyhow::bail;
use serde::{Deserialize, Serialize};

macro_rules! coded_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($(#[$variant_meta:meta])* $variant:ident => $code:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        pub enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl $name {
            pub const fn encode(self) -> &'static str {
                match self {
                    $(Self::$variant => $code),+
                }
            }
        }

        impl crate::descriptors::Encode for $name {
            fn encode(&self) -> &'static str {
                (*self).encode()
            }
        }
    };
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PluginActivation {
        Eager => "eager",
        Lazy => "lazy",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PluginKind {
        Native => "native",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PluginState {
        Discovered => "discovered",
        Loaded => "loaded",
        Active => "active",
        Disabled => "disabled",
        Failed => "failed",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum AdminMenuNodeKind {
        Branch => "branch",
        Page => "page",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum AdminFieldKind {
        Text => "text",
        Number => "number",
        Boolean => "boolean",
        Badge => "badge",
        Time => "time",
        Json => "json",
        Relation => "relation",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum UiContributionSlot {
        AppSidebar => "app-sidebar",
        AppTopbar => "app-topbar",
        Content => "content",
        SettingsContent => "settings-content",
        ProjectSidebar => "project-sidebar",
        ProjectContent => "project-content",
        SandboxPanel => "sandbox-panel",
    }
}

impl UiContributionSlot {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AppSidebar => "应用侧边栏",
            Self::AppTopbar => "应用顶栏",
            Self::Content => "内容区",
            Self::SettingsContent => "设置内容区",
            Self::ProjectSidebar => "项目侧边栏",
            Self::ProjectContent => "项目内容区",
            Self::SandboxPanel => "沙箱调试面板",
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum CatalogTagGroup {
        Developer => "developer",
        Design => "design",
    }
}

impl CatalogTagGroup {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Developer => "开发人员",
            Self::Design => "设计",
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum CatalogItemKind {
        Plugin => "plugin",
        Skill => "skill",
    }
}

impl CatalogItemKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Plugin => "插件",
            Self::Skill => "技能",
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum CatalogSource {
        Bundled => "bundled",
        Community => "community",
        Local => "local",
        System => "system",
        User => "user",
    }
}

impl CatalogSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bundled => "预置",
            Self::Community => "社区",
            Self::Local => "本地",
            Self::System => "系统",
            Self::User => "用户",
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ShellEntryKind {
        Alias => "alias",
        Export => "export",
        Function => "function",
        ScriptSnippet => "script-snippet",
    }
}

impl ShellEntryKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Alias => "别名",
            Self::Export => "环境变量",
            Self::Function => "函数",
            Self::ScriptSnippet => "脚本片段",
        }
    }

    pub const fn is_cli(self) -> bool {
        matches!(self, Self::Alias | Self::Function | Self::ScriptSnippet)
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum GeneratedFileStatus {
        Generated => "generated",
        Failed => "failed",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum SystemFeatureStatus {
        StarterBacked => "starter-backed",
        ReferenceOnly => "reference-only",
    }
}

impl SystemFeatureStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::StarterBacked => "已接入 starter",
            Self::ReferenceOnly => "参考建模",
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum SystemFieldKind {
        Text => "text",
        Badge => "badge",
        Count => "count",
        Time => "time",
        Route => "route",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum PageState {
        #[default]
        Draft => "draft",
        Published => "published",
        Disabled => "disabled",
    }
}

impl PageState {
    pub const fn as_str(self) -> &'static str {
        self.encode()
    }
}

impl TryFrom<&str> for PageState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "disabled" => Ok(Self::Disabled),
            _ => bail!("未知页面状态: {value}"),
        }
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ComponentShape {
        Leaf => "leaf",
        Container => "container",
        Dual => "dual",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ComponentPropertyKind {
        Text => "text",
        Boolean => "boolean",
        Number => "number",
        Choice => "choice",
        Action => "action",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ComponentBehavior {
        #[default]
        Generic => "generic",
        Button => "button",
        Input => "input",
        Progress => "progress",
        Table => "table",
    }
}

impl ComponentBehavior {
    pub fn is_generic(value: &Self) -> bool {
        *value == Self::Generic
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum WeatherProvider {
        #[default]
        OpenMeteo => "open_meteo",
    }
}

coded_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum LinuxDistribution {
        Ubuntu => "ubuntu",
    }
}

impl LinuxDistribution {
    pub const fn id(self) -> &'static str {
        self.encode()
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ubuntu => "Ubuntu",
        }
    }
}

::az_dict_macros::dict_enum!(
    name = IotOnlineStatus,
    dict = "iot_online_status",
    spec = include_str!("../specs/iot_online_status.json"),
    raw_type = &'static str
);

impl crate::descriptors::Encode for IotOnlineStatus {
    fn encode(&self) -> &'static str {
        IotOnlineStatus::encode(self)
    }
}

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

#[cfg(test)]
mod tests {
    use super::IotOnlineStatus;

    #[test]
    fn dictionary_status_uses_encode_and_mother_tongue_label() {
        assert_eq!(IotOnlineStatus::HeartbeatLost.encode(), "heartbeat_lost");
        assert_eq!(IotOnlineStatus::HeartbeatLost.label(), "心跳丢失");
        assert_eq!(IotOnlineStatus::default(), IotOnlineStatus::Online);
    }
}

// nature-compiler 动态枚举开始
// nature-compiler 动态枚举结束
