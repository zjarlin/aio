use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DictItemRecord {
    pub id: String,
    pub type_id: String,
    pub label: String,
    pub value: String,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeInput {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeUpdateInput {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictItemInput {
    pub type_id: String,
    pub label: String,
    pub value: String,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictItemUpdateInput {
    pub id: String,
    pub type_id: String,
    pub label: String,
    pub value: String,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictItemPageRequest {
    pub type_id: Option<String>,
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
}

pub async fn type_page(
    pool: &SqlitePool,
    token: String,
    request: PageRequest,
) -> AppResult<PageResult<DictTypeRecord>> {
    require_session(pool, &token).await?;
    let (offset, size) = normalize_page(&request);
    let keyword = format!("%{}%", request.keyword.unwrap_or_default());

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dict_types WHERE code LIKE ? OR name LIKE ?",
    )
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, DictTypeRecord>(
        "SELECT * FROM dict_types
         WHERE code LIKE ? OR name LIKE ?
         ORDER BY sort_order, created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(&keyword)
    .bind(&keyword)
    .bind(size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(PageResult {
        d: rows,
        t: total,
        p: PageInfo { o: offset, s: size },
    })
}

pub async fn create_type(
    pool: &SqlitePool,
    token: String,
    input: DictTypeInput,
) -> AppResult<DictTypeRecord> {
    require_session(pool, &token).await?;
    validate_pair(&input.code, &input.name)?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO dict_types (id, code, name, description, status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.code)
    .bind(input.name)
    .bind(input.description.unwrap_or_default())
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_type(pool, &id).await
}

pub async fn update_type(
    pool: &SqlitePool,
    token: String,
    input: DictTypeUpdateInput,
) -> AppResult<DictTypeRecord> {
    require_session(pool, &token).await?;
    validate_pair(&input.code, &input.name)?;
    let now = now_millis();

    sqlx::query(
        "UPDATE dict_types
         SET code = ?, name = ?, description = ?, status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.code)
    .bind(input.name)
    .bind(input.description.unwrap_or_default())
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_type(pool, &input.id).await
}

pub async fn delete_type(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    sqlx::query("DELETE FROM dict_types WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn item_page(
    pool: &SqlitePool,
    token: String,
    request: DictItemPageRequest,
) -> AppResult<PageResult<DictItemRecord>> {
    require_session(pool, &token).await?;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let type_id = request.type_id.unwrap_or_default();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dict_items
         WHERE (? = '' OR type_id = ?) AND (label LIKE ? OR value LIKE ?)",
    )
    .bind(&type_id)
    .bind(&type_id)
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, DictItemRecord>(
        "SELECT * FROM dict_items
         WHERE (? = '' OR type_id = ?) AND (label LIKE ? OR value LIKE ?)
         ORDER BY sort_order, created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(&type_id)
    .bind(&type_id)
    .bind(&keyword)
    .bind(&keyword)
    .bind(size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(PageResult {
        d: rows,
        t: total,
        p: PageInfo { o: offset, s: size },
    })
}

pub async fn create_item(
    pool: &SqlitePool,
    token: String,
    input: DictItemInput,
) -> AppResult<DictItemRecord> {
    require_session(pool, &token).await?;
    validate_pair(&input.value, &input.label)?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO dict_items (id, type_id, label, value, status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.type_id)
    .bind(input.label)
    .bind(input.value)
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_item(pool, &id).await
}

pub async fn update_item(
    pool: &SqlitePool,
    token: String,
    input: DictItemUpdateInput,
) -> AppResult<DictItemRecord> {
    require_session(pool, &token).await?;
    validate_pair(&input.value, &input.label)?;
    let now = now_millis();

    sqlx::query(
        "UPDATE dict_items
         SET type_id = ?, label = ?, value = ?, status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.type_id)
    .bind(input.label)
    .bind(input.value)
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_item(pool, &input.id).await
}

pub async fn delete_item(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    sqlx::query("DELETE FROM dict_items WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn find_type(pool: &SqlitePool, id: &str) -> AppResult<DictTypeRecord> {
    sqlx::query_as::<_, DictTypeRecord>("SELECT * FROM dict_types WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn find_item(pool: &SqlitePool, id: &str) -> AppResult<DictItemRecord> {
    sqlx::query_as::<_, DictItemRecord>("SELECT * FROM dict_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

fn validate_pair(code: &str, name: &str) -> AppResult<()> {
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(AppError::BadRequest("编码和值不能为空".to_string()));
    }
    Ok(())
}
