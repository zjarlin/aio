use std::{env, error::Error, path::PathBuf, process};

use app_lib::{
    extension_host::ExtensionHostRuntime,
    plugin_factory::{
        create_from_prompt_and_write, publish_gate, publish_local, repair_from_diagnostics,
        verify_plugin_draft, PluginCreateFromPromptInput, PluginRepairFromDiagnosticsInput,
        PluginVerifyDraftInput,
    },
    plugin_registry::{default_project_root, registry_from_project_root},
    plugin_store::{PluginRegistryRollbackInput, PluginRegistryStore},
    web_bridge,
};
use serde_json::json;

fn main() {
    let exit_code = match run() {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    };
    process::exit(exit_code);
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        print_usage();
        return Ok(1);
    };

    match command {
        "validate" => {
            let root = parse_root(&args[1..])?;
            let snapshot = registry_from_project_root(&root)?;
            let ok = snapshot.diagnostics.is_empty();
            let output = json!({
                "status": if ok { "ok" } else { "failed" },
                "root": root,
                "schemaVersion": snapshot.schema_version,
                "systemCapsules": snapshot.system_capsules.len(),
                "plugins": snapshot.plugins.len(),
                "commands": snapshot.commands.len(),
                "tools": snapshot.tools.len(),
                "views": snapshot.views.len(),
                "events": snapshot.events.len(),
                "extensionPoints": snapshot.extension_points.len(),
                "permissions": snapshot.permissions.len(),
                "capabilities": snapshot.capabilities.len(),
                "policies": snapshot.policies.len(),
                "diagnostics": snapshot.diagnostics,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(if ok { 0 } else { 2 })
        }
        "inspect" => {
            let root = parse_root(&args[1..])?;
            let snapshot = registry_from_project_root(&root)?;
            let output = json!({
                "root": root,
                "plugins": snapshot.plugins,
                "systemCapsules": snapshot.system_capsules,
                "extensionTree": snapshot.extension_tree,
                "commands": snapshot.commands,
                "tools": snapshot.tools,
                "views": snapshot.views,
                "events": snapshot.events,
                "extensionPoints": snapshot.extension_points,
                "capabilities": snapshot.capabilities,
                "policies": snapshot.policies,
                "diagnostics": snapshot.diagnostics,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(0)
        }
        "compile" => {
            let root = parse_root(&args[1..])?;
            let snapshot = registry_from_project_root(&root)?;
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
            Ok(if snapshot.diagnostics.is_empty() {
                0
            } else {
                2
            })
        }
        "create-from-prompt" => {
            let input = parse_prompt_input(&args[1..])?;
            let draft = create_from_prompt_and_write(input)?;
            println!("{}", serde_json::to_string_pretty(&draft)?);
            Ok(0)
        }
        "publish-local" => {
            let input = parse_publish_local_input(&args[1..])?;
            let record = publish_local(input.source_path, input.data_dir)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
            Ok(0)
        }
        "publish-gate" => {
            let input = parse_publish_gate_input(&args[1..])?;
            let report = publish_gate(input.source_path, input.data_dir, input.write)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(0)
        }
        "rollback-local" => {
            let input = parse_rollback_input(&args[1..])?;
            let result =
                PluginRegistryStore::new(input.data_dir).rollback(PluginRegistryRollbackInput {
                    id: input.id,
                    content_hash: input.content_hash,
                })?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(0)
        }
        "repair-from-diagnostics" => {
            let input = parse_repair_input(&args[1..])?;
            let result = repair_from_diagnostics(input)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(0)
        }
        "verify-draft" => {
            let input = parse_verify_draft_input(&args[1..])?;
            let report = verify_plugin_draft(input)?;
            let ok = report.status == "passed";
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(if ok { 0 } else { 2 })
        }
        "host-cycle" => {
            let source_path = parse_host_cycle_input(&args[1..])?;
            let records = ExtensionHostRuntime::default().lifecycle_cycle(source_path)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
            Ok(0)
        }
        "command-bridge" => {
            let input = parse_bridge_input(&args[1..])?;
            web_bridge::serve_headless(input.data_dir)?;
            Ok(0)
        }
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(0)
        }
        other => Err(format!("不支持的 aio-plugin 命令：{other}").into()),
    }
}

fn parse_root(args: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    let mut root = default_project_root();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--root 需要目录参数".into());
                };
                root = PathBuf::from(value);
                index += 2;
            }
            other => return Err(format!("不支持的 aio-plugin 参数：{other}").into()),
        }
    }
    Ok(root)
}

