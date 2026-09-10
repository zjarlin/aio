#![forbid(unsafe_code)]

mod initialize;
mod marketplace;
mod repository_plugin;

use std::{env, path::PathBuf};

use anyhow::{Result, bail};
use initialize::{
    ApplicationOptions, PluginLanguage, PluginTemplate, RepositoryPluginOptions, parse_runtime,
};
use repository_plugin::{PluginSource, PublicationOptions};

fn main() -> Result<()> {
    run(env::args().skip(1).collect())
}

fn run(arguments: Vec<String>) -> Result<()> {
    let Some(command) = arguments.first().map(String::as_str) else {
        print_usage();
        return Ok(());
    };

    match command {
        "init" => initialize_application(&arguments[1..]),
        "marketplace" => run_marketplace_command(&arguments[1..]),
        "plugin" => run_plugin_command(&arguments[1..]),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        _ => bail!("未知命令: {command}\n\n{}", usage()),
    }
}

fn run_marketplace_command(arguments: &[String]) -> Result<()> {
    let Some(command) = arguments.first().map(String::as_str) else {
        bail!("缺少市场命令\n\n{}", usage());
    };
    match (command, &arguments[1..]) {
        ("build", []) => marketplace::build(
            &PathBuf::from("marketplace/registry"),
            &PathBuf::from("marketplace/index.json"),
        ),
        ("build", [registry, output]) => {
            marketplace::build(&PathBuf::from(registry), &PathBuf::from(output))
        }
        ("build", _) => bail!("marketplace build 只接受可选的 <registry> <output>"),
        _ => bail!("未知市场命令: {command}\n\n{}", usage()),
    }
}

fn initialize_application(arguments: &[String]) -> Result<()> {
    let (path, name, title) = parse_init_arguments(arguments)?;
    initialize::application(ApplicationOptions { path, name, title })
}

fn run_plugin_command(arguments: &[String]) -> Result<()> {
    let Some(command) = arguments.first().map(String::as_str) else {
        bail!("缺少插件命令\n\n{}", usage());
    };

    match command {
        "init" if arguments[1..].iter().any(|argument| is_help(argument)) => {
            println!("{}", plugin_init_usage());
            Ok(())
        }
        "init" => initialize::repository_plugin(parse_plugin_init_arguments(&arguments[1..])?),
        "install" => {
            let source = parse_plugin_source(&arguments[1..])?;
            repository_plugin::install(&env::current_dir()?, source)
        }
        "publish" if arguments[1..].iter().any(|argument| is_help(argument)) => {
            println!("{}", plugin_publish_usage());
            Ok(())
        }
        "publish" => repository_plugin::publish(parse_plugin_publish_arguments(&arguments[1..])?),
        "uninstall" => {
            let git = exactly_one(&arguments[1..], "缺少要卸载的 Git 地址")?;
            repository_plugin::uninstall(&env::current_dir()?, git)
        }
        "sync" => no_arguments(&arguments[1..], || {
            repository_plugin::sync(&env::current_dir()?)
        }),
        "list" => no_arguments(&arguments[1..], || {
            repository_plugin::list(&env::current_dir()?)
        }),
        "validate" => match &arguments[1..] {
            [] => repository_plugin::validate(&env::current_dir()?),
            [path] => repository_plugin::validate(&PathBuf::from(path)),
            _ => bail!("plugin validate 只接受可选的 <仓库目录>"),
        },
        "schema" => match &arguments[1..] {
            [] => repository_plugin::write_schemas(&PathBuf::from("schemas")),
            [path] => repository_plugin::write_schemas(&PathBuf::from(path)),
            _ => bail!("plugin schema 只接受可选的 <输出目录>"),
        },
        _ => bail!("未知插件命令: {command}\n\n{}", usage()),
    }
}

fn parse_init_arguments(arguments: &[String]) -> Result<(PathBuf, Option<String>, Option<String>)> {
    let Some(path) = arguments.first() else {
        bail!("缺少初始化目录");
    };
    let mut name = None;
    let mut title = None;
    let mut index = 1;
    while index < arguments.len() {
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("选项 {} 缺少值", arguments[index]))?;
        match arguments[index].as_str() {
            "--name" => name = Some(value.clone()),
            "--title" => title = Some(value.clone()),
            option => bail!("未知初始化选项: {option}"),
        }
        index += 2;
    }
    Ok((PathBuf::from(path), name, title))
}

