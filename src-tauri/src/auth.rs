use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    home_paths::home_path_for_roles,
};

const SESSION_TTL_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub real_name: String,
    pub avatar: String,
    pub home_path: String,
    pub roles: Vec<String>,
    pub desc: String,
    pub token: String,
}

#[derive(Debug, FromRow)]
struct UserInfoRow {
    user_id: String,
    username: String,
    real_name: String,
    avatar: String,
    home_path: String,
}

#[derive(Debug, FromRow)]
struct LoginUser {
    id: String,
    password_hash: String,
    status: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct SessionUser {
    pub user_id: String,
}

pub async fn login(pool: &SqlitePool, request: LoginRequest) -> AppResult<LoginResult> {
    let username = request.username.trim();
    if username.is_empty() || request.password.is_empty() {
        return Err(AppError::BadRequest("用户名和密码不能为空".to_string()));
    }

    let user = sqlx::query_as::<_, LoginUser>(
        "SELECT id, password_hash, status FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if user.status != "enabled" {
        return Err(AppError::Forbidden);
    }

    verify_password(&request.password, &user.password_hash)?;

    let token = new_id();
    let now = now_millis();
    let expires_at = now + SESSION_TTL_MILLIS;
    sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(&user.id)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(LoginResult {
        access_token: token,
    })
}

pub async fn logout(pool: &SqlitePool, token: String) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn require_session(pool: &SqlitePool, token: &str) -> AppResult<SessionUser> {
    let now = now_millis();
    let user = sqlx::query_as::<_, SessionUser>(
        "SELECT users.id AS user_id
         FROM sessions
         JOIN users ON users.id = sessions.user_id
         WHERE sessions.token = ? AND sessions.expires_at > ? AND users.status = 'enabled'",
    )
    .bind(token)
    .bind(now)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(user)
}

pub async fn current_user(pool: &SqlitePool, token: String) -> AppResult<UserInfo> {
    let session = require_session(pool, &token).await?;
    let roles = user_role_codes(pool, &session.user_id).await?;

    let row = sqlx::query_as::<_, UserInfoRow>(
        "SELECT id AS user_id, username, real_name, avatar, home_path
         FROM users
         WHERE id = ?",
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await?;

    Ok(UserInfo {
        user_id: row.user_id,
        username: row.username,
        real_name: row.real_name,
        avatar: row.avatar,
        home_path: home_path_for_roles(&row.home_path, &roles),
        roles,
        desc: String::new(),
        token,
    })
}

pub async fn user_role_codes(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT roles.code
         FROM roles
         JOIN user_roles ON user_roles.role_id = roles.id
         WHERE user_roles.user_id = ? AND roles.status = 'enabled'
         ORDER BY roles.code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

fn verify_password(password: &str, password_hash: &str) -> AppResult<()> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|error| AppError::Password(error.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::verify_password;
    use crate::db::hash_password;

    #[test]
    fn password_hash_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let hash = hash_password("admin123456")?;
        verify_password("admin123456", &hash)?;
        assert!(verify_password("wrong", &hash).is_err());
        Ok(())
    }
}
