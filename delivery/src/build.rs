use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, ensure};
use az_plugin_delivery::{BuildJob, Documentation};
use sha2::{Digest, Sha256};

use crate::{Worker, documents};

#[derive(Debug)]
pub struct Retryable;
impl std::fmt::Display for Retryable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("源码网络操作失败，等待重试")
    }
}
impl std::error::Error for Retryable {}

fn checked(command: &mut Command) -> Result<()> {
    let status = command
        .stdin(Stdio::null())
        .status()
        .context("执行构建工具失败")?;
    ensure!(status.success(), "构建工具返回 {status}");
    Ok(())
}

pub fn execute(worker: &Worker, job: &BuildJob, root: &Path) -> Result<Documentation> {
    fs::create_dir_all(root)?;
    let archive = root.join("plugin.aio-plugin");
    let source = root.join("source");
    let documentation;
    if !archive.exists() {
        if source.exists() {
            fs::remove_dir_all(&source)?;
        }
        checked(
            Command::new("git")
                .args([
                    "clone",
                    "--no-checkout",
                    "--filter=blob:none",
                    "--",
                    &job.git,
                ])
                .arg(&source),
        )
        .context(Retryable)?;
        checked(Command::new("git").arg("-C").arg(&source).args([
            "checkout",
            "--detach",
            &job.source_revision,
        ]))
        .context(Retryable)?;
        documentation = documents::collect(&source)?;
        fs::write(
            root.join("documentation.json"),
            serde_json::to_vec(&documentation)?,
        )?;
        let image = env::var(job.recipe.environment.image_variable()).context("未配置构建镜像")?;
        ensure!(
            (image.contains("@sha256:") || image.starts_with("sha256:"))
                && !image.chars().any(char::is_whitespace),
            "构建镜像必须固定 digest"
        );
        let cache = worker.root.join("cache").join(format!(
            "{:x}",
            Sha256::digest(format!("{}:{image}", job.git))
        ));
        fs::create_dir_all(&cache)?;
        fs::create_dir_all(source.join("target"))?;
        fs::create_dir_all(cache.join("target"))?;
        checked(
            Command::new("chown")
                .args(["-R", "65534:65534"])
                .arg(&source)
                .arg(&cache),
        )?;
        let name = format!("aio-build-{}", job.id);
        let _ = Command::new("docker").args(["rm", "-f", &name]).output();
        let log_path = root.join("build.log");
        let log = fs::File::create(&log_path)?;
        let mut process = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--name",
                &name,
                "--cpus",
                "4",
                "--memory",
                "8g",
                "--memory-swap",
                "8g",
                "--pids-limit",
                "512",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges",
                "--user",
                "65534:65534",
                "--read-only",
                "--tmpfs",
                "/tmp:rw,exec,size=2g",
                "--workdir",
                "/source",
                "--env",
                "HOME=/cache",
                "--env",
                "CARGO_HOME=/cache/cargo",
                "--env",
                "KOTLIN_CLI_NO_WELCOME_BANNER=1",
            ])
            .arg("--mount")
            .arg(format!("type=bind,src={},dst=/source", source.display()))
            .arg("--mount")
            .arg(format!("type=bind,src={},dst=/cache", cache.display()))
            .arg("--mount")
            .arg(format!(
                "type=bind,src={},dst=/source/target",
                cache.join("target").display()
            ))
            .arg(image)
            .args(&job.recipe.command)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait()? {
                break status;
            }
            if start.elapsed() > Duration::from_secs(2700) {
                let _ = Command::new("docker").args(["rm", "-f", &name]).status();
                let _ = process.kill();
                let _ = process.wait();
                anyhow::bail!("构建超过 45 分钟");
            }
            let heartbeat = worker
                .request(&format!("/api/internal/delivery/jobs/{}/heartbeat", job.id))
                .json(&serde_json::json!({"lease": job.lease}))
                .send();
            if heartbeat.as_ref().is_ok_and(|r| r.status().as_u16() == 409) {
                let _ = Command::new("docker").args(["rm", "-f", &name]).status();
                let _ = process.wait();
                anyhow::bail!("构建已被较新提交替代");
            }
            thread::sleep(Duration::from_secs(15));
        };
        let log = fs::read_to_string(&log_path).unwrap_or_default();
        let tail: String = log
            .chars()
            .rev()
            .take(12000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        ensure!(status.success(), "构建失败 ({status}):\n{tail}");
        checked(Command::new("chown").args(["-R", "0:0"]).arg(&source))?;
        checked(
            Command::new(env::var("AIO_CLI").unwrap_or_else(|_| "aio".into()))
                .args(["plugin", "package"])
                .arg(&source)
                .args(["--git", &job.git, "--version", &job.version, "-o"])
                .arg(&archive),
        )?;
    } else {
        documentation = serde_json::from_slice(&fs::read(root.join("documentation.json"))?)?;
    }
    worker
        .request(&format!("/api/internal/delivery/jobs/{}/package", job.id))
        .header("x-aio-build-lease", &job.lease)
        .header("content-type", "application/vnd.aio.plugin+gzip")
        .body(fs::read(&archive)?)
        .send()?
        .error_for_status()?;
    Ok(documentation)
}