fn parse_prompt_input(args: &[String]) -> Result<PluginCreateFromPromptInput, Box<dyn Error>> {
    let mut prompt: Option<String> = None;
    let mut display_name = None;
    let mut id = None;
    let mut kind = None;
    let mut parent_plugin_id = None;
    let mut parent_mount = None;
    let mut route_path = None;
    let mut output_dir = None;
    let mut force = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--prompt" => {
                prompt = Some(parse_value(args, &mut index, "--prompt")?);
            }
            "--display-name" => {
                display_name = Some(parse_value(args, &mut index, "--display-name")?);
            }
            "--id" => {
                id = Some(parse_value(args, &mut index, "--id")?);
            }
            "--kind" => {
                kind = Some(parse_value(args, &mut index, "--kind")?);
            }
            "--parent-plugin-id" => {
                parent_plugin_id = Some(parse_value(args, &mut index, "--parent-plugin-id")?);
            }
            "--parent-mount" => {
                parent_mount = Some(parse_value(args, &mut index, "--parent-mount")?);
            }
            "--route-path" => {
                route_path = Some(parse_value(args, &mut index, "--route-path")?);
            }
            "--output-dir" | "--out" => {
                output_dir = Some(parse_value(args, &mut index, "--output-dir")?);
            }
            "--force" => {
                force = true;
                index += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if prompt.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                prompt = Some(other.to_string());
                index += 1;
            }
        }
    }

    let Some(prompt) = prompt else {
        return Err("create-from-prompt 需要 prompt 参数".into());
    };

    Ok(PluginCreateFromPromptInput {
        prompt,
        display_name,
        id,
        kind,
        parent_plugin_id,
        parent_mount,
        route_path,
        output_dir,
        force,
    })
}

struct PublishLocalArgs {
    source_path: PathBuf,
    data_dir: PathBuf,
}

struct PublishGateArgs {
    source_path: PathBuf,
    data_dir: PathBuf,
    write: bool,
}

struct RollbackArgs {
    id: String,
    content_hash: Option<String>,
    data_dir: PathBuf,
}

struct BridgeArgs {
    data_dir: PathBuf,
}

fn parse_publish_local_input(args: &[String]) -> Result<PublishLocalArgs, Box<dyn Error>> {
    let mut source_path: Option<PathBuf> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--source-path" | "--source-dir" => {
                source_path = Some(PathBuf::from(parse_value(
                    args,
                    &mut index,
                    "--source-path",
                )?));
            }
            "--data-dir" => {
                data_dir = Some(PathBuf::from(parse_value(args, &mut index, "--data-dir")?));
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if source_path.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                source_path = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }

    let Some(source_path) = source_path else {
        return Err("publish-local 需要 source-path 参数".into());
    };
    let Some(data_dir) = data_dir else {
        return Err("publish-local 需要 data-dir 参数".into());
    };

    Ok(PublishLocalArgs {
        source_path,
        data_dir,
    })
}

fn parse_publish_gate_input(args: &[String]) -> Result<PublishGateArgs, Box<dyn Error>> {
    let mut source_path: Option<PathBuf> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut write = true;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--source-path" | "--source-dir" => {
                source_path = Some(PathBuf::from(parse_value(
                    args,
                    &mut index,
                    "--source-path",
                )?));
            }
            "--data-dir" => {
                data_dir = Some(PathBuf::from(parse_value(args, &mut index, "--data-dir")?));
            }
            "--no-write" => {
                write = false;
                index += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if source_path.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                source_path = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }

    let Some(source_path) = source_path else {
        return Err("publish-gate 需要 source-path 参数".into());
    };
    let Some(data_dir) = data_dir else {
        return Err("publish-gate 需要 data-dir 参数".into());
    };

    Ok(PublishGateArgs {
        source_path,
        data_dir,
        write,
    })
}

fn parse_rollback_input(args: &[String]) -> Result<RollbackArgs, Box<dyn Error>> {
    let mut id: Option<String> = None;
    let mut content_hash = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--id" => {
                id = Some(parse_value(args, &mut index, "--id")?);
            }
            "--content-hash" => {
                content_hash = Some(parse_value(args, &mut index, "--content-hash")?);
            }
            "--data-dir" => {
                data_dir = Some(PathBuf::from(parse_value(args, &mut index, "--data-dir")?));
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if id.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                id = Some(other.to_string());
                index += 1;
            }
        }
    }

    let Some(id) = id else {
        return Err("rollback-local 需要 id 参数".into());
    };
    let Some(data_dir) = data_dir else {
        return Err("rollback-local 需要 data-dir 参数".into());
    };

    Ok(RollbackArgs {
        id,
        content_hash,
        data_dir,
    })
}

