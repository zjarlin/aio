use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{normalize_page, PageInfo, PageRequest, PageResult},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteRecord {
    pub id: String,
    pub owner_id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
struct NoteRow {
    id: String,
    owner_id: String,
    title: String,
    content: String,
    category: String,
    is_favorite: i64,
    is_archived: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteUpdateInput {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFlagInput {
    pub id: String,
    pub value: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePageRequest {
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub archived: Option<bool>,
}

pub async fn page(
    pool: &SqlitePool,
    token: String,
    request: NotePageRequest,
) -> AppResult<PageResult<NoteRecord>> {
    let session = require_session(pool, &token).await?;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let category = request.category.unwrap_or_default();
    let archived = request.archived.unwrap_or(false) as i64;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notes
         WHERE owner_id = ?
           AND is_archived = ?
           AND (? = '' OR category = ?)
           AND (title LIKE ? OR content LIKE ?)",
    )
    .bind(&session.user_id)
    .bind(archived)
    .bind(&category)
    .bind(&category)
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, NoteRow>(
        "SELECT * FROM notes
         WHERE owner_id = ?
           AND is_archived = ?
           AND (? = '' OR category = ?)
           AND (title LIKE ? OR content LIKE ?)
         ORDER BY is_favorite DESC, updated_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(&session.user_id)
    .bind(archived)
    .bind(&category)
    .bind(&category)
    .bind(&keyword)
    .bind(&keyword)
    .bind(size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let mut notes = Vec::with_capacity(rows.len());
    for row in rows {
        notes.push(to_record(pool, row).await?);
    }

    Ok(PageResult {
        d: notes,
        t: total,
        p: PageInfo { o: offset, s: size },
    })
}

pub async fn create(pool: &SqlitePool, token: String, input: NoteInput) -> AppResult<NoteRecord> {
    let session = require_session(pool, &token).await?;
    validate_title(&input.title)?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO notes (id, owner_id, title, content, category, is_favorite, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, 0, ?, ?)",
    )
    .bind(&id)
    .bind(session.user_id)
    .bind(input.title)
    .bind(input.content.unwrap_or_default())
    .bind(input.category.unwrap_or_default())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    replace_tags(pool, &id, input.tags.unwrap_or_default()).await?;
    find_owned(pool, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    token: String,
    input: NoteUpdateInput,
) -> AppResult<NoteRecord> {
    let session = require_session(pool, &token).await?;
    validate_title(&input.title)?;
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE notes
         SET title = ?, content = ?, category = ?, updated_at = ?
         WHERE id = ? AND owner_id = ?",
    )
    .bind(input.title)
    .bind(input.content.unwrap_or_default())
    .bind(input.category.unwrap_or_default())
    .bind(now)
    .bind(&input.id)
    .bind(session.user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    replace_tags(pool, &input.id, input.tags.unwrap_or_default()).await?;
    find_owned(pool, &input.id).await
}

pub async fn delete(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    let session = require_session(pool, &token).await?;
    sqlx::query("DELETE FROM notes WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(session.user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_archived(
    pool: &SqlitePool,
    token: String,
    input: NoteFlagInput,
) -> AppResult<NoteRecord> {
    set_flag(pool, token, input, "is_archived").await
}

pub async fn set_favorite(
    pool: &SqlitePool,
    token: String,
    input: NoteFlagInput,
) -> AppResult<NoteRecord> {
    set_flag(pool, token, input, "is_favorite").await
}

async fn set_flag(
    pool: &SqlitePool,
    token: String,
    input: NoteFlagInput,
    field: &str,
) -> AppResult<NoteRecord> {
    let session = require_session(pool, &token).await?;
    let value = input.value as i64;
    let now = now_millis();
    let sql = format!("UPDATE notes SET {field} = ?, updated_at = ? WHERE id = ? AND owner_id = ?");

    let affected = sqlx::query(&sql)
        .bind(value)
        .bind(now)
        .bind(&input.id)
        .bind(session.user_id)
        .execute(pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    find_owned(pool, &input.id).await
}

async fn replace_tags(pool: &SqlitePool, note_id: &str, tags: Vec<String>) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM note_tags WHERE note_id = ?")
        .bind(note_id)
        .execute(&mut *tx)
        .await?;

    for tag in tags {
        let normalized = tag.trim();
        if normalized.is_empty() {
            continue;
        }
        sqlx::query("INSERT OR IGNORE INTO note_tags (note_id, tag) VALUES (?, ?)")
            .bind(note_id)
            .bind(normalized)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn find_owned(pool: &SqlitePool, id: &str) -> AppResult<NoteRecord> {
    let row = sqlx::query_as::<_, NoteRow>("SELECT * FROM notes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    to_record(pool, row).await
}

async fn to_record(pool: &SqlitePool, row: NoteRow) -> AppResult<NoteRecord> {
    let tags =
        sqlx::query_scalar::<_, String>("SELECT tag FROM note_tags WHERE note_id = ? ORDER BY tag")
            .bind(&row.id)
            .fetch_all(pool)
            .await?;

    Ok(NoteRecord {
        id: row.id,
        owner_id: row.owner_id,
        title: row.title,
        content: row.content,
        category: row.category,
        tags,
        is_favorite: row.is_favorite == 1,
        is_archived: row.is_archived == 1,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn validate_title(title: &str) -> AppResult<()> {
    if title.trim().is_empty() {
        return Err(AppError::BadRequest("标题不能为空".to_string()));
    }
    Ok(())
}
