use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    asset_items::{normalize_kind, normalize_status},
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVariableRecord {
    pub id: String,
    pub kind: String,
    pub asset_item_id: Option<String>,
    pub category: String,
    pub key: String,
    pub value: String,
    pub default_value: String,
    pub description: String,
    pub value_kind: String,
    pub source: String,
    pub scope: String,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
struct AssetVariableRow {
    id: String,
    kind: String,
    asset_item_id: Option<String>,
    category: String,
    key: String,
    value: String,
    default_value: String,
    description: String,
    value_kind: String,
    source: String,
    status: String,
    sort_order: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVariablePageRequest {
    pub kind: String,
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub asset_item_id: Option<String>,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVariableInput {
    pub id: Option<String>,
    pub kind: String,
    pub asset_item_id: Option<String>,
    pub category: Option<String>,
    pub key: String,
    pub value: Option<String>,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub value_kind: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

pub async fn page(
    pool: &SqlitePool,
    token: String,
    request: AssetVariablePageRequest,
) -> AppResult<PageResult<AssetVariableRecord>> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&request.kind)?;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let asset_item_id = trim_optional(request.asset_item_id).unwrap_or_default();
    let category = request.category.unwrap_or_default();
    let scope = normalize_scope(request.scope)?;
    let status = request.status.unwrap_or_default();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM asset_variables
         WHERE kind = ?
           AND (? = '' OR asset_item_id = ?)
           AND (? = '' OR category = ?)
           AND (? = '' OR status = ?)
           AND (
             ? = ''
             OR (? = 'grid' AND asset_item_id IS NULL)
             OR (? = 'file' AND asset_item_id IS NOT NULL)
           )
           AND (
             key LIKE ? OR value LIKE ? OR default_value LIKE ?
             OR description LIKE ? OR value_kind LIKE ? OR source LIKE ?
           )",
    )
    .bind(&kind)
    .bind(&asset_item_id)
    .bind(&asset_item_id)
    .bind(&category)
    .bind(&category)
    .bind(&status)
    .bind(&status)
    .bind(&scope)
    .bind(&scope)
    .bind(&scope)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AssetVariableRow>(
        "SELECT * FROM asset_variables
         WHERE kind = ?
           AND (? = '' OR asset_item_id = ?)
           AND (? = '' OR category = ?)
           AND (? = '' OR status = ?)
           AND (
             ? = ''
             OR (? = 'grid' AND asset_item_id IS NULL)
             OR (? = 'file' AND asset_item_id IS NOT NULL)
           )
           AND (
             key LIKE ? OR value LIKE ? OR default_value LIKE ?
             OR description LIKE ? OR value_kind LIKE ? OR source LIKE ?
           )
         ORDER BY sort_order, key
         LIMIT ? OFFSET ?",
    )
    .bind(&kind)
    .bind(&asset_item_id)
    .bind(&asset_item_id)
    .bind(&category)
    .bind(&category)
    .bind(&status)
    .bind(&status)
    .bind(&scope)
    .bind(&scope)
    .bind(&scope)
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

    Ok(PageResult {
        d: rows.into_iter().map(to_record).collect(),
        t: total,
        p: PageInfo { o: offset, s: size },
    })
}

pub async fn upsert(
    pool: &SqlitePool,
    token: String,
    input: AssetVariableInput,
) -> AppResult<AssetVariableRecord> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&input.kind)?;
    let key = normalize_key(&input.key)?;
    let asset_item_id = trim_optional(input.asset_item_id);
    let category = input.category.unwrap_or_default().trim().to_string();
    let value = input.value.unwrap_or_default();
    let default_value = input.default_value.unwrap_or_else(|| value.clone());
    let description = input.description.unwrap_or_default();
    let value_kind = normalize_label(input.value_kind, "text")?;
    let source = normalize_label(input.source, "manual")?;
    let status = normalize_status(input.status)?;
    let sort_order = input.sort_order.unwrap_or(0);
    let now = now_millis();

    if let Some(id) = trim_optional(input.id) {
        update_by_id(
            pool,
            &id,
            VariableWrite {
                kind,
                asset_item_id,
                category,
                key,
                value,
                default_value,
                description,
                value_kind,
                source,
                status,
                sort_order,
                now,
            },
        )
        .await?;
        return find(pool, &id).await;
    }

    let existing_id =
        find_existing_id(pool, &kind, asset_item_id.as_deref(), &category, &key).await?;
    if let Some(id) = existing_id {
        update_by_id(
            pool,
            &id,
            VariableWrite {
                kind,
                asset_item_id,
                category,
                key,
                value,
                default_value,
                description,
                value_kind,
                source,
                status,
                sort_order,
                now,
            },
        )
        .await?;
        return find(pool, &id).await;
    }

    let id = new_id();
    sqlx::query(
        "INSERT INTO asset_variables
         (id, kind, asset_item_id, category, key, value, default_value, description,
          value_kind, source, status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(kind)
    .bind(asset_item_id)
    .bind(category)
    .bind(key)
    .bind(value)
    .bind(default_value)
    .bind(description)
    .bind(value_kind)
    .bind(source)
    .bind(status)
    .bind(sort_order)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find(pool, &id).await
}

pub async fn delete(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    let affected = sqlx::query("DELETE FROM asset_variables WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

struct VariableWrite {
    kind: String,
    asset_item_id: Option<String>,
    category: String,
    key: String,
    value: String,
    default_value: String,
    description: String,
    value_kind: String,
    source: String,
    status: String,
    sort_order: i64,
    now: i64,
}

async fn update_by_id(pool: &SqlitePool, id: &str, values: VariableWrite) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE asset_variables
         SET kind = ?, asset_item_id = ?, category = ?, key = ?, value = ?,
             default_value = ?, description = ?, value_kind = ?, source = ?,
             status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(values.kind)
    .bind(values.asset_item_id)
    .bind(values.category)
    .bind(values.key)
    .bind(values.value)
    .bind(values.default_value)
    .bind(values.description)
    .bind(values.value_kind)
    .bind(values.source)
    .bind(values.status)
    .bind(values.sort_order)
    .bind(values.now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

async fn find_existing_id(
    pool: &SqlitePool,
    kind: &str,
    asset_item_id: Option<&str>,
    category: &str,
    key: &str,
) -> AppResult<Option<String>> {
    if let Some(asset_item_id) = asset_item_id {
        return sqlx::query_scalar::<_, String>(
            "SELECT id FROM asset_variables
             WHERE kind = ? AND asset_item_id = ? AND key = ?",
        )
        .bind(kind)
        .bind(asset_item_id)
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(Into::into);
    }

    sqlx::query_scalar::<_, String>(
        "SELECT id FROM asset_variables
         WHERE kind = ? AND asset_item_id IS NULL AND category = ? AND key = ?",
    )
    .bind(kind)
    .bind(category)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

async fn find(pool: &SqlitePool, id: &str) -> AppResult<AssetVariableRecord> {
    let row = sqlx::query_as::<_, AssetVariableRow>("SELECT * FROM asset_variables WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(to_record(row))
}

fn to_record(row: AssetVariableRow) -> AssetVariableRecord {
    let scope = if row.asset_item_id.is_some() {
        "file"
    } else {
        "grid"
    };
    AssetVariableRecord {
        id: row.id,
        kind: row.kind,
        asset_item_id: row.asset_item_id,
        category: row.category,
        key: row.key,
        value: row.value,
        default_value: row.default_value,
        description: row.description,
        value_kind: row.value_kind,
        source: row.source,
        scope: scope.to_string(),
        status: row.status,
        sort_order: row.sort_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn normalize_scope(scope: Option<String>) -> AppResult<String> {
    let scope = scope.unwrap_or_default();
    let scope = scope.trim();
    if scope.is_empty() || matches!(scope, "grid" | "file") {
        Ok(scope.to_string())
    } else {
        Err(AppError::BadRequest("变量作用域不合法".to_string()))
    }
}

fn normalize_key(value: &str) -> AppResult<String> {
    let key = value.trim().to_ascii_uppercase();
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(AppError::BadRequest("变量名不能为空".to_string()));
    };
    if !first.is_ascii_alphabetic() {
        return Err(AppError::BadRequest("变量名必须以英文字母开头".to_string()));
    }
    if !chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Err(AppError::BadRequest(
            "变量名只能包含英文字母、数字和下划线".to_string(),
        ));
    }
    Ok(key)
}

fn normalize_label(value: Option<String>, fallback: &str) -> AppResult<String> {
    let value = value.unwrap_or_else(|| fallback.to_string());
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return Ok(fallback.to_string());
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
    {
        Ok(value)
    } else {
        Err(AppError::BadRequest("变量分类字段不合法".to_string()))
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
