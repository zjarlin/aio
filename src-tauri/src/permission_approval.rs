use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    permission_consent::{self, PermissionConsentGrantInput},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionApprovalRecord {
    pub id: String,
    pub user_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub capability: String,
    pub scope: String,
    pub target: String,
    pub status: String,
    pub reason: String,
    pub decision_reason: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub decided_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionApprovalRequestInput {
    pub source_id: String,
    #[serde(default)]
    pub source_kind: String,
    pub capability: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionApprovalDecisionInput {
    pub id: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, FromRow)]
struct PermissionApprovalRow {
    id: String,
    user_id: String,
    source_id: String,
    source_kind: String,
    capability: String,
    scope: String,
    target: String,
    status: String,
    reason: String,
    decision_reason: String,
    created_at: i64,
    updated_at: i64,
    decided_at: Option<i64>,
}

impl From<PermissionApprovalRow> for PermissionApprovalRecord {
    fn from(row: PermissionApprovalRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            source_id: row.source_id,
            source_kind: row.source_kind,
            capability: row.capability,
            scope: row.scope,
            target: row.target,
            status: row.status,
            reason: row.reason,
            decision_reason: row.decision_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
            decided_at: row.decided_at,
        }
    }
}

pub async fn request_for_user(
    pool: &SqlitePool,
    user_id: &str,
    input: PermissionApprovalRequestInput,
) -> AppResult<PermissionApprovalRecord> {
    let source_id = normalize_required(&input.source_id, "sourceId")?;
    let source_kind = normalize_kind(&input.source_kind);
    let capability = normalize_required(&input.capability, "capability")?;
    let scope = normalize_scope(&input.scope);
    let target = normalize_target(&input.target, &scope);
    let reason = input.reason.trim().to_string();
    let now = now_millis();

    if let Some(existing) =
        find_pending(pool, user_id, &source_id, &capability, &scope, &target).await?
    {
        sqlx::query(
            "UPDATE permission_approval_requests
             SET source_kind = ?, reason = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&source_kind)
        .bind(&reason)
        .bind(now)
        .bind(&existing.id)
        .execute(pool)
        .await?;
        return find_by_id(pool, user_id, &existing.id).await;
    }

    let id = new_id();
    sqlx::query(
        "INSERT INTO permission_approval_requests
         (id, user_id, source_id, source_kind, capability, scope, target, status, reason, decision_reason, created_at, updated_at, decided_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, '', ?, ?, NULL)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(&source_id)
    .bind(&source_kind)
    .bind(&capability)
    .bind(&scope)
    .bind(&target)
    .bind(&reason)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    find_by_id(pool, user_id, &id).await
}

pub async fn list_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> AppResult<Vec<PermissionApprovalRecord>> {
    let rows = sqlx::query_as::<_, PermissionApprovalRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, target, status, reason, decision_reason, created_at, updated_at, decided_at
         FROM permission_approval_requests
         WHERE user_id = ?
         ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn approve_for_user(
    pool: &SqlitePool,
    user_id: &str,
    input: PermissionApprovalDecisionInput,
) -> AppResult<PermissionApprovalRecord> {
    let id = normalize_required(&input.id, "id")?;
    let record = find_by_id(pool, user_id, &id).await?;
    if record.status != "pending" {
        return Err(AppError::Conflict(format!(
            "approval request {} is already {}",
            record.id, record.status
        )));
    }

    permission_consent::grant_for_user(
        pool,
        user_id,
        PermissionConsentGrantInput {
            source_id: record.source_id.clone(),
            source_kind: record.source_kind.clone(),
            capability: record.capability.clone(),
            scope: record.scope.clone(),
            reason: if input.reason.trim().is_empty() {
                format!("approved request {}", record.id)
            } else {
                input.reason.trim().to_string()
            },
        },
    )
    .await?;

    decide(pool, user_id, &record.id, "approved", input.reason.trim()).await
}

pub async fn deny_for_user(
    pool: &SqlitePool,
    user_id: &str,
    input: PermissionApprovalDecisionInput,
) -> AppResult<PermissionApprovalRecord> {
    let id = normalize_required(&input.id, "id")?;
    let record = find_by_id(pool, user_id, &id).await?;
    if record.status != "pending" {
        return Err(AppError::Conflict(format!(
            "approval request {} is already {}",
            record.id, record.status
        )));
    }

    decide(pool, user_id, &record.id, "denied", input.reason.trim()).await
}

async fn decide(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    status: &str,
    decision_reason: &str,
) -> AppResult<PermissionApprovalRecord> {
    let now = now_millis();
    let affected = sqlx::query(
        "UPDATE permission_approval_requests
         SET status = ?, decision_reason = ?, updated_at = ?, decided_at = ?
         WHERE id = ? AND user_id = ? AND status = 'pending'",
    )
    .bind(status)
    .bind(decision_reason)
    .bind(now)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::Conflict(format!(
            "approval request {id} is not pending"
        )));
    }

    find_by_id(pool, user_id, id).await
}

