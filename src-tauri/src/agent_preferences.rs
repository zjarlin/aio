use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreferenceRecord {
    pub id: String,
    pub code: String,
    pub section: String,
    pub domain: String,
    pub title: String,
    pub content: String,
    pub rationale: String,
    pub tags: Vec<String>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
struct AgentPreferenceRow {
    id: String,
    code: String,
    section: String,
    domain: String,
    title: String,
    content: String,
    rationale: String,
    tags_json: String,
    status: String,
    sort_order: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreferenceInput {
    pub code: String,
    pub section: String,
    pub domain: Option<String>,
    pub title: String,
    pub content: Option<String>,
    pub rationale: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreferenceUpdateInput {
    pub id: String,
    pub code: String,
    pub section: String,
    pub domain: Option<String>,
    pub title: String,
    pub content: Option<String>,
    pub rationale: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreferenceToggleInput {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreferencePageRequest {
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub section: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
}

pub async fn page(
    pool: &SqlitePool,
    token: String,
    request: AgentPreferencePageRequest,
) -> AppResult<PageResult<AgentPreferenceRecord>> {
    require_session(pool, &token).await?;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let section = request.section.unwrap_or_default();
    let domain = request.domain.unwrap_or_default();
    let status = request.status.unwrap_or_default();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM agent_preferences
         WHERE (? = '' OR section = ?)
           AND (? = '' OR domain = ?)
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR title LIKE ? OR content LIKE ?
             OR rationale LIKE ? OR domain LIKE ? OR tags_json LIKE ?
           )",
    )
    .bind(&section)
    .bind(&section)
    .bind(&domain)
    .bind(&domain)
    .bind(&status)
    .bind(&status)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AgentPreferenceRow>(
        "SELECT * FROM agent_preferences
         WHERE (? = '' OR section = ?)
           AND (? = '' OR domain = ?)
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR title LIKE ? OR content LIKE ?
             OR rationale LIKE ? OR domain LIKE ? OR tags_json LIKE ?
           )
         ORDER BY sort_order, updated_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(&section)
    .bind(&section)
    .bind(&domain)
    .bind(&domain)
    .bind(&status)
    .bind(&status)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let records = rows
        .into_iter()
        .map(to_record)
        .collect::<AppResult<Vec<_>>>()?;

    Ok(PageResult {
        d: records,
        t: total,
        p: PageInfo { o: offset, s: size },
    })
}

pub async fn create(
    pool: &SqlitePool,
    token: String,
    input: AgentPreferenceInput,
) -> AppResult<AgentPreferenceRecord> {
    require_session(pool, &token).await?;
    validate_code_title_section(&input.code, &input.title, &input.section)?;
    let status = normalize_status(input.status)?;
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO agent_preferences
         (id, code, section, domain, title, content, rationale, tags_json, status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.code)
    .bind(input.section)
    .bind(input.domain.unwrap_or_default())
    .bind(input.title)
    .bind(input.content.unwrap_or_default())
    .bind(input.rationale.unwrap_or_default())
    .bind(tags_json)
    .bind(status)
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find(pool, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    token: String,
    input: AgentPreferenceUpdateInput,
) -> AppResult<AgentPreferenceRecord> {
    require_session(pool, &token).await?;
    validate_code_title_section(&input.code, &input.title, &input.section)?;
    let status = normalize_status(input.status)?;
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE agent_preferences
         SET code = ?, section = ?, domain = ?, title = ?, content = ?,
             rationale = ?, tags_json = ?, status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.code)
    .bind(input.section)
    .bind(input.domain.unwrap_or_default())
    .bind(input.title)
    .bind(input.content.unwrap_or_default())
    .bind(input.rationale.unwrap_or_default())
    .bind(tags_json)
    .bind(status)
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    find(pool, &input.id).await
}

pub async fn delete(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    let affected = sqlx::query("DELETE FROM agent_preferences WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn toggle(
    pool: &SqlitePool,
    token: String,
    input: AgentPreferenceToggleInput,
) -> AppResult<AgentPreferenceRecord> {
    require_session(pool, &token).await?;
    let status = normalize_status(Some(input.status))?;
    let now = now_millis();
    let affected =
        sqlx::query("UPDATE agent_preferences SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(now)
            .bind(&input.id)
            .execute(pool)
            .await?
            .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    find(pool, &input.id).await
}

async fn find(pool: &SqlitePool, id: &str) -> AppResult<AgentPreferenceRecord> {
    let row =
        sqlx::query_as::<_, AgentPreferenceRow>("SELECT * FROM agent_preferences WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(AppError::NotFound)?;
    to_record(row)
}

fn to_record(row: AgentPreferenceRow) -> AppResult<AgentPreferenceRecord> {
    let tags = serde_json::from_str::<Vec<String>>(&row.tags_json)
        .map_err(|source| AppError::Json { source })?;

    Ok(AgentPreferenceRecord {
        id: row.id,
        code: row.code,
        section: row.section,
        domain: row.domain,
        title: row.title,
        content: row.content,
        rationale: row.rationale,
        tags,
        status: row.status,
        sort_order: row.sort_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn encode_tags(tags: Vec<String>) -> AppResult<String> {
    let mut seen = HashSet::new();
    let normalized = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.clone()))
        .collect::<Vec<_>>();

    serde_json::to_string(&normalized).map_err(|source| AppError::Json { source })
}

fn validate_code_title_section(code: &str, title: &str, section: &str) -> AppResult<()> {
    if code.trim().is_empty() || title.trim().is_empty() || section.trim().is_empty() {
        return Err(AppError::BadRequest(
            "AGENTS.md 规则编码、标题和分区不能为空".to_string(),
        ));
    }
    Ok(())
}

fn normalize_status(status: Option<String>) -> AppResult<String> {
    let status = status.unwrap_or_else(|| "enabled".to_string());
    match status.as_str() {
        "enabled" | "disabled" => Ok(status),
        _ => Err(AppError::BadRequest(
            "AGENTS.md 规则状态只能是启用或禁用".to_string(),
        )),
    }
}
