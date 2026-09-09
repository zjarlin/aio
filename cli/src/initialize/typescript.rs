use std::path::Path;

use anyhow::Result;

use super::scaffold::{TemplateFile, materialize};

const FILES: &[TemplateFile] = &[
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

pub fn repository_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let title_literal = serde_json::to_string(title)?;
    let name_literal = serde_json::to_string(name)?;
    let replacements = [
        ("\"aio-plugin-ts-component\"", name_literal),
        ("ts-counter", name.to_owned()),
        ("ts-echo-service", format!("{name}-echo")),
        ("\"TypeScript Component\"", title_literal.clone()),
        ("\"TypeScript\"", title_literal),
    ];
    materialize(path, FILES, &replacements)?;
    super::write(&path.join("README.md"), &readme(title))
}

fn readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是实现 `aio:plugin/page@1` 的 TypeScript Wasm Component 插件。它返回语言无关的 PageDefinition，不直接接管宿主 DOM。\n\n```bash\ncorepack enable\npnpm install --frozen-lockfile --ignore-scripts\npnpm typecheck\npnpm test\npnpm build\nwasm-tools validate --features component-model dist/plugin.wasm\nwasm-tools component wit dist/plugin.wasm\naio plugin validate\n```\n\n发布前提交 `pnpm-lock.yaml` 和 `dist/plugin.wasm`，生产安装器不会执行 pnpm。\n"
    )
}
