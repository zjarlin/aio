use std::{
    env, fs,
    io::{Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use anyhow::{Context as _, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::{Compression, write::GzEncoder};
use reqwest::{
    Url,
    blocking::{Client, Response},
    header,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MANIFEST_FILE: &str = "aio-plugin.toml";
const PUBLISH_URL_ENV: &str = "AIO_PLUGIN_PUBLISH_URL";
const PUBLISH_TOKEN_ENV: &str = "AIO_PLUGIN_PUBLISH_TOKEN";
const POLL_ATTEMPTS: usize = 90;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_RESPONSE_BYTES: u64 = 128 * 1024;

pub struct PublicationOptions {
    pub root: PathBuf,
    pub git: Option<String>,
    pub revision: Option<String>,
}

pub fn publish(options: PublicationOptions) -> Result<()> {
    let publication = prepare_publication(options)?;
    let endpoint = env::var(PUBLISH_URL_ENV)
        .with_context(|| format!("缺少 {PUBLISH_URL_ENV}，无法发布插件"))?;
    let token = env::var(PUBLISH_TOKEN_ENV)
        .with_context(|| format!("缺少 {PUBLISH_TOKEN_ENV}，无法发布插件"))?;
    ensure!(!token.trim().is_empty(), "{PUBLISH_TOKEN_ENV} 不能为空");
    let active = publish_to(&publication, &endpoint, &token)?;
    println!(
        "插件发布已激活: revision={} pages={} detail={}",
        active.revision, active.page_count, active.detail
    );
    Ok(())
}

struct PreparedPublication {
    git: String,
    revision: String,
    manifest_toml: String,
    artifact_base64: String,
    artifact_sha256: String,
}

fn prepare_publication(options: PublicationOptions) -> Result<PreparedPublication> {
    let root = options
        .root
        .canonicalize()
        .with_context(|| format!("解析插件仓库目录失败: {}", options.root.display()))?;
    let repository_root = PathBuf::from(git_text(&root, &["rev-parse", "--show-toplevel"])?);
    ensure!(
        repository_root.canonicalize()? == root,
        "发布目录必须是独立插件 Git 仓库根目录"
    );

    let report = az_plugin_manifest::validate_repository(&root)?;
    let artifact_path = report
        .artifact
        .context("Rust 源码插件不通过在线 artifact 发布")?;
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest_bytes = fs::read(&manifest_path)
        .with_context(|| format!("读取插件清单失败: {}", manifest_path.display()))?;
    let manifest_toml =
        String::from_utf8(manifest_bytes.clone()).context("aio-plugin.toml 必须使用 UTF-8 编码")?;
    let manifest = az_plugin_manifest::parse_manifest(&manifest_toml)?;
    ensure!(
        manifest.plugin.marketplace.is_some(),
        "在线发布插件必须声明 [plugin.marketplace]"
    );
    let runtime = manifest
        .plugin
        .runtime
        .as_ref()
        .context("在线发布插件必须声明 [plugin.runtime]")?;

    let head = git_text(&root, &["rev-parse", "HEAD"])?;
    let revision = options
        .revision
        .or_else(|| env::var("GITHUB_SHA").ok())
        .unwrap_or_else(|| head.clone());
    ensure!(is_full_revision(&revision), "发布插件必须使用完整提交 SHA");
    ensure!(
        revision.eq_ignore_ascii_case(&head),
        "发布 revision 必须等于当前仓库 HEAD: {head}"
    );
    ensure_committed_bytes(&root, &revision, MANIFEST_FILE, &manifest_bytes, "插件清单")?;

    let artifact_bytes = fs::read(&artifact_path)
        .with_context(|| format!("读取插件 artifact 失败: {}", artifact_path.display()))?;
    ensure_committed_bytes(
        &root,
        &revision,
        &runtime.artifact,
        &artifact_bytes,
        "插件 artifact",
    )?;
    let git = resolve_git_source(&root, options.git)?;
    let artifact_sha256 = format!("{:x}", Sha256::digest(&artifact_bytes));
    Ok(PreparedPublication {
        git,
        revision,
        manifest_toml,
        artifact_base64: STANDARD.encode(artifact_bytes),
        artifact_sha256,
    })
}

fn resolve_git_source(root: &Path, configured: Option<String>) -> Result<String> {
    let source = configured
        .or_else(github_source)
        .map(Ok)
        .unwrap_or_else(|| git_text(root, &["remote", "get-url", "origin"]))?;
    normalize_git_source(&source)
}

fn github_source() -> Option<String> {
    let server = env::var("GITHUB_SERVER_URL").ok()?;
    let repository = env::var("GITHUB_REPOSITORY").ok()?;
    Some(format!(
        "{}/{}.git",
        server.trim_end_matches('/'),
        repository.trim_matches('/')
    ))
}

fn normalize_git_source(source: &str) -> Result<String> {
    let mut url = Url::parse(source).context("插件 Git 来源不是有效 URL")?;
    ensure!(
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "在线发布只接受无凭证、无查询参数的 HTTPS Git 来源"
    );
    let path = url.path().trim_end_matches('/');
    ensure!(path != "/" && !path.is_empty(), "插件 Git 来源缺少仓库路径");
    if !path.ends_with(".git") {
        url.set_path(&format!("{path}.git"));
    }
    Ok(url.into())
}

fn ensure_committed_bytes(
    root: &Path,
    revision: &str,
    relative: &str,
    current: &[u8],
    label: &str,
) -> Result<()> {
    let object = format!("{revision}:{relative}");
    let committed = git_output(root, &["cat-file", "blob", &object])?;
    ensure!(
        committed == current,
        "{label} 与提交 {revision} 中的字节不一致"
    );
    Ok(())
}

fn is_full_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Serialize)]
struct PublishPluginRequest<'a> {
    git: &'a str,
    rev: &'a str,
    manifest_toml: &'a str,
    artifact_base64: &'a str,
    artifact_sha256: &'a str,
}

#[derive(Deserialize)]
struct RuntimeResponse<T> {
    data: T,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PublishState {
    Queued,
    Running,
    Active,
    Failed,
}

#[derive(Deserialize)]
struct PublishedPlugin {
    job_id: String,
    revision: String,
    page_count: usize,
    state: PublishState,
    detail: String,
}

fn publish_to(
    publication: &PreparedPublication,
    endpoint: &str,
    token: &str,
) -> Result<PublishedPlugin> {
    let endpoint = publish_url(endpoint)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("创建插件发布客户端失败")?;
    let body = encode_request(publication)?;
    let response = client
        .post(endpoint.clone())
        .bearer_auth(token)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(body)
        .send()
        .context("请求插件发布接口失败")?;
    let mut status = decode_response(response)?;
    ensure!(
        status.revision.eq_ignore_ascii_case(&publication.revision),
        "发布接口返回了不一致的 revision"
    );
    let job_url = publish_job_url(endpoint, &status.job_id)?;
    for attempt in 0..=POLL_ATTEMPTS {
        match status.state {
            PublishState::Active => return Ok(status),
            PublishState::Failed => bail!("插件发布失败: {}", status.detail),
            PublishState::Queued | PublishState::Running => {}
        }
        ensure!(attempt < POLL_ATTEMPTS, "等待插件发布激活超时");
        thread::sleep(POLL_INTERVAL);
        let response = client
            .get(job_url.clone())
            .bearer_auth(token)
            .send()
            .context("查询插件发布任务失败")?;
        status = decode_response(response)?;
        ensure!(
            status.revision.eq_ignore_ascii_case(&publication.revision),
            "发布任务返回了不一致的 revision"
        );
    }
    unreachable!()
}

fn encode_request(publication: &PreparedPublication) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(&PublishPluginRequest {
        git: &publication.git,
        rev: &publication.revision,
        manifest_toml: &publication.manifest_toml,
        artifact_base64: &publication.artifact_base64,
        artifact_sha256: &publication.artifact_sha256,
    })?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json)?;
    encoder.finish().context("压缩插件发布请求失败")
}

