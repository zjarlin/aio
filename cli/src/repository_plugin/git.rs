use std::{path::Path, process::Command};

use anyhow::{Context as _, Result, bail};

pub fn checkout(source: &str, revision: Option<&str>, target: &Path) -> Result<String> {
    if target.exists() {
        if !target.join(".git").is_dir() {
            bail!("插件缓存不是 Git 仓库: {}", target.display());
        }
        let origin = run(target, ["remote", "get-url", "origin"])?;
        if origin.trim() != source {
            bail!("插件缓存来源不一致: {} != {source}", origin.trim());
        }
    } else {
        let parent = target.parent().context("插件缓存目录缺少父目录")?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建插件缓存目录失败: {}", parent.display()))?;
        let target_text = target.to_str().context("插件缓存路径不是有效的 Unicode")?;
        run(
            parent,
            [
                "clone",
                "--quiet",
                "--no-checkout",
                "--",
                source,
                target_text,
            ],
        )?;
    }

    match revision {
        Some(revision) => {
            run(
                target,
                ["fetch", "--quiet", "--tags", "origin", "--", revision],
            )?;
        }
        None => {
            run(target, ["fetch", "--quiet", "origin"])?;
        }
    }
    run(target, ["checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
    run(target, ["rev-parse", "HEAD"])
}

fn run<'a>(directory: &Path, arguments: impl IntoIterator<Item = &'a str>) -> Result<String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    let output = Command::new("git")
        .args(&arguments)
        .current_dir(directory)
        .output()
        .with_context(|| format!("执行 git 失败: git {}", arguments.join(" ")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        bail!("git {} 失败: {}", arguments.join(" "), message.trim());
    }
    String::from_utf8(output.stdout).context("git 输出不是有效的 UTF-8")
}
