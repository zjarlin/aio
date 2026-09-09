use std::path::Path;

use anyhow::Result;
use az_plugin_manifest::PluginRuntime;

use super::scaffold::{TemplateFile, materialize};

const COMMON_FILES: &[TemplateFile] = &[TemplateFile {
    path: "kotlin",
    content: include_str!("../../templates/plugin/kotlin-process/kotlin"),
    executable: true,
}];

const PAGE_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../../templates/plugin/kotlin-pages/.gitignore"),
        executable: false,
    },
    TemplateFile {
        path: "aio-plugin.toml",
        content: include_str!("../../templates/plugin/kotlin-pages/aio-plugin.toml"),
        executable: false,
    },
    TemplateFile {
        path: "project.yaml",
        content: include_str!("../../templates/plugin/kotlin-pages/project.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "model/module.yaml",
        content: include_str!("../../templates/plugin/kotlin-pages/model/module.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "model/README.md",
        content: include_str!("../../templates/plugin/kotlin-pages/model/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "model/src/__PACKAGE_PATH__/PageDefinition.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/model/src/site/addzero/aio/plugin/kmppages/PageDefinition.kt"
        ),
        executable: false,
    },
    TemplateFile {
        path: "model/src/__PACKAGE_PATH__/README.md",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/model/src/site/addzero/aio/plugin/kmppages/README.md"
        ),
        executable: false,
    },
    TemplateFile {
        path: "model/test/__PACKAGE_PATH__/PageDefinitionTest.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/model/test/site/addzero/aio/plugin/kmppages/PageDefinitionTest.kt"
        ),
        executable: false,
    },
    TemplateFile {
        path: "model/test/__PACKAGE_PATH__/README.md",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/model/test/site/addzero/aio/plugin/kmppages/README.md"
        ),
        executable: false,
    },
    TemplateFile {
        path: "generator/module.yaml",
        content: include_str!("../../templates/plugin/kotlin-pages/generator/module.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "generator/README.md",
        content: include_str!("../../templates/plugin/kotlin-pages/generator/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "generator/src/__PACKAGE_PATH__/Main.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/generator/src/site/addzero/aio/plugin/kmppages/Main.kt"
        ),
        executable: false,
    },
    TemplateFile {
        path: "generator/src/__PACKAGE_PATH__/README.md",
        content: include_str!(
            "../../templates/plugin/kotlin-pages/generator/src/site/addzero/aio/plugin/kmppages/README.md"
        ),
        executable: false,
    },
];

const PROCESS_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: ".gitignore",
        content: include_str!("../../templates/plugin/kotlin-process/.gitignore"),
        executable: false,
    },
    TemplateFile {
        path: "aio-plugin.toml",
        content: include_str!("../../templates/plugin/kotlin-process/aio-plugin.toml"),
        executable: false,
    },
    TemplateFile {
        path: "project.yaml",
        content: include_str!("../../templates/plugin/kotlin-process/project.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "model/module.yaml",
        content: include_str!("../../templates/plugin/kotlin-process/model/module.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "model/README.md",
        content: include_str!("../../templates/plugin/kotlin-process/model/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "model/src/__PACKAGE_PATH__/RuntimeModel.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-process/model/src/site/addzero/aio/plugin/kmpservice/RuntimeModel.kt"
        ),
        executable: false,
    },
    TemplateFile {
        path: "model/test/__PACKAGE_PATH__/RuntimeModelTest.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-process/model/test/site/addzero/aio/plugin/kmpservice/RuntimeModelTest.kt"
        ),
        executable: false,
    },
    TemplateFile {
        path: "service/module.yaml",
        content: include_str!("../../templates/plugin/kotlin-process/service/module.yaml"),
        executable: false,
    },
    TemplateFile {
        path: "service/README.md",
        content: include_str!("../../templates/plugin/kotlin-process/service/README.md"),
        executable: false,
    },
    TemplateFile {
        path: "service/src/__PACKAGE_PATH__/Main.kt",
        content: include_str!(
            "../../templates/plugin/kotlin-process/service/src/site/addzero/aio/plugin/kmpservice/Main.kt"
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
        PluginRuntime::Process => process_plugin(path, name, title),
        _ => unreachable!("语言与运行目标已在初始化入口校验"),
    }
}

fn page_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let package = format!("site.addzero.aio.plugin.{}", name.replace('-', "_"));
    let package_path = package.replace('.', "/");
    let name_literal = serde_json::to_string(name)?;
    let about_literal = serde_json::to_string(&format!("{name}-about"))?;
    let title_literal = serde_json::to_string(title)?;
    let replacements = [
        ("__PACKAGE_PATH__", package_path),
        ("__PACKAGE_NAME__", package),
        ("__PRIMARY_ID__", name_literal),
        ("__ABOUT_ID__", about_literal),
        ("__TITLE__", title_literal),
    ];
    materialize(path, COMMON_FILES, &replacements)?;
    materialize(path, PAGE_FILES, &replacements)?;
    super::write(&path.join("README.md"), &page_readme(title))
}

fn process_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
    let package = format!("site.addzero.aio.plugin.{}", name.replace('-', "_"));
    let package_path = package.replace('.', "/");
    let title_literal = serde_json::to_string(title)?;
    let replacements = [
        ("__PACKAGE_PATH__", package_path),
        ("site.addzero.aio.plugin.kmpservice", package),
        ("kmp-process", name.to_owned()),
        ("\"KMP 服务\"", title_literal.clone()),
        ("\"Kotlin 进程插件 v2 已在线\"", title_literal),
    ];
    materialize(path, COMMON_FILES, &replacements)?;
    materialize(path, PROCESS_FILES, &replacements)?;
    super::write(&path.join("README.md"), &process_readme(title))
}

fn page_readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是使用 Kotlin Toolchain 构建的 AIO `page-definition` 插件。页面模型位于 commonMain，并同时通过 JVM 与 wasmJs 编译；JVM 生成器只负责输出语言无关的安装产物。\n\n```bash\n./kotlin check\n./kotlin build -m model -p jvm -p wasmJs\n./kotlin run -m generator -p jvm -- dist/pages.json\naio plugin validate\n```\n\n发布前提交 `dist/pages.json`；静态页面插件没有运行实例，也不能声明动作页面。\n"
    )
}

fn process_readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是使用 Kotlin Toolchain 构建的 AIO `process` 插件。页面和请求模型位于 commonMain，JVM 服务只负责 HTTP 适配。\n\n```bash\n./kotlin check\n./kotlin build -m model -p wasmJs\n./kotlin package -m service -p jvm -f executable-jar\nmkdir -p dist\ncp build/tasks/_service_executableJarJvm/service-jvm-executable.jar dist/plugin.jar\naio plugin validate\n```\n\n发布前提交 `dist/plugin.jar`，生产安装器不会执行 Kotlin 构建。\n"
    )
}
