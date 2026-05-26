use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    db::{new_id, now_millis},
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConsentRecord {
    pub id: String,
    pub user_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub capability: String,
    pub scope: String,
    pub status: String,
    pub reason: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConsentGrantInput {
    pub source_id: String,
    pub source_kind: String,
    pub capability: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConsentRevokeInput {
    pub source_id: String,
    pub capability: String,
    #[serde(default)]
    pub scope: String,
}

#[derive(Debug, Clone, FromRow)]
struct PermissionConsentRow {
    id: String,
    user_id: String,
    source_id: String,
    source_kind: String,
    capability: String,
    scope: String,
    status: String,
    reason: String,
    created_at: i64,
    updated_at: i64,
}

impl From<PermissionConsentRow> for PermissionConsentRecord {
    fn from(row: PermissionConsentRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            source_id: row.source_id,
            source_kind: row.source_kind,
            capability: row.capability,
            scope: row.scope,
            status: row.status,
            reason: row.reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub async fn seed_system_consents(pool: &SqlitePool) -> AppResult<()> {
    let user_ids = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE status = 'enabled'")
        .fetch_all(pool)
        .await?;

    for user_id in user_ids {
        for grant in system_grants() {
            grant_for_user(
                pool,
                &user_id,
                PermissionConsentGrantInput {
                    source_id: grant.source_id.to_string(),
                    source_kind: "system".to_string(),
                    capability: grant.capability.to_string(),
                    scope: grant.scope.to_string(),
                    reason: "seeded trusted platform consent".to_string(),
                },
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn grant_for_user(
    pool: &SqlitePool,
    user_id: &str,
    input: PermissionConsentGrantInput,
) -> AppResult<PermissionConsentRecord> {
    let source_id = normalize_required(&input.source_id, "sourceId")?;
    let capability = normalize_required(&input.capability, "capability")?;
    let source_kind = normalize_kind(&input.source_kind);
    let scope = normalize_scope(&input.scope);
    let now = now_millis();

    sqlx::query(
        "INSERT INTO permission_consents
         (id, user_id, source_id, source_kind, capability, scope, status, reason, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 'granted', ?, ?, ?)
         ON CONFLICT(user_id, source_id, capability, scope) DO UPDATE SET
           source_kind = excluded.source_kind,
           status = 'granted',
           reason = excluded.reason,
           updated_at = excluded.updated_at",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(&source_id)
    .bind(&source_kind)
    .bind(&capability)
    .bind(&scope)
    .bind(input.reason.trim())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    find(pool, user_id, &source_id, &capability, &scope).await
}

pub async fn revoke_for_user(
    pool: &SqlitePool,
    user_id: &str,
    input: PermissionConsentRevokeInput,
) -> AppResult<PermissionConsentRecord> {
    let source_id = normalize_required(&input.source_id, "sourceId")?;
    let capability = normalize_required(&input.capability, "capability")?;
    let scope = normalize_scope(&input.scope);
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE permission_consents
         SET status = 'revoked', updated_at = ?
         WHERE user_id = ? AND source_id = ? AND capability = ? AND scope = ?",
    )
    .bind(now)
    .bind(user_id)
    .bind(&source_id)
    .bind(&capability)
    .bind(&scope)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    find(pool, user_id, &source_id, &capability, &scope).await
}

pub async fn list_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> AppResult<Vec<PermissionConsentRecord>> {
    let rows = sqlx::query_as::<_, PermissionConsentRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, status, reason, created_at, updated_at
         FROM permission_consents
         WHERE user_id = ?
         ORDER BY source_id, capability, scope",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn is_granted(
    pool: &SqlitePool,
    user_id: &str,
    source_id: &str,
    capability: &str,
    requested_scope: &str,
) -> AppResult<bool> {
    let rows = sqlx::query_as::<_, PermissionConsentRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, status, reason, created_at, updated_at
         FROM permission_consents
         WHERE user_id = ? AND source_id = ? AND capability = ? AND status = 'granted'",
    )
    .bind(user_id)
    .bind(source_id)
    .bind(capability)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .any(|row| scope_allows(&row.scope, requested_scope)))
}

async fn find(
    pool: &SqlitePool,
    user_id: &str,
    source_id: &str,
    capability: &str,
    scope: &str,
) -> AppResult<PermissionConsentRecord> {
    let row = sqlx::query_as::<_, PermissionConsentRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, status, reason, created_at, updated_at
         FROM permission_consents
         WHERE user_id = ? AND source_id = ? AND capability = ? AND scope = ?",
    )
    .bind(user_id)
    .bind(source_id)
    .bind(capability)
    .bind(scope)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(row.into())
}

fn normalize_required(value: &str, field: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn normalize_kind(value: &str) -> String {
    match value.trim() {
        "" => "plugin".to_string(),
        value => value.to_string(),
    }
}

fn normalize_scope(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "*".to_string();
    }
    value.to_string()
}

fn scope_allows(granted_scope: &str, requested_scope: &str) -> bool {
    let granted_scope = granted_scope.trim();
    let requested_scope = requested_scope.trim();
    granted_scope == "*"
        || requested_scope.is_empty()
        || granted_scope == requested_scope
        || requested_scope.starts_with(&format!("{}/", granted_scope.trim_end_matches('/')))
}

struct SystemGrant {
    source_id: &'static str,
    capability: &'static str,
    scope: &'static str,
}

fn system_grants() -> Vec<SystemGrant> {
    vec![
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "browser.openDirectory",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "browser.openUrl",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "clipboard.read",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "clipboard.write",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "fs.read",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "fs.write",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "notification.send",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.capability-broker",
            capability: "process.exec",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.permission-core",
            capability: "permission.audit",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.permission-core",
            capability: "permission.consent",
            scope: "*",
        },
        SystemGrant {
            source_id: "platform.permission-core",
            capability: "permission.approval",
            scope: "*",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::scope_allows;

    #[test]
    fn scope_allows_exact_wildcard_and_child_path() {
        assert!(scope_allows("*", "/tmp/a.txt"));
        assert!(scope_allows("/tmp", "/tmp/a.txt"));
        assert!(scope_allows("/tmp/a.txt", "/tmp/a.txt"));
        assert!(!scope_allows("/tmp/a", "/tmp/abc"));
    }
}