fn decode_response(response: Response) -> Result<PublishedPlugin> {
    let status = response.status();
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .context("读取插件发布响应失败")?;
    ensure!(body.len() as u64 <= MAX_RESPONSE_BYTES, "插件发布响应过大");
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&body);
        bail!(
            "插件发布接口返回 {status}: {}",
            detail.chars().take(4096).collect::<String>()
        );
    }
    serde_json::from_slice::<RuntimeResponse<PublishedPlugin>>(&body)
        .context("解析插件发布响应失败")
        .map(|response| response.data)
}

fn publish_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("AIO_PLUGIN_PUBLISH_URL 不是有效 URL")?;
    let loopback_http = url.scheme() == "http" && url_is_loopback(&url);
    ensure!(
        (url.scheme() == "https" || loopback_http)
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path().ends_with("/plugins/publish"),
        "AIO_PLUGIN_PUBLISH_URL 必须是 HTTPS 发布接口；仅本机开发允许 HTTP"
    );
    Ok(url)
}

fn url_is_loopback(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

fn publish_job_url(mut endpoint: Url, job_id: &str) -> Result<Url> {
    ensure!(
        !job_id.is_empty()
            && job_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
        "发布任务 ID 无效"
    );
    let base = endpoint
        .path()
        .strip_suffix("/plugins/publish")
        .context("发布接口路径无效")?;
    endpoint.set_path(&format!("{base}/publish-jobs/{job_id}"));
    Ok(endpoint)
}

fn git_text(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = git_output(root, arguments)?;
    String::from_utf8(output)
        .context("Git 输出不是有效 UTF-8")
        .map(|value| value.trim().to_owned())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .with_context(|| format!("执行 git {} 失败", arguments.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} 失败: {}",
            arguments.join(" "),
            stderr.chars().take(4096).collect::<String>().trim()
        );
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read as _, Write as _},
        net::TcpListener,
    };

    use anyhow::{Result, anyhow};
    use flate2::read::GzDecoder;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn prepares_only_committed_manifest_and_artifact_bytes() -> Result<()> {
        let repository = plugin_repository()?;
        let revision = git_text(repository.path(), &["rev-parse", "HEAD"])?;
        let publication = prepare_publication(PublicationOptions {
            root: repository.path().to_path_buf(),
            git: Some("https://example.com/community/pages".to_owned()),
            revision: Some(revision.clone()),
        })?;

        assert_eq!(publication.git, "https://example.com/community/pages.git");
        assert_eq!(publication.revision, revision);
        assert_eq!(publication.artifact_sha256.len(), 64);

        let artifact = fs::read_to_string(repository.path().join("dist/pages.json"))?;
        fs::write(
            repository.path().join("dist/pages.json"),
            artifact.replace("Committed", "Changed"),
        )?;
        let error = prepare_publication(PublicationOptions {
            root: repository.path().to_path_buf(),
            git: Some("https://example.com/community/pages.git".to_owned()),
            revision: Some(publication.revision),
        })
        .err()
        .context("修改后的 artifact 应被拒绝")?;
        assert!(error.to_string().contains("artifact 与提交"));
        Ok(())
    }

    #[test]
    fn encodes_gzip_request_and_derives_job_url() -> Result<()> {
        let publication = PreparedPublication {
            git: "https://example.com/plugin.git".to_owned(),
            revision: "a".repeat(40),
            manifest_toml: "[plugin.runtime]\nkind='page-definition'\nartifact='pages.json'\n"
                .to_owned(),
            artifact_base64: "W10=".to_owned(),
            artifact_sha256: "b".repeat(64),
        };
        let encoded = encode_request(&publication)?;
        let mut decoder = GzDecoder::new(encoded.as_slice());
        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded)?;
        let payload = serde_json::from_str::<serde_json::Value>(&decoded)?;
        assert_eq!(payload["rev"], publication.revision);
        assert_eq!(payload["artifact_base64"], "W10=");

        let endpoint = publish_url("http://127.0.0.1:8080/api/runtime/plugins/publish")?;
        assert_eq!(
            publish_job_url(endpoint, "job-42")?.as_str(),
            "http://127.0.0.1:8080/api/runtime/publish-jobs/job-42"
        );
        Ok(())
    }

    #[test]
    fn rejects_insecure_remote_publish_url() {
        assert!(publish_url("http://example.com/api/runtime/plugins/publish").is_err());
        assert!(publish_url("https://example.com/other").is_err());
    }

    #[test]
    fn posts_gzip_publication_with_bearer_token() -> Result<()> {
        let publication = PreparedPublication {
            git: "https://example.com/plugin.git".to_owned(),
            revision: "a".repeat(40),
            manifest_toml: "[plugin.runtime]\nkind='page-definition'\nartifact='pages.json'\n"
                .to_owned(),
            artifact_base64: "W10=".to_owned(),
            artifact_sha256: "b".repeat(64),
        };
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let endpoint = format!(
            "http://{}/api/runtime/plugins/publish",
            listener.local_addr()?
        );
        let revision = publication.revision.clone();
        let server = thread::spawn(move || receive_publication(listener, &revision));

        let active = publish_to(&publication, &endpoint, "test-token")?;

        assert_eq!(active.state, PublishState::Active);
        assert_eq!(active.page_count, 1);
        server
            .join()
            .map_err(|_| anyhow!("发布测试服务线程崩溃"))??;
        Ok(())
    }

    fn receive_publication(listener: TcpListener, revision: &str) -> Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = Vec::new();
        let (header_end, content_length) = loop {
            let mut chunk = [0_u8; 8192];
            let read = stream.read(&mut chunk)?;
            ensure!(read > 0, "发布测试请求提前结束");
            request.extend_from_slice(&chunk[..read]);
            ensure!(request.len() <= 1024 * 1024, "发布测试请求过大");
            if let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let header_end = header_end + 4;
                let headers = std::str::from_utf8(&request[..header_end])?;
                assert!(headers.starts_with("POST /api/runtime/plugins/publish HTTP/1.1\r\n"));
                assert!(
                    headers
                        .to_ascii_lowercase()
                        .contains("content-encoding: gzip")
                );
                assert!(
                    headers
                        .to_ascii_lowercase()
                        .contains("authorization: bearer test-token")
                );
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>())
                    })
                    .context("发布测试请求缺少 Content-Length")??;
                break (header_end, content_length);
            }
        };
        while request.len() < header_end + content_length {
            let mut chunk = [0_u8; 8192];
            let read = stream.read(&mut chunk)?;
            ensure!(read > 0, "发布测试请求正文提前结束");
            request.extend_from_slice(&chunk[..read]);
        }
        let mut decoder = GzDecoder::new(&request[header_end..header_end + content_length]);
        let mut body = String::new();
        decoder.read_to_string(&mut body)?;
        let payload = serde_json::from_str::<serde_json::Value>(&body)?;
        assert_eq!(payload["rev"], revision);
        assert_eq!(payload["artifact_base64"], "W10=");

        let response = format!(
            "{{\"data\":{{\"job_id\":\"job-1\",\"revision\":\"{revision}\",\"page_count\":1,\"state\":\"active\",\"detail\":\"activated\"}}}}"
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
            response.len()
        )?;
        Ok(())
    }

    fn plugin_repository() -> Result<tempfile::TempDir> {
        let repository = tempdir()?;
        fs::create_dir(repository.path().join("dist"))?;
        fs::write(
            repository.path().join(MANIFEST_FILE),
            "[plugin.runtime]\nkind = \"page-definition\"\nartifact = \"dist/pages.json\"\n\n[plugin.marketplace]\ntitle = \"Pages\"\nsummary = \"Committed pages\"\nlicense = \"MIT\"\ntags = [\"test\"]\n\n[[plugin.subplugins]]\nid = \"pages\"\npages = [\"page\"]\n",
        )?;
        fs::write(
            repository.path().join("dist/pages.json"),
            "[{\"id\":\"page\",\"label\":\"Page\",\"icon\":null,\"scene\":{\"id\":\"workspace\",\"label\":\"Workspace\"},\"required_permission\":null,\"body\":{\"kind\":\"text\",\"title\":\"Page\",\"content\":\"Committed\"}}]\n",
        )?;
        git(repository.path(), &["init", "--quiet"])?;
        git(
            repository.path(),
            &["config", "user.email", "test@example.com"],
        )?;
        git(repository.path(), &["config", "user.name", "AIO Test"])?;
        git(repository.path(), &["add", "--force", "."])?;
        git(repository.path(), &["commit", "--quiet", "-m", "init"])?;
        Ok(repository)
    }

    fn git(root: &Path, arguments: &[&str]) -> Result<()> {
        let output = git_output(root, arguments)?;
        ensure!(output.is_empty(), "测试 Git 命令产生了意外输出");
        Ok(())
    }
}