fn parse_plugin_init_arguments(arguments: &[String]) -> Result<RepositoryPluginOptions> {
    let Some(path) = arguments.first() else {
        bail!("缺少初始化目录");
    };
    let mut name = None;
    let mut title = None;
    let mut language = None;
    let mut runtime = None;
    let mut index = 1;
    while index < arguments.len() {
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("选项 {} 缺少值", arguments[index]))?;
        match arguments[index].as_str() {
            "--name" => name = Some(value.clone()),
            "--title" => title = Some(value.clone()),
            "--language" => language = Some(PluginLanguage::parse(value)?),
            "--runtime" => runtime = Some(parse_runtime(value)?),
            option => bail!("未知插件初始化选项: {option}"),
        }
        index += 2;
    }
    let language = language.unwrap_or_default();
    let template = PluginTemplate::resolve(language, runtime)?;
    Ok(RepositoryPluginOptions {
        path: PathBuf::from(path),
        name,
        title,
        template,
    })
}

fn parse_plugin_source(arguments: &[String]) -> Result<PluginSource> {
    let Some(git) = arguments.first() else {
        bail!("缺少要安装的 Git 地址");
    };
    let mut rev = None;
    match &arguments[1..] {
        [] => {}
        [option, value] if option == "--rev" => rev = Some(value.clone()),
        _ => bail!("插件安装只接受可选的 --rev <版本>"),
    }
    Ok(PluginSource {
        git: git.clone(),
        rev,
    })
}

fn parse_plugin_publish_arguments(arguments: &[String]) -> Result<PublicationOptions> {
    let mut root = None;
    let mut git = None;
    let mut revision = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--git" | "--rev" => {
                let value = arguments
                    .get(index + 1)
                    .ok_or_else(|| anyhow::anyhow!("选项 {} 缺少值", arguments[index]))?;
                match arguments[index].as_str() {
                    "--git" => git = Some(value.clone()),
                    "--rev" => revision = Some(value.clone()),
                    _ => unreachable!(),
                }
                index += 2;
            }
            value if !value.starts_with('-') && root.is_none() => {
                root = Some(PathBuf::from(value));
                index += 1;
            }
            option => bail!("未知插件发布选项: {option}"),
        }
    }
    Ok(PublicationOptions {
        root: root.unwrap_or(env::current_dir()?),
        git,
        revision,
    })
}

fn exactly_one<'a>(arguments: &'a [String], missing: &str) -> Result<&'a str> {
    match arguments {
        [] => bail!("{missing}"),
        [value] => Ok(value),
        _ => bail!("命令只接受一个参数"),
    }
}

fn no_arguments(action_arguments: &[String], action: impl FnOnce() -> Result<()>) -> Result<()> {
    if !action_arguments.is_empty() {
        bail!("命令不接受参数");
    }
    action()
}

fn print_usage() {
    println!("{}", usage());
}

fn is_help(argument: &str) -> bool {
    matches!(argument, "--help" | "-h")
}

fn usage() -> &'static str {
    "用法:\n  aio init <目录> [--name <包名>] [--title <标题>]\n  aio plugin init <目录> [--name <包名>] [--title <插件标题>] [--language <rust|kotlin|typescript>]\n  aio plugin init --help\n  aio plugin install <git> [--rev <分支、标签或提交>]\n  aio plugin publish [<仓库目录>] [--git <HTTPS Git>] [--rev <完整提交 SHA>]\n  aio plugin uninstall <git>\n  aio plugin sync\n  aio plugin list\n  aio plugin validate [<仓库目录>]\n  aio plugin schema [<输出目录>]\n  aio marketplace build [<registry> <output>]"
}

fn plugin_init_usage() -> &'static str {
    "用法:\n  aio plugin init <目录> [--name <包名>] [--title <插件标题>] [--language <rust|kotlin|typescript>]\n\n自动选择:\n  未指定语言 / rust   Rust 源码插件，由 trait + Dill/TypeId 自动聚合\n  kotlin              Kotlin 服务\n  typescript          TypeScript Wasm Component\n\n高级模板覆盖（仅 Kotlin/TypeScript）:\n  --runtime page-definition   静态 PageDefinition\n  --runtime wasm-component    可在线替换的 Wasm Component\n  --runtime process           JVM/Node 隔离服务\n\n示例:\n  aio plugin init hello\n  aio plugin init orders --language kotlin\n  aio plugin init dashboard --language typescript\n  aio plugin init reports --language kotlin --runtime page-definition\n  aio plugin init worker --language typescript --runtime process"
}

