use super::Preview;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};
use kuchikiki::traits::TendrilSink;
use std::{path::Component, sync::Arc};

pub async fn shell(State(state): State<Arc<Preview>>) -> Html<String> {
    Html(include_str!("shell.html").replace("__TICKET__", &state.ticket))
}

pub async fn guest() -> Response {
    javascript(include_str!("../../../../../sdk/web/guest.js"))
}
pub async fn host() -> Response {
    javascript(include_str!("../../../../../sdk/web/host.mjs"))
}

fn javascript(source: &'static str) -> Response {
    (
        [
            ("content-type", "text/javascript"),
            ("access-control-allow-origin", "*"),
        ],
        source,
    )
        .into_response()
}

pub async fn asset(
    State(state): State<Arc<Preview>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    if !std::path::Path::new(&path)
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let file = tokio::fs::canonicalize(state.assets.join(&path))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !file.starts_with(&state.assets) {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut bytes = tokio::fs::read(file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_str(
            mime_guess::from_path(&path)
                .first_or_octet_stream()
                .as_ref(),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    headers.insert(
        "cache-control",
        HeaderValue::from_static("no-store, no-transform"),
    );
    if path.ends_with(".html") {
        let document = kuchikiki::parse_html()
            .one(String::from_utf8(bytes).map_err(|_| StatusCode::BAD_REQUEST)?)
            .document_node;
        let script = kuchikiki::parse_html()
            .one("<script src=\"/bridge/guest.js\"></script>")
            .document_node;
        let node = script
            .select_first("script")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .as_node()
            .clone();
        document
            .select_first("head")
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .as_node()
            .prepend(node);
        bytes = document.to_string().into_bytes();
        let csp = format!(
            "sandbox allow-scripts; default-src 'none'; script-src {} 'unsafe-inline' 'wasm-unsafe-eval'; connect-src {}/assets/; img-src {}/assets/ data: blob:; font-src {}/assets/; style-src 'unsafe-inline'; worker-src blob:; base-uri 'none'; form-action 'none'",
            state.origin, state.origin, state.origin, state.origin
        );
        headers.insert(
            "content-security-policy",
            HeaderValue::from_str(&csp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
    }
    Ok((headers, bytes).into_response())
}
