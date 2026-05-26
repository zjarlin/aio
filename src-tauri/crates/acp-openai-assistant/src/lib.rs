mod context;
mod error;
mod openai;

use serde::{Deserialize, Serialize};

pub use context::preview_page_context;
pub use error::{AssistantError, AssistantResult};
pub use openai::ask_assistant;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageContextInput {
    pub url: Option<String>,
    pub title: Option<String>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub selection: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageContextPreview {
    pub title: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub source: String,
    pub character_count: usize,
    pub truncated: bool,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatRequest {
    #[serde(default)]
    pub context: PageContextInput,
    #[serde(default)]
    pub history: Vec<AssistantTurn>,
    pub question: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantKnowledgeContext {
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantOpenAIConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

impl Default for AssistantChatRequest {
    fn default() -> Self {
        Self {
            context: PageContextInput::default(),
            history: Vec::new(),
            question: String::new(),
            model: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantTurn {
    pub role: AssistantTurnRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AssistantTurnRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantAnswer {
    pub answer: String,
    pub model: String,
    pub response_id: Option<String>,
    pub context: PageContextPreview,
}
