use std::sync::Arc;

use anyhow::{Context, Result};
use az_admin_shell_core::{DynPageExtensionCompiler, PageCompileContext, PageExtensionCompiler};
use az_dioxus_admin_shell::{
    DynPageExtensionRenderer, PageExtensionEditorContext, PageExtensionRenderer,
    PageExtensionRuntimeContext,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub(super) struct AioRuntimePageExtension;

#[derive(Clone, Debug, Default)]
pub(super) struct AioStudioPageExtension;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct AioRuntimePageConfig {
    pub renderer: studio::PageRendererDefinition,
}

impl Default for AioRuntimePageConfig {
    fn default() -> Self {
        Self {
            renderer: studio::PageRendererDefinition::MenuTree,
        }
    }
}

impl PageExtensionCompiler for AioStudioPageExtension {
    fn key(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn title(&self) -> &'static str {
        "AIO Studio"
    }

    fn description(&self) -> &'static str {
        "AIO 模型、页面、函数、接口与权限编辑器"
    }

    fn schema_version(&self) -> u32 {
        1
    }

    fn default_config(&self) -> Value {
        Value::Object(Default::default())
    }

    fn validate(&self, _context: PageCompileContext<'_>, _config: &Value) -> Vec<String> {
        Vec::new()
    }

    fn compile(&self, _context: PageCompileContext<'_>, config: &Value) -> Result<Value> {
        Ok(config.clone())
    }
}

impl PageExtensionRenderer for AioStudioPageExtension {
    fn key(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn render_editor(&self, _context: PageExtensionEditorContext) -> Element {
        rsx! {}
    }

    fn render(&self, _context: PageExtensionRuntimeContext) -> Element {
        rsx! {
            studio::StudioPage { api_base_url: String::new(), published_scene: None }
        }
    }
}

impl PageExtensionCompiler for AioRuntimePageExtension {
    fn key(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn title(&self) -> &'static str {
        "AIO 菜单树"
    }

    fn description(&self) -> &'static str {
        "使用 AIO ProgramImage 渲染菜单管理页面"
    }

    fn schema_version(&self) -> u32 {
        1
    }

    fn default_config(&self) -> Value {
        serde_json::to_value(AioRuntimePageConfig::default()).unwrap_or_default()
    }

    fn validate(&self, _context: PageCompileContext<'_>, config: &Value) -> Vec<String> {
        serde_json::from_value::<AioRuntimePageConfig>(config.clone())
            .err()
            .map(|error| vec![format!("AIO 运行时页面配置无效: {error}")])
            .unwrap_or_default()
    }

    fn compile(&self, _context: PageCompileContext<'_>, config: &Value) -> Result<Value> {
        let config = serde_json::from_value::<AioRuntimePageConfig>(config.clone())
            .context("解析 AIO 运行时页面配置失败")?;
        serde_json::to_value(config).context("编译 AIO 运行时页面配置失败")
    }
}

impl PageExtensionRenderer for AioRuntimePageExtension {
    fn key(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn render_editor(&self, _context: PageExtensionEditorContext) -> Element {
        rsx! {}
    }

    fn render(&self, context: PageExtensionRuntimeContext) -> Element {
        rsx! {
            studio::RuntimePage { page_id: context.page.id.to_string() }
        }
    }
}

#[rudi::Singleton(name = std::any::type_name::<AioRuntimePageExtension>())]
fn runtime_page_compiler() -> DynPageExtensionCompiler {
    Arc::new(AioRuntimePageExtension)
}

#[rudi::Singleton(name = std::any::type_name::<AioRuntimePageExtension>())]
fn runtime_page_renderer() -> DynPageExtensionRenderer {
    Arc::new(AioRuntimePageExtension)
}

#[rudi::Singleton(name = std::any::type_name::<AioStudioPageExtension>())]
fn studio_page_compiler() -> DynPageExtensionCompiler {
    Arc::new(AioStudioPageExtension)
}

#[rudi::Singleton(name = std::any::type_name::<AioStudioPageExtension>())]
fn studio_page_renderer() -> DynPageExtensionRenderer {
    Arc::new(AioStudioPageExtension)
}
