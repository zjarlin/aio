use anyhow::{Result, bail, ensure};
use az_plugin_manifest::PluginRuntime;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PluginLanguage {
    #[default]
    Rust,
    Kotlin,
    TypeScript,
}

impl PluginLanguage {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "rust" => Ok(Self::Rust),
            "kotlin" => Ok(Self::Kotlin),
            "typescript" => Ok(Self::TypeScript),
            _ => bail!("插件语言必须是 rust、kotlin 或 typescript: {value}"),
        }
    }

    pub fn default_runtime(self) -> PluginRuntime {
        match self {
            Self::Rust => PluginRuntime::RustSource,
            Self::Kotlin => PluginRuntime::Process,
            Self::TypeScript => PluginRuntime::WasmComponent,
        }
    }

    pub fn validate_runtime(self, runtime: PluginRuntime) -> Result<()> {
        let supported = matches!(
            (self, runtime),
            (Self::Rust, PluginRuntime::RustSource)
                | (Self::Kotlin, PluginRuntime::Process)
                | (Self::Kotlin, PluginRuntime::PageDefinition)
                | (Self::TypeScript, PluginRuntime::WasmComponent)
                | (Self::TypeScript, PluginRuntime::Process)
                | (Self::TypeScript, PluginRuntime::PageDefinition)
        );
        ensure!(
            supported,
            "当前脚手架只支持 rust+rust-source、kotlin+page-definition、kotlin+process、typescript+page-definition、typescript+wasm-component、typescript+process"
        );
        Ok(())
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Kotlin => "kotlin",
            Self::TypeScript => "typescript",
        }
    }
}

pub fn parse_runtime(value: &str) -> Result<PluginRuntime> {
    match value {
        "rust-source" => Ok(PluginRuntime::RustSource),
        "page-definition" => Ok(PluginRuntime::PageDefinition),
        "wasm-component" => Ok(PluginRuntime::WasmComponent),
        "process" => Ok(PluginRuntime::Process),
        _ => bail!(
            "插件运行目标必须是 rust-source、page-definition、wasm-component 或 process: {value}"
        ),
    }
}

pub fn runtime_name(runtime: PluginRuntime) -> &'static str {
    match runtime {
        PluginRuntime::RustSource => "rust-source",
        PluginRuntime::PageDefinition => "page-definition",
        PluginRuntime::WasmComponent => "wasm-component",
        PluginRuntime::Process => "process",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_stable_default_runtime_to_each_language() {
        assert_eq!(
            PluginLanguage::Rust.default_runtime(),
            PluginRuntime::RustSource
        );
        assert_eq!(
            PluginLanguage::Kotlin.default_runtime(),
            PluginRuntime::Process
        );
        assert_eq!(
            PluginLanguage::TypeScript.default_runtime(),
            PluginRuntime::WasmComponent
        );
    }

    #[test]
    fn rejects_unpublished_language_runtime_combinations() {
        assert!(
            PluginLanguage::Kotlin
                .validate_runtime(PluginRuntime::WasmComponent)
                .is_err()
        );
        assert!(
            PluginLanguage::TypeScript
                .validate_runtime(PluginRuntime::RustSource)
                .is_err()
        );
        assert!(
            PluginLanguage::Kotlin
                .validate_runtime(PluginRuntime::PageDefinition)
                .is_ok()
        );
        assert!(
            PluginLanguage::TypeScript
                .validate_runtime(PluginRuntime::PageDefinition)
                .is_ok()
        );
    }
}
