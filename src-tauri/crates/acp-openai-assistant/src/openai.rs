use async_openai::{types::responses::Response, Client};
use serde_json::json;

use crate::{
    context::preview_page_context,
    error::{AssistantError, AssistantResult},
    AssistantAnswer, AssistantChatRequest, AssistantTurn, AssistantTurnRole, PageContextPreview,
};

const DEFAULT_MODEL: &str = "gpt-5.5";
const MAX_HISTORY_TURNS: usize = 8;
const MAX_TURN_CHARS: usize = 2_000;
const BASE_INSTRUCTIONS: &str = r#"You are the OpenAI page-context assistant inside a local desktop admin app.
Use the supplied page context as the primary source of truth.
If the context does not contain enough evidence, say what is missing instead of inventing details.
Answer in Simplified Chinese by default unless the user asks for another language.
Keep responses concise, concrete, and action-oriented."#;

pub async fn ask_assistant(request: AssistantChatRequest) -> AssistantResult<AssistantAnswer> {
    let question = request.question.trim();
    if question.is_empty() {
        return Err(AssistantError::EmptyQuestion);
    }
    ensure_api_key()?;

    let context = preview_page_context(&request.context).await?;
    let model = resolve_model(request.model.as_deref());
    let prompt = build_prompt(&context, &request.history, question);

    let response: Response = Client::new()
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

fn build_prompt(context: &PageContextPreview, history: &[AssistantTurn], question: &str) -> String {
    let mut prompt = String::new();
    prompt.push_str("Page context:\n");
    prompt.push_str(&context.content);

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

fn resolve_model(input: Option<&str>) -> String {
    input
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn ensure_api_key() -> AssistantResult<()> {
    let key = std::env::var("OPENAI_API_KEY").or_else(|_| std::env::var("OPENAI_ADMIN_KEY"));
    match key {
        Ok(value) if !value.trim().is_empty() => Ok(()),
        _ => Err(AssistantError::MissingApiKey),
    }
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
        AssistantTurn, AssistantTurnRole, PageContextPreview,
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

        let prompt = build_prompt(&context, &history, "What should I do next?");

        assert!(prompt.contains("All systems healthy"));
        assert!(prompt.contains("User: What changed?"));
        assert!(prompt.contains("What should I do next?"));
    }

    #[test]
    fn model_override_is_preserved() {
        assert_eq!(resolve_model(Some("gpt-5.4")), "gpt-5.4");
    }
}