async fn find_pending(
    pool: &SqlitePool,
    user_id: &str,
    source_id: &str,
    capability: &str,
    scope: &str,
    target: &str,
) -> AppResult<Option<PermissionApprovalRecord>> {
    let row = sqlx::query_as::<_, PermissionApprovalRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, target, status, reason, decision_reason, created_at, updated_at, decided_at
         FROM permission_approval_requests
         WHERE user_id = ? AND source_id = ? AND capability = ? AND scope = ? AND target = ? AND status = 'pending'
         LIMIT 1",
    )
    .bind(user_id)
    .bind(source_id)
    .bind(capability)
    .bind(scope)
    .bind(target)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

async fn find_by_id(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
) -> AppResult<PermissionApprovalRecord> {
    let row = sqlx::query_as::<_, PermissionApprovalRow>(
        "SELECT id, user_id, source_id, source_kind, capability, scope, target, status, reason, decision_reason, created_at, updated_at, decided_at
         FROM permission_approval_requests
         WHERE user_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(id)
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

fn normalize_target(value: &str, scope: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return scope.to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{
        auth::{self, LoginRequest},
        db::{connect, migrate_and_seed},
        permission_approval::{
            approve_for_user, deny_for_user, list_for_user, request_for_user,
            PermissionApprovalDecisionInput, PermissionApprovalRequestInput,
        },
        permission_consent,
    };

    fn with_seeded_pool<F, Fut>(test: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(sqlx::SqlitePool, String) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn Error>>>,
    {
        tauri::async_runtime::block_on(async {
            let dir = tempfile::tempdir()?;
            let pool = connect(dir.path().join("approval-test.sqlite")).await?;
            migrate_and_seed(&pool).await?;
            let login = auth::login(
                &pool,
                LoginRequest {
                    username: "admin".to_string(),
                    password: "admin123456".to_string(),
                },
            )
            .await?;
            let user = auth::require_session(&pool, &login.access_token).await?;
            let result = test(pool.clone(), user.user_id).await;
            pool.close().await;
            result
        })
    }

    #[test]
    fn approval_request_is_deduplicated_and_approval_grants_consent() -> Result<(), Box<dyn Error>>
    {
        with_seeded_pool(|pool, user_id| async move {
            let input = PermissionApprovalRequestInput {
                source_id: "third-party.files".to_string(),
                source_kind: "plugin".to_string(),
                capability: "fs.read".to_string(),
                scope: "/tmp/demo".to_string(),
                target: "/tmp/demo/file.txt".to_string(),
                reason: "missing consent".to_string(),
            };
            let first = request_for_user(&pool, &user_id, input.clone()).await?;
            let second = request_for_user(&pool, &user_id, input).await?;
            assert_eq!(first.id, second.id);
            assert_eq!(list_for_user(&pool, &user_id).await?.len(), 1);

            let approved = approve_for_user(
                &pool,
                &user_id,
                PermissionApprovalDecisionInput {
                    id: first.id,
                    reason: "ok".to_string(),
                },
            )
            .await?;

            assert_eq!(approved.status, "approved");
            assert!(
                permission_consent::is_granted(
                    &pool,
                    &user_id,
                    "third-party.files",
                    "fs.read",
                    "/tmp/demo/file.txt"
                )
                .await?
            );
            Ok(())
        })
    }

    #[test]
    fn denied_approval_does_not_grant_consent() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool, user_id| async move {
            let request = request_for_user(
                &pool,
                &user_id,
                PermissionApprovalRequestInput {
                    source_id: "third-party.files".to_string(),
                    source_kind: "plugin".to_string(),
                    capability: "fs.write".to_string(),
                    scope: "/tmp/nope".to_string(),
                    target: "/tmp/nope".to_string(),
                    reason: "missing consent".to_string(),
                },
            )
            .await?;

            let denied = deny_for_user(
                &pool,
                &user_id,
                PermissionApprovalDecisionInput {
                    id: request.id,
                    reason: "no".to_string(),
                },
            )
            .await?;

            assert_eq!(denied.status, "denied");
            assert!(
                !permission_consent::is_granted(
                    &pool,
                    &user_id,
                    "third-party.files",
                    "fs.write",
                    "/tmp/nope"
                )
                .await?
            );
            Ok(())
        })
    }
}
