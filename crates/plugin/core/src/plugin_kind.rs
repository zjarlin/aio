use serde::{Deserialize, Serialize};

macro_rules! plugin_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn encode(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }
    };
}

plugin_enum!(PluginActivation { Eager => "eager", Lazy => "lazy" });
plugin_enum!(PluginKind { Native => "native" });
plugin_enum!(PluginState {
    Discovered => "discovered",
    Loaded => "loaded",
    Active => "active",
    Disabled => "disabled",
    Failed => "failed",
});
plugin_enum!(AdminMenuNodeKind { Branch => "branch", Page => "page" });
plugin_enum!(AdminFieldKind {
    Text => "text",
    Number => "number",
    Boolean => "boolean",
    Badge => "badge",
    Time => "time",
    Json => "json",
    Relation => "relation",
});
plugin_enum!(UiContributionSlot {
    AppSidebar => "app-sidebar",
    AppTopbar => "app-topbar",
    Content => "content",
    SettingsContent => "settings-content",
    ProjectSidebar => "project-sidebar",
    ProjectContent => "project-content",
    SandboxPanel => "sandbox-panel",
});
plugin_enum!(CatalogTagGroup { Developer => "developer", Design => "design" });
plugin_enum!(CatalogItemKind { Plugin => "plugin", Skill => "skill" });
plugin_enum!(CatalogSource {
    Bundled => "bundled",
    Community => "community",
    Local => "local",
    System => "system",
    User => "user",
});

impl CatalogTagGroup {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Developer => "开发人员",
            Self::Design => "设计",
        }
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
