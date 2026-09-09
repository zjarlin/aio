use std::path::Path;

use anyhow::Result;
use az_plugin_manifest::PluginRuntime;

use super::scaffold::{TemplateFile, materialize};

const PAGE_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../../templates/plugin/typescript-pages/.gitignore"),
        executable: false,
    },
    TemplateFile {
        path: "aio-plugin.toml",
        content: include_str!("../../templates/plugin/typescript-pages/aio-plugin.toml"),
        executable: false,
    },
    TemplateFile {
        path: "package.json",
        content: include_str!("../../templates/plugin/typescript-pages/package.json"),
        executable: false,
    },
    TemplateFile {
        path: "pnpm-lock.yaml",
        content: include_str!("../../templates/plugin/typescript-process/pnpm-lock.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "tsconfig.json",
        content: include_str!("../../templates/plugin/typescript-pages/tsconfig.json"),
        executable: false,
    },
    TemplateFile {
        path: "src/node-runtime.d.ts",
        content: include_str!("../../templates/plugin/typescript-pages/src/node-runtime.d.ts"),
        executable: false,
    },
    TemplateFile {
        path: "src/pages/README.md",
        content: include_str!("../../templates/plugin/typescript-pages/src/pages/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "src/pages/definition.ts",
        content: include_str!("../../templates/plugin/typescript-pages/src/pages/definition.ts"),
        executable: false,
    },
    TemplateFile {
        path: "src/generator/README.md",
        content: include_str!("../../templates/plugin/typescript-pages/src/generator/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "src/generator/main.ts",
        content: include_str!("../../templates/plugin/typescript-pages/src/generator/main.ts"),
        executable: false,
    },
    TemplateFile {
        path: "test/pages/README.md",
        content: include_str!("../../templates/plugin/typescript-pages/test/pages/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "test/pages/definition.test.mjs",
        content: include_str!(
            "../../templates/plugin/typescript-pages/test/pages/definition.test.mjs"
        ),
        executable: false,
    },
];

const COMPONENT_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../../templates/plugin/typescript-component/.gitignore"),
        executable: false,
    },
    TemplateFile {
        path: "aio-plugin.toml",
        content: include_str!("../../templates/plugin/typescript-component/aio-plugin.toml"),
        executable: false,
    },
    TemplateFile {
        path: "package.json",
        content: include_str!("../../templates/plugin/typescript-component/package.json"),
        executable: false,
    },
    TemplateFile {
        path: "pnpm-lock.yaml",
        content: include_str!("../../templates/plugin/typescript-component/pnpm-lock.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "tsconfig.json",
        content: include_str!("../../templates/plugin/typescript-component/tsconfig.json"),
        executable: false,
    },
    TemplateFile {
        path: "src/plugin/README.md",
        content: include_str!("../../templates/plugin/typescript-component/src/plugin/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "src/plugin/component.ts",
        content: include_str!(
            "../../templates/plugin/typescript-component/src/plugin/component.ts"
        ),
        executable: false,
    },
    TemplateFile {
        path: "test/plugin/README.md",
        content: include_str!("../../templates/plugin/typescript-component/test/plugin/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "test/plugin/component.test.mjs",
        content: include_str!(
            "../../templates/plugin/typescript-component/test/plugin/component.test.mjs"
        ),
        executable: false,
    },
    TemplateFile {
        path: "wit/page.wit",
        content: include_str!("../../templates/plugin/typescript-component/wit/page.wit"),
        executable: false,
    },
];

const PROCESS_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../../templates/plugin/typescript-process/.gitignore"),
        executable: false,
    },
    TemplateFile {
        path: "aio-plugin.toml",
        content: include_str!("../../templates/plugin/typescript-process/aio-plugin.toml"),
        executable: false,
    },
    TemplateFile {
        path: "package.json",
        content: include_str!("../../templates/plugin/typescript-process/package.json"),
        executable: false,
    },
    TemplateFile {
        path: "pnpm-lock.yaml",
        content: include_str!("../../templates/plugin/typescript-process/pnpm-lock.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "tsconfig.json",
        content: include_str!("../../templates/plugin/typescript-process/tsconfig.json"),
        executable: false,
    },
    TemplateFile {
        path: "src/node-http.d.ts",
        content: include_str!("../../templates/plugin/typescript-process/src/node-http.d.ts"),
        executable: false,
    },
    TemplateFile {
        path: "src/service/README.md",
        content: include_str!("../../templates/plugin/typescript-process/src/service/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "src/service/server.ts",
        content: include_str!("../../templates/plugin/typescript-process/src/service/server.ts"),
        executable: false,
    },
    TemplateFile {
        path: "test/service/README.md",
        content: include_str!("../../templates/plugin/typescript-process/test/service/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "test/service/server.test.mjs",
        content: include_str!(
            "../../templates/plugin/typescript-process/test/service/server.test.mjs"
        ),
        executable: false,
    },
];

pub fn repository_plugin(
    path: &Path,
    name: &str,
    title: &str,
    runtime: PluginRuntime,
) -> Result<()> {
    match runtime {
        PluginRuntime::PageDefinition => page_plugin(path, name, title),
        PluginRuntime::WasmComponent => component_plugin(path, name, title),
        PluginRuntime::Process => process_plugin(path, name, title),
        _ => unreachable!("语言与运行目标已在初始化入口校验"),
    }
}

fn page_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let title_literal = serde_json::to_string(title)?;
    let name_literal = serde_json::to_string(name)?;
    let about_literal = serde_json::to_string(&format!("{name}-about"))?;
    let replacements = [
        ("__PLUGIN_NAME__", name_literal.clone()),
        ("__PRIMARY_ID__", name_literal),
        ("__ABOUT_ID__", about_literal),
        ("__TITLE__", title_literal),
    ];
    materialize(path, PAGE_FILES, &replacements)?;
    super::write(&path.join("README.md"), &page_readme(title))
}

fn component_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let title_literal = serde_json::to_string(title)?;
    let name_literal = serde_json::to_string(name)?;
    let replacements = [
        ("\"aio-plugin-ts-component\"", name_literal),
        ("ts-counter", name.to_owned()),
        ("ts-echo-service", format!("{name}-echo")),
        ("\"TypeScript Component\"", title_literal.clone()),
        ("\"TypeScript\"", title_literal),
    ];
    materialize(path, COMPONENT_FILES, &replacements)?;
    super::write(&path.join("README.md"), &component_readme(title))
}

fn process_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let title_literal = serde_json::to_string(title)?;
    let name_literal = serde_json::to_string(name)?;
    let replacements = [
        ("\"aio-plugin-ts-service\"", name_literal),
        ("ts-process", name.to_owned()),
        ("\"TS 服务\"", title_literal.clone()),
        ("\"TypeScript 进程插件 v2 已在线\"", title_literal),
    ];
    materialize(path, PROCESS_FILES, &replacements)?;
    super::write(&path.join("README.md"), &process_readme(title))
}

fn page_readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是 TypeScript `page-definition` 插件。类型化模型生成 `dist/pages.json`，宿主在安装时校验并挂载页面，不创建运行实例。\n\n```bash\ncorepack enable\npnpm install --frozen-lockfile --ignore-scripts\npnpm typecheck\npnpm test\npnpm build\naio plugin validate\n```\n\n发布前提交 `pnpm-lock.yaml` 和 `dist/pages.json`；静态页面插件不能声明动作页面。\n"
    )
}

fn component_readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是实现 `aio:plugin/page@1` 的 TypeScript Wasm Component 插件。它返回语言无关的 PageDefinition，不直接接管宿主 DOM。\n\n```bash\ncorepack enable\npnpm install --frozen-lockfile --ignore-scripts\npnpm typecheck\npnpm test\npnpm build\nwasm-tools validate --features component-model dist/plugin.wasm\nwasm-tools component wit dist/plugin.wasm\naio plugin validate\n```\n\n发布前提交 `pnpm-lock.yaml` 和 `dist/plugin.wasm`，生产安装器不会执行 pnpm。\n"
    )
}

fn process_readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是使用 Node 标准库承载的 TypeScript `process` 插件。它监听 `AIO_PLUGIN_PORT`，页面动作使用严格 `PluginRequest`，状态由宿主持久化。\n\n```bash\ncorepack enable\npnpm install --frozen-lockfile --ignore-scripts\npnpm typecheck\npnpm test\npnpm build\naio plugin validate\n```\n\n发布前提交 `pnpm-lock.yaml` 和 `dist/service/server.js`，生产安装器不会执行 pnpm。\n"
    )
}
