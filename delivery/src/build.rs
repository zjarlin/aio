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
    let mut child = command
        .stdin(Stdio::null())
        .spawn()
        .context("执行构建工具失败")?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("构建准备工具超过 3 分钟");
        }
        thread::sleep(Duration::from_millis(100));
    };
    ensure!(status.success(), "构建工具返回 {status}");
    Ok(())
}

pub fn execute(worker: &Worker, job: &BuildJob, root: &Path) -> Result<Documentation> {
    fs::create_dir_all(root)?;
    let archive = root.join("plugin.aio-plugin");
    let source = root.join("source");
    let documentation;
    if !archive.exists() {
        let image = env::var(job.recipe.environment.image_variable()).context("未配置构建镜像")?;
        ensure!(
            (image.contains("@sha256:") || image.starts_with("sha256:"))
                && !image.chars().any(char::is_whitespace),
            "构建镜像必须固定 digest"
        );
        if source.exists() {
            fs::remove_dir_all(&source)?;
        }
        fs::create_dir_all(&source)?;
        checked(Command::new("chown").arg("65534:65534").arg(&source))?;
        let fetch_name = format!("aio-fetch-{}", job.id);
        for args in [
            vec!["git", "init", "."],
            vec![
                "git",
                "fetch",
                "--depth=1",
                "--",
                &job.git,
                &job.source_revision,
            ],
            vec!["git", "checkout", "--detach", "FETCH_HEAD"],
        ] {
            let _ = Command::new("docker")
                .args(["rm", "-f", &fetch_name])
                .output();
            let result = checked(container(&fetch_name, &source)?.arg(&image).args(args));
            if result.is_err() {
                let _ = Command::new("docker")
                    .args(["rm", "-f", &fetch_name])
                    .output();
            }
            result.context(Retryable)?;
        }
        documentation = documents::collect(&source)?;
        fs::write(
            root.join("documentation.json"),
            serde_json::to_vec(&documentation)?,
        )?;
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
        let mut process = container(&name, &source)?
            .args([
                "--env",
                "HOME=/cache",
                "--env",
                "CARGO_HOME=/cache/cargo",
                "--env",
                "CARGO_NET_GIT_FETCH_WITH_CLI=false",
                "--env",
                "CARGO_UNSTABLE_GIT=shallow-deps",
                "--env",
                "CARGO_HTTP_TIMEOUT=30",
                "--env",
                "CARGO_NET_RETRY=3",
                "--env",
                "KOTLIN_CLI_NO_WELCOME_BANNER=1",
            ])
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
                .timeout(Duration::from_secs(10))
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
        if !status.success() {
            let error = anyhow::anyhow!("构建失败 ({status}):\n{tail}");
            if [
                "Could not resolve host",
                "Failed to connect to",
                "Connection timed out",
                "Timeout was reached",
                "Operation too slow",
                "failed to download from",
                "EAI_AGAIN",
                "ETIMEDOUT",
                "ConnectTimeoutError",
                "SocketTimeoutException",
            ]
            .iter()
            .any(|message| tail.contains(message))
            {
                return Err(error.context(Retryable));
            }
            return Err(error);
        }
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

fn container(name: &str, source: &Path) -> Result<Command> {
    let mut command = Command::new("docker");
    command.args([
        "run",
        "--rm",
        "--name",
        name,
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
        "GIT_CONFIG_COUNT=2",
        "--env",
        "GIT_CONFIG_KEY_0=http.lowSpeedLimit",
        "--env",
        "GIT_CONFIG_VALUE_0=100",
        "--env",
        "GIT_CONFIG_KEY_1=http.lowSpeedTime",
        "--env",
        "GIT_CONFIG_VALUE_1=30",
    ]);
    if let Ok(dns) = env::var("AIO_BUILD_DNS") {
        let dns: std::net::IpAddr = dns.parse().context("无效构建 DNS 地址")?;
        command.args(["--dns", &dns.to_string()]);
    }
    command
        .arg("--mount")
        .arg(format!("type=bind,src={},dst=/source", source.display()));
    Ok(command)
}
