# ACP OpenAI Assistant

Reusable Rust crate for the AIO desktop app's page-context OpenAI assistant.

## Purpose

This crate turns page context into a compact prompt payload, then sends it to OpenAI through the Responses API. It accepts three context sources:

- `url`: fetches an HTTP(S) page and extracts visible text.
- `html`: parses pasted or captured HTML without network access.
- `text`: uses pasted visible page text directly.

`selection` is always treated as high-priority context and is placed before the full page text.

## Library Choice

The extraction layer uses [`scraper`](https://docs.rs/scraper), a Rust HTML parser with CSS selector support. This was chosen over article-readability ports because the assistant must handle admin and app pages, not only blog or documentation articles. It also remains compatible with this repository's `rust-version = 1.77.2`; the crate pins the `0.24.x` line because newer `scraper` releases use Rust 2024 edition.

OpenAI calls use [`async-openai`](https://docs.rs/async-openai) with the Responses API and BYOT request support so the app can keep a small request shape while still using typed response parsing.

## Environment

Required:

```sh
export OPENAI_API_KEY="sk-..."
```

Optional:

```sh
export OPENAI_MODEL="gpt-5.5"
export OPENAI_BASE_URL="https://api.openai.com/v1"
export OPENAI_ORG_ID="org_..."
export OPENAI_PROJECT_ID="proj_..."
```

`OPENAI_MODEL` defaults to `gpt-5.5`. `OPENAI_BASE_URL`, organization, and project environment variables are read by `async-openai`.

## Example

```rust
use acp_openai_assistant::{
    ask_assistant, AssistantChatRequest, PageContextInput,
};

# async fn run() -> Result<(), acp_openai_assistant::AssistantError> {
let response = ask_assistant(AssistantChatRequest {
    context: PageContextInput {
        url: Some("https://example.com".to_string()),
        ..Default::default()
    },
    question: "Summarize the page and list required follow-up actions.".to_string(),
    ..Default::default()
})
.await?;

println!("{}", response.answer);
# Ok(())
# }
```

## Boundaries

- URL fetching is limited to `http` and `https`.
- The generated context is truncated to keep requests predictable.
- The crate is stateless; UI layers should pass conversation history when they need multi-turn continuity.
