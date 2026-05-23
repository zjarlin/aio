use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::{
    error::{AssistantError, AssistantResult},
    PageContextInput, PageContextPreview,
};

const MAX_CONTEXT_CHARS: usize = 12_000;
const MAX_HEADING_LINES: usize = 24;
const MAX_VISIBLE_LINES: usize = 120;
const USER_AGENT: &str = "aio-openai-assistant/0.1";

pub async fn preview_page_context(input: &PageContextInput) -> AssistantResult<PageContextPreview> {
    if let Some(html) = non_empty(input.html.as_deref()) {
        return build_html_preview(input, html, "html");
    }

    if let Some(text) = non_empty(input.text.as_deref()) {
        return build_text_preview(input, text);
    }

    if let Some(url) = non_empty(input.url.as_deref()) {
        let validated = validate_url(url)?;
        let html = fetch_html(validated.as_str()).await?;
        return build_html_preview(input, &html, "fetched-url");
    }

    if non_empty(input.selection.as_deref()).is_some() {
        return build_text_preview(input, "");
    }

    Err(AssistantError::EmptyContext)
}

fn build_html_preview(
    input: &PageContextInput,
    html: &str,
    source: &str,
) -> AssistantResult<PageContextPreview> {
    let document = Html::parse_document(html);
    let extracted_title = first_selector_text(&document, "title")?;
    let title = first_non_empty([input.title.as_deref(), extracted_title.as_deref()])
        .unwrap_or_else(|| "Untitled page".to_string());
    let standard_description = meta_content(&document, r#"meta[name="description"]"#)?;
    let og_description = meta_content(&document, r#"meta[property="og:description"]"#)?;
    let description = first_non_empty([standard_description.as_deref(), og_description.as_deref()]);
    let headings = collect_text_lines(&document, "h1, h2, h3, h4, h5, h6", MAX_HEADING_LINES)?;
    let visible_lines = collect_text_lines(
        &document,
        "p, li, th, td, label, button, a, pre, code, summary, figcaption",
        MAX_VISIBLE_LINES,
    )?;
    let body_fallback = if visible_text_len(&visible_lines) < 240 {
        root_text(&document, "main, [role=\"main\"], article, body")?
    } else {
        String::new()
    };

    let content = assemble_context(
        &title,
        input.url.as_deref(),
        description.as_deref(),
        source,
        input.selection.as_deref(),
        Some(&headings),
        Some(&visible_lines),
        non_empty(Some(body_fallback.as_str())),
    )?;
    let full_count = content.chars().count();
    let (content, truncated) = truncate_to_chars(&content, MAX_CONTEXT_CHARS);

    Ok(PageContextPreview {
        title,
        url: input.url.clone().filter(|value| !value.trim().is_empty()),
        description,
        source: source.to_string(),
        character_count: full_count,
        truncated,
        content,
    })
}

fn build_text_preview(input: &PageContextInput, text: &str) -> AssistantResult<PageContextPreview> {
    let normalized = normalize_multiline(text);
    let first_line = normalized.lines().find(|line| !line.trim().is_empty());
    let title = first_non_empty([input.title.as_deref(), first_line])
        .unwrap_or_else(|| "Plain text context".to_string());

    let content = assemble_context(
        &title,
        input.url.as_deref(),
        None,
        "text",
        input.selection.as_deref(),
        None,
        None,
        non_empty(Some(normalized.as_str())),
    )?;
    let full_count = content.chars().count();
    let (content, truncated) = truncate_to_chars(&content, MAX_CONTEXT_CHARS);

    Ok(PageContextPreview {
        title,
        url: input.url.clone().filter(|value| !value.trim().is_empty()),
        description: None,
        source: "text".to_string(),
        character_count: full_count,
        truncated,
        content,
    })
}

fn assemble_context(
    title: &str,
    url: Option<&str>,
    description: Option<&str>,
    source: &str,
    selection: Option<&str>,
    headings: Option<&[String]>,
    visible_lines: Option<&[String]>,
    fallback_text: Option<&str>,
) -> AssistantResult<String> {
    let mut sections = Vec::new();
    let mut meta = vec![format!("Title: {title}"), format!("Source: {source}")];
    if let Some(url) = non_empty(url) {
        meta.push(format!("URL: {}", url.trim()));
    }
    if let Some(description) = non_empty(description) {
        meta.push(format!("Description: {}", normalize_inline(description)));
    }
    sections.push(meta.join("\n"));

    if let Some(selection) = non_empty(selection) {
        sections.push(format!(
            "Selected text:\n{}",
            normalize_multiline(selection)
        ));
    }

    if let Some(headings) = headings.filter(|items| !items.is_empty()) {
        sections.push(format!("Headings:\n- {}", headings.join("\n- ")));
    }

    if let Some(lines) = visible_lines.filter(|items| !items.is_empty()) {
        sections.push(format!("Visible text:\n{}", lines.join("\n")));
    }

    if let Some(text) = non_empty(fallback_text) {
        sections.push(format!("Page text:\n{}", normalize_multiline(text)));
    }

    let content = sections.join("\n\n");
    if content.trim().is_empty() {
        return Err(AssistantError::EmptyContext);
    }
    Ok(content)
}

async fn fetch_html(url: &str) -> AssistantResult<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| AssistantError::Fetch(error.to_string()))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AssistantError::Fetch(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AssistantError::Fetch(format!("HTTP {status}")));
    }
    response
        .text()
        .await
        .map_err(|error| AssistantError::Fetch(error.to_string()))
}

