use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    auth::require_session,
    db::now_millis,
    error::AppResult,
};

const OPENAI_API_KEY: &str = "openai.api_key";
const OPENAI_BASE_URL: &str = "openai.base_url";
const OPENAI_MODEL: &str = "openai.model";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAISettingsRecord {
    pub api_key_configured: bool,
    pub api_key_preview: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAISettingsInput {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedOpenAISettings {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

pub async fn get_openai_settings(
    pool: &SqlitePool,
    token: String,
) -> AppResult<OpenAISettingsRecord> {
    require_session(pool, &token).await?;
    build_openai_settings_record(pool).await
}

pub async fn save_openai_settings(
    pool: &SqlitePool,
    token: String,
    input: OpenAISettingsInput,
) -> AppResult<OpenAISettingsRecord> {
    require_session(pool, &token).await?;

    if let Some(api_key) = input.api_key {
        let value = api_key.trim().to_string();
        if value.is_empty() {
            delete_setting(pool, OPENAI_API_KEY).await?;
        } else {
            upsert_setting(pool, OPENAI_API_KEY, value).await?;
        }
    }

    if let Some(base_url) = input.base_url {
        let value = normalize_base_url(&base_url);
        if value.is_empty() {
            delete_setting(pool, OPENAI_BASE_URL).await?;
        } else {
            upsert_setting(pool, OPENAI_BASE_URL, value).await?;
        }
    }

    if let Some(model) = input.model {
        let value = model.trim().to_string();
        if value.is_empty() {
            delete_setting(pool, OPENAI_MODEL).await?;
        } else {
            upsert_setting(pool, OPENAI_MODEL, value).await?;
        }
    }

    build_openai_settings_record(pool).await
}

pub async fn resolve_openai_settings(pool: &SqlitePool) -> AppResult<ResolvedOpenAISettings> {
    let api_key = get_setting(pool, OPENAI_API_KEY)
        .await?
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .or_else(|| std::env::var("OPENAI_ADMIN_KEY").ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    let base_url = get_setting(pool, OPENAI_BASE_URL)
        .await?
        .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
        .map(|value| normalize_base_url(&value))
        .filter(|value| !value.is_empty());

    let model = get_setting(pool, OPENAI_MODEL)
        .await?
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(ResolvedOpenAISettings {
        api_key,
        base_url,
        model,
    })
}

async fn build_openai_settings_record(pool: &SqlitePool) -> AppResult<OpenAISettingsRecord> {
    let resolved = resolve_openai_settings(pool).await?;
    Ok(OpenAISettingsRecord {
        api_key_configured: !resolved.api_key.is_empty(),
        api_key_preview: mask_api_key(&resolved.api_key),
        base_url: resolved.base_url.unwrap_or_default(),
        model: resolved.model.unwrap_or_else(|| "gpt-5.5".to_string()),
    })
}

async fn upsert_setting(pool: &SqlitePool, key: &str, value: String) -> AppResult<()> {
    let now = now_millis();
    sqlx::query(
        "INSERT INTO app_settings (setting_key, setting_value, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(setting_key) DO UPDATE SET
           setting_value = excluded.setting_value,
           updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn delete_setting(pool: &SqlitePool, key: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM app_settings WHERE setting_key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

async fn get_setting(pool: &SqlitePool, key: &str) -> AppResult<Option<String>> {
    sqlx::query_scalar::<_, String>(
        "SELECT setting_value FROM app_settings WHERE setting_key = ? LIMIT 1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

fn normalize_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn mask_api_key(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= 8 {
        return if trimmed.is_empty() {
            String::new()
        } else {
            "********".to_string()
        };
    }
    format!("{}****{}", &trimmed[..4], &trimmed[trimmed.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::{mask_api_key, normalize_base_url};

    #[test]
    fn base_url_is_trimmed() {
        assert_eq!(
            normalize_base_url(" https://api.addzero.site/ "),
            "https://api.addzero.site"
        );
    }

    #[test]
    fn api_key_is_masked() {
        let masked = mask_api_key("sk-1234567890");
        assert!(masked.starts_with("sk-1"));
        assert!(masked.ends_with("7890"));
    }
}
