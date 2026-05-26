use async_openai::{config::OpenAIConfig, types::responses::Response, Client};
use serde_json::json;

use crate::{
    context::preview_page_context,
    error::{AssistantError, AssistantResult},
    AssistantAnswer, AssistantChatRequest, AssistantKnowledgeContext, AssistantOpenAIConfig,
    AssistantTurn, AssistantTurnRole, PageContextPreview,
};

const DEFAULT_MODEL: &str = "gpt-5.5";
const MAX_HISTORY_TURNS: usize = 8;
const MAX_TURN_CHARS: usize = 2_000;
const BASE_INSTRUCTIONS: &str = r#"You are the OpenAI page-context assistant inside a local desktop admin app.
Use the supplied page context as the primary source of truth.
If the context does not contain enough evidence, say what is missing instead of inventing details.
Answer in Simplified Chinese by default unless the user asks for another language.
Keep responses concise, concrete, and action-oriented."#;

pub async fn ask_assistant(
    request: AssistantChatRequest,
    knowledge: Option<AssistantKnowledgeContext>,
    config: Option<AssistantOpenAIConfig>,
) -> AssistantResult<AssistantAnswer> {
    let question = request.question.trim();
    if question.is_empty() {
        return Err(AssistantError::EmptyQuestion);
    }
    let resolved = resolve_openai_config(config)?;

    let context = preview_page_context(&request.context).await?;
    let model = resolve_model(request.model.as_deref(), resolved.model.as_deref());
    let prompt = build_prompt(&context, knowledge.as_ref(), &request.history, question);

    let response: Response = Client::with_config(resolved.client_config)
        .responses()
        .create_byot(json!({
            "model": model,
            "instructions": BASE_INSTRUCTIONS,
            "input": prompt,
            "store": false
        }))
        .await
        .map_err(|error| AssistantError::OpenAI(error.to_string()))?;

    let answer = response
        .output_text()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AssistantError::OpenAI("OpenAI 未返回文本内容".to_string()))?;

    Ok(AssistantAnswer {
        answer,
        model,
        response_id: Some(response.id),
        context,
    })
}

fn build_prompt(
    context: &PageContextPreview,
    knowledge: Option<&AssistantKnowledgeContext>,
    history: &[AssistantTurn],
    question: &str,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Page context:\n");
    prompt.push_str(&context.content);

    if let Some(summary) = knowledge
        .and_then(|value| value.summary.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        prompt.push_str("\n\nKnowledge base context:\n");
        prompt.push_str(summary);
    }

    let retained_history = history
        .iter()
        .filter(|turn| !turn.content.trim().is_empty())
        .rev()
        .take(MAX_HISTORY_TURNS)
        .collect::<Vec<_>>();
    if !retained_history.is_empty() {
        prompt.push_str("\n\nConversation history:\n");
        for turn in retained_history.iter().rev() {
            prompt.push_str(match turn.role {
                AssistantTurnRole::User => "User: ",
                AssistantTurnRole::Assistant => "Assistant: ",
            });
            prompt.push_str(&truncate_to_chars(turn.content.trim(), MAX_TURN_CHARS));
            prompt.push('\n');
        }
    }

    prompt.push_str("\nCurrent question:\n");
    prompt.push_str(&truncate_to_chars(question, MAX_TURN_CHARS));
    prompt
}

fn resolve_model(input: Option<&str>, configured: Option<&str>) -> String {
    input
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .or_else(|| configured.map(ToOwned::to_owned))
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn resolve_openai_config(
    input: Option<AssistantOpenAIConfig>,
) -> AssistantResult<ResolvedOpenAIConfig> {
    let input = input.unwrap_or_default();
    let api_key = input
        .api_key
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .or_else(|| std::env::var("OPENAI_ADMIN_KEY").ok())
        .unwrap_or_default()
        .trim()
        .to_string();
    if api_key.is_empty() {
        return Err(AssistantError::MissingApiKey);
    }

    let mut client_config = OpenAIConfig::new().with_api_key(api_key);
    if let Some(base_url) = input
        .base_url
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        client_config = client_config.with_api_base(base_url);
    }

    Ok(ResolvedOpenAIConfig {
        client_config,
        model: input
            .model
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

fn truncate_to_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut end = 0;
    for (count, (index, ch)) in value.char_indices().enumerate() {
        if count >= max_chars {
            break;
        }
        end = index + ch.len_utf8();
    }
    format!("{} [truncated]", &value[..end])
}

#[cfg(test)]
mod tests {
    use crate::{
        openai::{build_prompt, resolve_model},
        AssistantKnowledgeContext, AssistantTurn, AssistantTurnRole, PageContextPreview,
    };

    #[test]
    fn prompt_includes_context_history_and_question() {
        let context = PageContextPreview {
            title: "Status".to_string(),
            url: None,
            description: None,
            source: "text".to_string(),
            character_count: 12,
            truncated: false,
            content: "Title: Status\nVisible text:\nAll systems healthy".to_string(),
        };
        let history = vec![AssistantTurn {
            role: AssistantTurnRole::User,
            content: "What changed?".to_string(),
        }];

        let prompt = build_prompt(
            &context,
            Some(&AssistantKnowledgeContext {
                summary: Some("Knowledge base excerpts:\nAIO".to_string()),
            }),
            &history,
            "What should I do next?",
        );

        assert!(prompt.contains("All systems healthy"));
        assert!(prompt.contains("Knowledge base excerpts"));
        assert!(prompt.contains("User: What changed?"));
        assert!(prompt.contains("What should I do next?"));
    }

    #[test]
    fn model_override_is_preserved() {
        assert_eq!(resolve_model(Some("gpt-5.4"), None), "gpt-5.4");
    }
}

struct ResolvedOpenAIConfig {
    client_config: OpenAIConfig,
    model: Option<String>,
}
