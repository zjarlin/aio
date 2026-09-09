use std::path::Path;

use anyhow::Result;

use super::scaffold::{TemplateFile, materialize};

const FILES: &[TemplateFile] = &[
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
        path: "kotlin",
        content: include_str!("../../templates/plugin/kotlin-process/kotlin"),
        executable: true,
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

pub fn repository_plugin(path: &Path, name: &str, title: &str) -> Result<()> {
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
    materialize(path, FILES, &replacements)?;
    super::write(&path.join("README.md"), &readme(title))
}

fn readme(title: &str) -> String {
    format!(
        "# {title}\n\n这是使用 Kotlin Toolchain 构建的 AIO `process` 插件。页面和请求模型位于 commonMain，JVM 服务只负责 HTTP 适配。\n\n```bash\n./kotlin check\n./kotlin build -m model -p wasmJs\n./kotlin package -m service -p jvm -f executable-jar\nmkdir -p dist\ncp build/tasks/_service_executableJarJvm/service-jvm-executable.jar dist/plugin.jar\naio plugin validate\n```\n\n发布前提交 `dist/plugin.jar`，生产安装器不会执行 Kotlin 构建。\n"
    )
}
