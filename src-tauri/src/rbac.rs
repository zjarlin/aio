use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    home_paths::{home_path_for_roles, DEFAULT_HOME_PATH},
};

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub permission_type: String,
    pub path: String,
    pub component: String,
    pub icon: String,
    pub sort_order: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMenu {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub meta: RouteMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<RouteMenu>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMeta {
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affix_tab: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub d: Vec<T>,
    pub t: i64,
    pub p: PageInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub o: i64,
    pub s: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RoleRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleInput {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleUpdateInput {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub real_name: String,
    pub avatar: String,
    pub home_path: String,
    pub status: String,
    pub roles: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInput {
    pub username: String,
    pub password: String,
    pub real_name: String,
    pub avatar: Option<String>,
    pub home_path: Option<String>,
    pub status: Option<String>,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserUpdateInput {
    pub id: String,
    pub username: String,
    pub real_name: String,
    pub avatar: Option<String>,
    pub home_path: Option<String>,
    pub status: Option<String>,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPasswordInput {
    pub id: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignPermissionsInput {
    pub role_id: String,
    pub permission_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInput {
    pub id: Option<String>,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub permission_type: String,
    pub path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i64>,
    pub status: Option<String>,
}

pub fn normalize_page(request: &PageRequest) -> (i64, i64) {
    let offset = request.o.unwrap_or(0).max(0);
    let size = request.s.unwrap_or(20).clamp(1, 100);
    (offset, size)
}

pub async fn access_codes(pool: &SqlitePool, token: String) -> AppResult<Vec<String>> {
    let session = require_session(pool, &token).await?;
    let codes = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT permissions.code
         FROM permissions
         JOIN role_permissions ON role_permissions.permission_id = permissions.id
         JOIN user_roles ON user_roles.role_id = role_permissions.role_id
         JOIN roles ON roles.id = user_roles.role_id
         WHERE user_roles.user_id = ?
           AND permissions.permission_type = 'button'
           AND permissions.status = 'enabled'
           AND roles.status = 'enabled'
         ORDER BY permissions.code",
    )
    .bind(session.user_id)
    .fetch_all(pool)
    .await?;

    Ok(codes)
}

pub async fn menus(pool: &SqlitePool, token: String) -> AppResult<Vec<RouteMenu>> {
    let session = require_session(pool, &token).await?;
    let rows = sqlx::query_as::<_, PermissionNode>(
        "SELECT DISTINCT permissions.*
         FROM permissions
         JOIN role_permissions ON role_permissions.permission_id = permissions.id
         JOIN user_roles ON user_roles.role_id = role_permissions.role_id
         JOIN roles ON roles.id = user_roles.role_id
         WHERE user_roles.user_id = ?
           AND permissions.permission_type = 'menu'
           AND permissions.status = 'enabled'
           AND roles.status = 'enabled'
         ORDER BY permissions.sort_order, permissions.name",
    )
    .bind(session.user_id)
    .fetch_all(pool)
    .await?;

    Ok(build_routes(rows))
}

pub async fn permission_tree(pool: &SqlitePool, token: String) -> AppResult<Vec<PermissionNode>> {
    require_session(pool, &token).await?;
    let rows =
        sqlx::query_as::<_, PermissionNode>("SELECT * FROM permissions ORDER BY sort_order, name")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

pub async fn save_permission(
    pool: &SqlitePool,
    token: String,
    input: PermissionInput,
) -> AppResult<PermissionNode> {
    require_session(pool, &token).await?;
    let now = now_millis();
    let id = input.id.unwrap_or_else(new_id);
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM permissions WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool)
        .await?;

    if exists.is_some() {
        sqlx::query(
            "UPDATE permissions
             SET parent_id = ?, code = ?, name = ?, permission_type = ?, path = ?, component = ?,
                 icon = ?, sort_order = ?, status = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(input.parent_id)
        .bind(input.code)
        .bind(input.name)
        .bind(input.permission_type)
        .bind(input.path.unwrap_or_default())
        .bind(input.component.unwrap_or_default())
        .bind(input.icon.unwrap_or_default())
        .bind(input.sort_order.unwrap_or(0))
        .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
        .bind(now)
        .bind(&id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO permissions
             (id, parent_id, code, name, permission_type, path, component, icon, sort_order, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(input.parent_id)
        .bind(input.code)
        .bind(input.name)
        .bind(input.permission_type)
        .bind(input.path.unwrap_or_default())
        .bind(input.component.unwrap_or_default())
        .bind(input.icon.unwrap_or_default())
        .bind(input.sort_order.unwrap_or(0))
        .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    find_permission(pool, &id).await
}

pub async fn role_page(
    pool: &SqlitePool,
    token: String,
    request: PageRequest,
) -> AppResult<PageResult<RoleRecord>> {
    require_session(pool, &token).await?;
    let (offset, size) = normalize_page(&request);
    let keyword = format!("%{}%", request.keyword.unwrap_or_default());

    let total =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE code LIKE ? OR name LIKE ?")
            .bind(&keyword)
            .bind(&keyword)
            .fetch_one(pool)
            .await?;

    let rows = sqlx::query_as::<_, RoleRecord>(
        "SELECT * FROM roles
         WHERE code LIKE ? OR name LIKE ?
         ORDER BY created_at DESC
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

pub async fn user_page(
    pool: &SqlitePool,
    token: String,
    request: PageRequest,
) -> AppResult<PageResult<UserRecord>> {
    require_session(pool, &token).await?;
    let (offset, size) = normalize_page(&request);
    let keyword = format!("%{}%", request.keyword.unwrap_or_default());

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username LIKE ? OR real_name LIKE ?",
    )
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, UserRecord>(
        "SELECT users.id, users.username, users.real_name, users.avatar, users.home_path,
                users.status, users.created_at, users.updated_at,
                COALESCE(json_group_array(roles.code) FILTER (WHERE roles.code IS NOT NULL), '[]') AS roles
         FROM users
         LEFT JOIN user_roles ON user_roles.user_id = users.id
         LEFT JOIN roles ON roles.id = user_roles.role_id
         WHERE users.username LIKE ? OR users.real_name LIKE ?
         GROUP BY users.id
         ORDER BY users.created_at DESC
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

pub async fn create_user(
    pool: &SqlitePool,
    token: String,
    input: UserInput,
) -> AppResult<UserRecord> {
    require_session(pool, &token).await?;
    validate_user(&input.username, &input.real_name)?;
    if input.password.len() < 8 {
        return Err(AppError::BadRequest("密码长度至少 8 位".to_string()));
    }

    let now = now_millis();
    let id = new_id();
    let password_hash = crate::db::hash_password(&input.password)?;
    let home_path = resolve_input_home_path(pool, input.home_path, &input.role_ids).await?;
    sqlx::query(
        "INSERT INTO users
         (id, username, password_hash, real_name, avatar, home_path, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.username)
    .bind(password_hash)
    .bind(input.real_name)
    .bind(input.avatar.unwrap_or_default())
    .bind(home_path)
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    replace_user_roles(pool, &id, input.role_ids).await?;
    find_user(pool, &id).await
}

pub async fn update_user(
    pool: &SqlitePool,
    token: String,
    input: UserUpdateInput,
) -> AppResult<UserRecord> {
    require_session(pool, &token).await?;
    validate_user(&input.username, &input.real_name)?;
    let now = now_millis();
    let home_path = resolve_input_home_path(pool, input.home_path, &input.role_ids).await?;

    sqlx::query(
        "UPDATE users
         SET username = ?, real_name = ?, avatar = ?, home_path = ?, status = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.username)
    .bind(input.real_name)
    .bind(input.avatar.unwrap_or_default())
    .bind(home_path)
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(now)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    replace_user_roles(pool, &input.id, input.role_ids).await?;
    find_user(pool, &input.id).await
}

pub async fn disable_user(pool: &SqlitePool, token: String, id: String) -> AppResult<UserRecord> {
    let session = require_session(pool, &token).await?;
    if session.user_id == id {
        return Err(AppError::BadRequest("不能禁用当前登录用户".to_string()));
    }

    let now = now_millis();
    sqlx::query("UPDATE users SET status = 'disabled', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&id)
        .execute(pool)
        .await?;
    find_user(pool, &id).await
}

pub async fn reset_password(
    pool: &SqlitePool,
    token: String,
    input: UserPasswordInput,
) -> AppResult<()> {
    require_session(pool, &token).await?;
    if input.password.len() < 8 {
        return Err(AppError::BadRequest("密码长度至少 8 位".to_string()));
    }
    let now = now_millis();
    let password_hash = crate::db::hash_password(&input.password)?;
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
        .bind(now)
        .bind(input.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    let session = require_session(pool, &token).await?;
    if session.user_id == id {
        return Err(AppError::BadRequest("不能删除当前登录用户".to_string()));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_role(
    pool: &SqlitePool,
    token: String,
    input: RoleInput,
) -> AppResult<RoleRecord> {
    require_session(pool, &token).await?;
    validate_code_name(&input.code, &input.name)?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO roles (id, code, name, description, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.code)
    .bind(input.name)
    .bind(input.description.unwrap_or_default())
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_role(pool, &id).await
}

pub async fn update_role(
    pool: &SqlitePool,
    token: String,
    input: RoleUpdateInput,
) -> AppResult<RoleRecord> {
    require_session(pool, &token).await?;
    validate_code_name(&input.code, &input.name)?;
    let now = now_millis();

    sqlx::query(
        "UPDATE roles
         SET code = ?, name = ?, description = ?, status = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.code)
    .bind(input.name)
    .bind(input.description.unwrap_or_default())
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(now)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    find_role(pool, &input.id).await
}

pub async fn delete_role(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    let affected = sqlx::query("DELETE FROM roles WHERE id = ? AND code <> 'super_admin'")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(AppError::BadRequest(
            "内置角色不能删除或角色不存在".to_string(),
        ));
    }
    Ok(())
}

pub async fn assign_permissions(
    pool: &SqlitePool,
    token: String,
    input: AssignPermissionsInput,
) -> AppResult<()> {
    require_session(pool, &token).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
        .bind(&input.role_id)
        .execute(&mut *tx)
        .await?;

    for permission_id in input.permission_ids {
        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
            .bind(&input.role_id)
            .bind(permission_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn role_permission_ids(
    pool: &SqlitePool,
    token: String,
    role_id: String,
) -> AppResult<Vec<String>> {
    require_session(pool, &token).await?;
    find_role(pool, &role_id).await?;

    let ids = sqlx::query_scalar::<_, String>(
        "SELECT permission_id
         FROM role_permissions
         WHERE role_id = ?
         ORDER BY permission_id",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

fn build_routes(nodes: Vec<PermissionNode>) -> Vec<RouteMenu> {
    let included: HashSet<String> = nodes.iter().map(|node| node.id.clone()).collect();
    let mut children_map: HashMap<Option<String>, Vec<PermissionNode>> = HashMap::new();
    for node in nodes {
        let parent_key = node
            .parent_id
            .clone()
            .filter(|parent_id| included.contains(parent_id));
        children_map.entry(parent_key).or_default().push(node);
    }

    fn build_node(
        node: PermissionNode,
        children_map: &mut HashMap<Option<String>, Vec<PermissionNode>>,
    ) -> RouteMenu {
        let children = children_map
            .remove(&Some(node.id.clone()))
            .unwrap_or_default()
            .into_iter()
            .map(|child| build_node(child, children_map))
            .collect::<Vec<_>>();
        let redirect = children.first().map(|child| child.path.clone());

        RouteMenu {
            name: route_name(&node.code),
            path: node.path,
            component: if node.component.is_empty() {
                None
            } else {
                Some(node.component)
            },
            meta: RouteMeta {
                title: node.name,
                icon: node.icon,
                order: Some(node.sort_order),
                affix_tab: if node.code == "dashboard:analytics" {
                    Some(true)
                } else {
                    None
                },
            },
            redirect,
            children,
        }
    }

    children_map
        .remove(&None)
        .unwrap_or_default()
        .into_iter()
        .map(|node| build_node(node, &mut children_map))
        .collect()
}

fn route_name(code: &str) -> String {
    code.split(':')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

async fn find_permission(pool: &SqlitePool, id: &str) -> AppResult<PermissionNode> {
    sqlx::query_as::<_, PermissionNode>("SELECT * FROM permissions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn find_role(pool: &SqlitePool, id: &str) -> AppResult<RoleRecord> {
    sqlx::query_as::<_, RoleRecord>("SELECT * FROM roles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn find_user(pool: &SqlitePool, id: &str) -> AppResult<UserRecord> {
    sqlx::query_as::<_, UserRecord>(
        "SELECT users.id, users.username, users.real_name, users.avatar, users.home_path,
                users.status, users.created_at, users.updated_at,
                COALESCE(json_group_array(roles.code) FILTER (WHERE roles.code IS NOT NULL), '[]') AS roles
         FROM users
         LEFT JOIN user_roles ON user_roles.user_id = users.id
         LEFT JOIN roles ON roles.id = user_roles.role_id
         WHERE users.id = ?
         GROUP BY users.id",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

async fn replace_user_roles(
    pool: &SqlitePool,
    user_id: &str,
    role_ids: Vec<String>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    for role_id in role_ids {
        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn resolve_input_home_path(
    pool: &SqlitePool,
    home_path: Option<String>,
    role_ids: &[String],
) -> AppResult<String> {
    let normalized = home_path.unwrap_or_default().trim().to_string();
    if !normalized.is_empty() {
        return Ok(normalized);
    }

    let roles = role_codes_by_ids(pool, role_ids).await?;
    Ok(home_path_for_roles(DEFAULT_HOME_PATH, &roles))
}

async fn role_codes_by_ids(pool: &SqlitePool, role_ids: &[String]) -> AppResult<Vec<String>> {
    let mut roles = Vec::with_capacity(role_ids.len());
    for role_id in role_ids {
        if let Some(code) = sqlx::query_scalar::<_, String>(
            "SELECT code FROM roles WHERE id = ? AND status = 'enabled'",
        )
        .bind(role_id)
        .fetch_optional(pool)
        .await?
        {
            roles.push(code);
        }
    }
    Ok(roles)
}

fn validate_user(username: &str, real_name: &str) -> AppResult<()> {
    if username.trim().is_empty() || real_name.trim().is_empty() {
        return Err(AppError::BadRequest("用户名和姓名不能为空".to_string()));
    }
    Ok(())
}

fn validate_code_name(code: &str, name: &str) -> AppResult<()> {
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(AppError::BadRequest("编码和名称不能为空".to_string()));
    }
    Ok(())
}

pub fn map_unique_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.message().contains("UNIQUE") {
            return AppError::Conflict("编码或值已存在".to_string());
        }
    }
    AppError::Database { source: error }
}

#[cfg(test)]
mod tests {
    use super::{build_routes, PermissionNode};

    #[test]
    fn routes_preserve_children() {
        let routes = build_routes(vec![
            PermissionNode {
                id: "root".to_string(),
                parent_id: None,
                code: "system".to_string(),
                name: "系统".to_string(),
                permission_type: "menu".to_string(),
                path: "/system".to_string(),
                component: "BasicLayout".to_string(),
                icon: "i".to_string(),
                sort_order: 1,
                status: "enabled".to_string(),
            },
            PermissionNode {
                id: "child".to_string(),
                parent_id: Some("root".to_string()),
                code: "system:user".to_string(),
                name: "用户".to_string(),
                permission_type: "menu".to_string(),
                path: "/system/users".to_string(),
                component: "/system/users/index".to_string(),
                icon: "i".to_string(),
                sort_order: 2,
                status: "enabled".to_string(),
            },
        ]);

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].children.len(), 1);
        assert_eq!(routes[0].redirect.as_deref(), Some("/system/users"));
        assert_eq!(routes[0].component.as_deref(), Some("BasicLayout"));
    }

    #[test]
    fn route_groups_omit_empty_component() {
        let routes = build_routes(vec![
            PermissionNode {
                id: "root".to_string(),
                parent_id: None,
                code: "assets".to_string(),
                name: "资产".to_string(),
                permission_type: "menu".to_string(),
                path: "/assets".to_string(),
                component: "BasicLayout".to_string(),
                icon: "i".to_string(),
                sort_order: 1,
                status: "enabled".to_string(),
            },
            PermissionNode {
                id: "group".to_string(),
                parent_id: Some("root".to_string()),
                code: "assets:agents".to_string(),
                name: "智能体".to_string(),
                permission_type: "menu".to_string(),
                path: "/assets/agents".to_string(),
                component: String::new(),
                icon: "i".to_string(),
                sort_order: 2,
                status: "enabled".to_string(),
            },
            PermissionNode {
                id: "child".to_string(),
                parent_id: Some("group".to_string()),
                code: "assets:agents:preferences".to_string(),
                name: "AGENTS.md 管理".to_string(),
                permission_type: "menu".to_string(),
                path: "/assets/agent-preferences".to_string(),
                component: "/assets/agent-preferences/index".to_string(),
                icon: "i".to_string(),
                sort_order: 3,
                status: "enabled".to_string(),
            },
        ]);

        let agents = &routes[0].children[0];
        assert_eq!(agents.path, "/assets/agents");
        assert!(agents.component.is_none());
        assert_eq!(agents.children.len(), 1);
        assert_eq!(agents.redirect.as_deref(), Some("/assets/agent-preferences"));
    }
}