fn parse_repair_input(args: &[String]) -> Result<PluginRepairFromDiagnosticsInput, Box<dyn Error>> {
    let mut diagnostics_path: Option<String> = None;
    let mut source_path = None;
    let mut force = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--diagnostics-path" => {
                diagnostics_path = Some(parse_value(args, &mut index, "--diagnostics-path")?);
            }
            "--source-path" => {
                source_path = Some(parse_value(args, &mut index, "--source-path")?);
            }
            "--force" => {
                force = true;
                index += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if diagnostics_path.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                diagnostics_path = Some(other.to_string());
                index += 1;
            }
        }
    }

    let Some(diagnostics_path) = diagnostics_path else {
        return Err("repair-from-diagnostics 需要 diagnostics-path 参数".into());
    };

    Ok(PluginRepairFromDiagnosticsInput {
        diagnostics_path,
        source_path,
        force,
    })
}

fn parse_verify_draft_input(args: &[String]) -> Result<PluginVerifyDraftInput, Box<dyn Error>> {
    let mut source_path: Option<String> = None;
    let mut output_dir = None;
    let mut write = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--source-path" | "--source-dir" => {
                source_path = Some(parse_value(args, &mut index, "--source-path")?);
            }
            "--output-dir" | "--out" => {
                output_dir = Some(parse_value(args, &mut index, "--output-dir")?);
            }
            "--write" => {
                write = true;
                index += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if source_path.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                source_path = Some(other.to_string());
                index += 1;
            }
        }
    }

    let Some(source_path) = source_path else {
        return Err("verify-draft 需要 source-path 参数".into());
    };

    Ok(PluginVerifyDraftInput {
        source_path,
        output_dir,
        write,
    })
}

fn parse_host_cycle_input(args: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    let mut source_path: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source-path" | "--source-dir" => {
                source_path = Some(PathBuf::from(parse_value(
                    args,
                    &mut index,
                    "--source-path",
                )?));
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if source_path.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                source_path = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }
    source_path.ok_or_else(|| "host-cycle 需要 source-path 参数".into())
}

fn parse_bridge_input(args: &[String]) -> Result<BridgeArgs, Box<dyn Error>> {
    let mut data_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--data-dir" => {
                data_dir = Some(PathBuf::from(parse_value(args, &mut index, "--data-dir")?));
            }
            other if other.starts_with('-') => {
                return Err(format!("不支持的 aio-plugin 参数：{other}").into());
            }
            other => {
                if data_dir.is_some() {
                    return Err(format!("多余的位置参数：{other}").into());
                }
                data_dir = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }
    let Some(data_dir) = data_dir else {
        return Err("command-bridge 需要 data-dir 参数".into());
    };
    Ok(BridgeArgs { data_dir })
}

fn parse_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, Box<dyn Error>> {
    let Some(value) = args.get(*index + 1) else {
        return Err(format!("{flag} 需要参数").into());
    };
    *index += 2;
    Ok(value.clone())
}

fn print_usage() {
    eprintln!(
        "用法: aio-plugin <validate|inspect|compile> [--root <repo-root>]\n\
         aio-plugin create-from-prompt --prompt <text> [--display-name <name>] [--id <plugin-id>] [--kind <plugin|child-plugin>] [--parent-plugin-id <id>] [--parent-mount <mount>] [--route-path <path>] [--output-dir <dir>] [--force]\n\
         aio-plugin publish-gate --source-path <dir> --data-dir <dir> [--no-write]\n\
         aio-plugin publish-local --source-path <dir> --data-dir <dir>\n\
         aio-plugin rollback-local --id <plugin-id> --data-dir <dir> [--content-hash <hash>]\n\
         aio-plugin repair-from-diagnostics --diagnostics-path <file> [--source-path <dir>] [--force]\n\
         aio-plugin verify-draft --source-path <dir> [--output-dir <dir>] [--write]\n\
         aio-plugin host-cycle --source-path <dir>\n\
         aio-plugin command-bridge --data-dir <dir>\n\
         默认 root 来自 AIO_PLUGIN_PLATFORM_ROOT，未设置时使用当前 Cargo 工程的上级目录。"
    );
}