fn plugin_publish_usage() -> &'static str {
    "用法:\n  aio plugin publish [<仓库目录>] [--git <HTTPS Git>] [--rev <完整提交 SHA>]\n\n环境变量:\n  AIO_PLUGIN_PUBLISH_URL     AIO 宿主发布接口\n  AIO_PLUGIN_PUBLISH_TOKEN   与租户和 Git 来源绑定的发布凭证\n\nGitHub Actions 会自动读取 GITHUB_SERVER_URL、GITHUB_REPOSITORY 和 GITHUB_SHA。发布前会校验仓库清单和 artifact，并确认两者与完整提交 SHA 中的字节完全一致。"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multilingual_plugin_template_options() -> Result<()> {
        let options = parse_plugin_init_arguments(&[
            "plugin".to_owned(),
            "--language".to_owned(),
            "kotlin".to_owned(),
            "--runtime".to_owned(),
            "process".to_owned(),
        ])?;

        assert_eq!(options.path, PathBuf::from("plugin"));
        assert_eq!(options.template, PluginTemplate::KotlinService);
        Ok(())
    }

    #[test]
    fn derives_runtime_from_language() -> Result<()> {
        let options = parse_plugin_init_arguments(&[
            "plugin".to_owned(),
            "--language".to_owned(),
            "typescript".to_owned(),
        ])?;

        assert_eq!(options.template, PluginTemplate::TypeScriptComponent);
        Ok(())
    }

    #[test]
    fn rejects_language_runtime_mismatch() {
        let result = parse_plugin_init_arguments(&[
            "plugin".to_owned(),
            "--language".to_owned(),
            "rust".to_owned(),
            "--runtime".to_owned(),
            "process".to_owned(),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_redundant_runtime_for_rust_plugin() {
        let result = parse_plugin_init_arguments(&[
            "plugin".to_owned(),
            "--runtime".to_owned(),
            "process".to_owned(),
        ]);

        let error = result.expect_err("Rust 插件不应要求运行目标");
        assert!(error.to_string().contains("trait"));
        assert!(error.to_string().contains("TypeId"));
    }

    #[test]
    fn primary_usage_hides_runtime_override() {
        let primary_command = usage()
            .lines()
            .find(|line| line.starts_with("  aio plugin init <"))
            .expect("主帮助应展示插件初始化命令");
        assert!(!primary_command.contains("--runtime"));
        assert!(plugin_init_usage().contains("高级模板覆盖"));
        assert!(plugin_init_usage().contains("trait + Dill/TypeId"));
    }

    #[test]
    fn detailed_help_matches_runtime_override_parser() {
        for runtime in ["page-definition", "wasm-component", "process"] {
            assert!(parse_runtime(runtime).is_ok());
            let option = format!("--runtime {runtime}");
            assert!(plugin_init_usage().contains(option.as_str()));
        }
        assert!(parse_runtime("rust-source").is_err());
        assert!(!plugin_init_usage().contains("rust-source"));
    }

    #[test]
    fn plain_help_can_be_used_as_plugin_directory() {
        assert!(!is_help("help"));
        assert!(is_help("--help"));
        assert!(is_help("-h"));
    }

    #[test]
    fn parses_plugin_publish_defaults_and_overrides() -> Result<()> {
        let options = parse_plugin_publish_arguments(&[
            "plugin".to_owned(),
            "--git".to_owned(),
            "https://example.com/plugin.git".to_owned(),
            "--rev".to_owned(),
            "a".repeat(40),
        ])?;
        assert_eq!(options.root, PathBuf::from("plugin"));
        assert_eq!(
            options.git.as_deref(),
            Some("https://example.com/plugin.git")
        );
        assert_eq!(options.revision, Some("a".repeat(40)));
        assert!(plugin_publish_usage().contains("GITHUB_SHA"));
        Ok(())
    }
}
