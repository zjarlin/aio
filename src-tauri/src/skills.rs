use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    asset_items::normalize_filter_categories,
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub prompt: String,
    pub tags: Vec<String>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
struct SkillRow {
    id: String,
    code: String,
    name: String,
    category: String,
    description: String,
    prompt: String,
    tags_json: String,
    status: String,
    sort_order: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInput {
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInput {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillToggleInput {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPageRequest {
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub categories: Option<Vec<String>>,
    pub status: Option<String>,
}

pub async fn page(
    pool: &SqlitePool,
    token: String,
    request: SkillPageRequest,
) -> AppResult<PageResult<SkillRecord>> {
    require_session(pool, &token).await?;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let tag_filters = normalize_filter_categories(request.category, request.categories);
    let tag_filter_sql = " AND (category = ? OR tags_json LIKE ?)".repeat(tag_filters.len());
    let status = request.status.unwrap_or_default();

    let total_sql = format!(
        "SELECT COUNT(*) FROM skills
         WHERE 1 = 1
           {tag_filter_sql}
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR name LIKE ? OR description LIKE ?
             OR prompt LIKE ? OR tags_json LIKE ?
           )",
    );
    let mut total_query = sqlx::query_scalar::<_, i64>(&total_sql);
    for tag in &tag_filters {
        total_query = total_query.bind(tag).bind(format!("%{}%", tag));
    }
    let total = total_query
        .bind(&status)
        .bind(&status)
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .fetch_one(pool)
        .await?;

    let rows_sql = format!(
        "SELECT * FROM skills
         WHERE 1 = 1
           {tag_filter_sql}
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR name LIKE ? OR description LIKE ?
             OR prompt LIKE ? OR tags_json LIKE ?
           )
         ORDER BY sort_order, updated_at DESC
         LIMIT ? OFFSET ?",
    );
    let mut rows_query = sqlx::query_as::<_, SkillRow>(&rows_sql);
    for tag in &tag_filters {
        rows_query = rows_query.bind(tag).bind(format!("%{}%", tag));
    }
    let rows = rows_query
        .bind(&status)
        .bind(&status)
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

pub async fn create(pool: &SqlitePool, token: String, input: SkillInput) -> AppResult<SkillRecord> {
    require_session(pool, &token).await?;
    validate_code_name(&input.code, &input.name)?;
    let status = normalize_status(input.status)?;
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO skills
         (id, code, name, category, description, prompt, tags_json, status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.code)
    .bind(input.name)
    .bind(input.category.unwrap_or_default())
    .bind(input.description.unwrap_or_default())
    .bind(input.prompt.unwrap_or_default())
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
    input: SkillUpdateInput,
) -> AppResult<SkillRecord> {
    require_session(pool, &token).await?;
    validate_code_name(&input.code, &input.name)?;
    let status = normalize_status(input.status)?;
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE skills
         SET code = ?, name = ?, category = ?, description = ?, prompt = ?,
             tags_json = ?, status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.code)
    .bind(input.name)
    .bind(input.category.unwrap_or_default())
    .bind(input.description.unwrap_or_default())
    .bind(input.prompt.unwrap_or_default())
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
    let affected = sqlx::query("DELETE FROM skills WHERE id = ?")
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
    input: SkillToggleInput,
) -> AppResult<SkillRecord> {
    require_session(pool, &token).await?;
    let status = normalize_status(Some(input.status))?;
    let now = now_millis();
    let affected = sqlx::query("UPDATE skills SET status = ?, updated_at = ? WHERE id = ?")
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

async fn find(pool: &SqlitePool, id: &str) -> AppResult<SkillRecord> {
    let row = sqlx::query_as::<_, SkillRow>("SELECT * FROM skills WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    to_record(row)
}

fn to_record(row: SkillRow) -> AppResult<SkillRecord> {
    let tags = serde_json::from_str::<Vec<String>>(&row.tags_json)
        .map_err(|source| AppError::Json { source })?;

    Ok(SkillRecord {
        id: row.id,
        code: row.code,
        name: row.name,
        category: row.category,
        description: row.description,
        prompt: row.prompt,
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

fn validate_code_name(code: &str, name: &str) -> AppResult<()> {
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(AppError::BadRequest("技能编码和名称不能为空".to_string()));
    }
    Ok(())
}

fn normalize_status(status: Option<String>) -> AppResult<String> {
    let status = status.unwrap_or_else(|| "enabled".to_string());
    match status.as_str() {
        "enabled" | "disabled" => Ok(status),
        _ => Err(AppError::BadRequest("技能状态只能是启用或禁用".to_string())),
    }
}
