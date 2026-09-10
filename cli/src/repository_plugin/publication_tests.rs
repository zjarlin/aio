use std::{
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
};

use anyhow::{Result, anyhow};

use super::*;

#[test]
fn posts_raw_binary_package_and_polls_activation() -> Result<()> {
    let package = package()?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!(
        "http://{}/api/runtime/plugins/publish",
        listener.local_addr()?
    );
    let expected = package.clone();
    let server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = listener.accept()?;
        let (headers, body) = read_request(&mut stream)?;
        assert!(headers.starts_with("POST /api/runtime/plugins/publish HTTP/1.1\r\n"));
        let headers = headers.to_ascii_lowercase();
        assert!(headers.contains("content-type: application/vnd.aio.plugin+gzip\r\n"));
        assert!(!headers.contains("content-encoding:"));
        assert!(headers.contains("authorization: bearer test-token\r\n"));
        assert_eq!(PluginPackage::decode(&body)?, expected);
        respond(&mut stream, &expected.rev, "queued")?;
        let (mut stream, _) = listener.accept()?;
        let (headers, body) = read_request(&mut stream)?;
        assert!(headers.starts_with("GET /api/runtime/publish-jobs/job-1 HTTP/1.1\r\n"));
        assert!(body.is_empty());
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
        respond(&mut stream, &expected.rev, "active")
    });
    let active = publish_to(&package, &endpoint, "test-token")?;
    assert_eq!(active.state, PublishState::Active);
    assert_eq!(active.page_count, 1);
    server.join().map_err(|_| anyhow!("发布测试线程失败"))??;
    Ok(())
}

#[test]
fn rejects_wrong_revision_and_reports_activation_failure() -> Result<()> {
    for (revision, state, expected_error) in [
        ("0".repeat(64), "active", "不一致的内容版本"),
        (package()?.rev, "failed", "插件发布失败"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let endpoint = format!(
            "http://{}/api/runtime/plugins/publish",
            listener.local_addr()?
        );
        let server = thread::spawn(move || -> Result<()> {
            let (mut stream, _) = listener.accept()?;
            read_request(&mut stream)?;
            respond(&mut stream, &revision, state)
        });
        let error = publish_to(&package()?, &endpoint, "test-token")
            .err()
            .context("无效响应必须失败")?;
        assert!(error.to_string().contains(expected_error));
        server.join().map_err(|_| anyhow!("发布测试线程失败"))??;
    }
    Ok(())
}

#[test]
fn accepts_only_https_or_loopback_endpoints_and_safe_job_paths() -> Result<()> {
    assert!(publish_url("http://example.com/api/runtime/plugins/publish").is_err());
    assert!(publish_url("https://example.com/other").is_err());
    assert!(publish_url("https://a:b@example.com/api/runtime/plugins/publish").is_err());
    assert!(publish_url("http://[::1]:8080/api/runtime/plugins/publish").is_ok());
    let endpoint = publish_url("http://127.0.0.1:8080/api/runtime/plugins/publish")?;
    assert!(publish_job_url(endpoint.clone(), "../tokens").is_err());
    assert_eq!(
        publish_job_url(endpoint, "job-42")?.as_str(),
        "http://127.0.0.1:8080/api/runtime/publish-jobs/job-42"
    );
    Ok(())
}

fn package() -> Result<PluginPackage> {
    PluginPackage::new(
        "https://example.com/plugin.git".to_owned(), "1.0.0".to_owned(), None,
        "[plugin.runtime]\nkind='page-definition'\nartifact='pages.json'\n[plugin.marketplace]\ntitle='Pages'\nsummary='Demo pages'\nlicense='MIT'\ntags=['test']\n".to_owned(),
        b"[]",
    )
}

fn read_request(stream: &mut TcpStream) -> Result<(String, Vec<u8>)> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
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
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>())
                })
                .transpose()?
                .unwrap_or(0);
            break (header_end, content_length);
        }
    };
    while request.len() < header_end + content_length {
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk)?;
        ensure!(read > 0, "发布测试请求正文提前结束");
        request.extend_from_slice(&chunk[..read]);
    }
    Ok((
        String::from_utf8(request[..header_end].to_vec())?,
        request[header_end..header_end + content_length].to_vec(),
    ))
}

fn respond(stream: &mut TcpStream, revision: &str, state: &str) -> Result<()> {
    let response = serde_json::json!({"data": {"job_id": "job-1", "revision": revision, "page_count": 1, "state": state, "detail": "health result"}}).to_string();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
        response.len()
    )?;
    Ok(())
}