fn validate_url(raw: &str) -> AssistantResult<Url> {
    let parsed =
        Url::parse(raw.trim()).map_err(|error| AssistantError::InvalidUrl(error.to_string()))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        other => Err(AssistantError::InvalidUrl(format!(
            "仅支持 http/https，当前为 {other}"
        ))),
    }
}

fn first_selector_text(document: &Html, selector: &str) -> AssistantResult<Option<String>> {
    let selector = parse_selector(selector)?;
    Ok(document
        .select(&selector)
        .next()
        .map(|element| normalize_inline(&element.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty()))
}

fn meta_content(document: &Html, selector: &str) -> AssistantResult<Option<String>> {
    let selector = parse_selector(selector)?;
    Ok(document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(normalize_inline)
        .filter(|value| !value.is_empty()))
}

fn collect_text_lines(
    document: &Html,
    selector: &str,
    limit: usize,
) -> AssistantResult<Vec<String>> {
    let selector = parse_selector(selector)?;
    let mut seen = HashSet::new();
    let mut lines = Vec::new();
    for element in document.select(&selector) {
        let text = normalize_inline(&element.text().collect::<Vec<_>>().join(" "));
        if text.is_empty() || !seen.insert(text.clone()) {
            continue;
        }
        lines.push(text);
        if lines.len() >= limit {
            break;
        }
    }
    Ok(lines)
}

fn root_text(document: &Html, selector: &str) -> AssistantResult<String> {
    let selector = parse_selector(selector)?;
    Ok(document
        .select(&selector)
        .next()
        .map(|element| normalize_inline(&element.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default())
}

fn parse_selector(selector: &str) -> AssistantResult<Selector> {
    Selector::parse(selector).map_err(|error| AssistantError::Parse(error.to_string()))
}

fn visible_text_len(lines: &[String]) -> usize {
    lines.iter().map(|line| line.chars().count()).sum()
}

fn first_non_empty(values: [Option<&str>; 2]) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_inline(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_multiline(value: &str) -> String {
    value
        .lines()
        .map(normalize_inline)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_to_chars(value: &str, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value.to_string(), false);
    }

    let mut end = 0;
    for (count, (index, ch)) in value.char_indices().enumerate() {
        if count >= max_chars {
            break;
        }
        end = index + ch.len_utf8();
    }
    (
        format!(
            "{}\n\n[context truncated at {max_chars} characters]",
            &value[..end]
        ),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{build_html_preview, build_text_preview};
    use crate::PageContextInput;

    #[test]
    fn html_preview_extracts_metadata_and_visible_text() {
        let input = PageContextInput {
            url: Some("https://example.com/page".to_string()),
            ..Default::default()
        };
        let html = r#"
          <html>
            <head>
              <title>Dashboard</title>
              <meta name="description" content="Runtime status page" />
            </head>
            <body>
              <main>
                <h1>Cluster</h1>
                <button>Restart</button>
                <p>Three services are healthy.</p>
              </main>
            </body>
          </html>
        "#;

        let preview = build_html_preview(&input, html, "html").unwrap();

        assert_eq!(preview.title, "Dashboard");
        assert_eq!(preview.description.as_deref(), Some("Runtime status page"));
        assert!(preview.content.contains("Cluster"));
        assert!(preview.content.contains("Restart"));
        assert!(preview.content.contains("Three services are healthy."));
    }

    #[test]
    fn text_preview_accepts_selection_only_context() {
        let input = PageContextInput {
            selection: Some("Selected row: failed job".to_string()),
            ..Default::default()
        };

        let preview = build_text_preview(&input, "").unwrap();

        assert!(preview.content.contains("Selected row: failed job"));
    }
}
