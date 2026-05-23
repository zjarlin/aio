use std::{collections::HashMap, path::PathBuf};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub async fn connect(path: PathBuf) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(options).await?;
    Ok(pool)
}

pub async fn migrate_and_seed(pool: &SqlitePool) -> AppResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    seed_defaults(pool).await?;
    ensure_builtin_roles(pool).await?;
    ensure_default_permissions(pool).await?;
    crate::asset_items::backfill_file_variables(pool).await?;
    crate::asset_items::refresh_page_global_variables(pool).await?;
    Ok(())
}

pub fn now_millis() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() as i64 / 1_000_000
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Password(error.to_string()))
}

async fn seed_defaults(pool: &SqlitePool) -> AppResult<()> {
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count > 0 {
        return Ok(());
    }

    let now = now_millis();
    let admin_id = new_id();
    let role_id = new_id();
    let password_hash = hash_password("admin123456")?;

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, real_name, avatar, home_path, status, created_at, updated_at)
         VALUES (?, 'admin', ?, '系统管理员', '', '/analytics', 'enabled', ?, ?)",
    )
    .bind(&admin_id)
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO roles (id, code, name, description, status, created_at, updated_at)
         VALUES (?, 'super_admin', '超级管理员', '内置超级管理员角色', 'enabled', ?, ?)",
    )
    .bind(&role_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
        .bind(&admin_id)
        .bind(&role_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO roles (id, code, name, description, status, created_at, updated_at)
         VALUES (?, 'ordinary_user', '普通用户', '普通本地用户角色', 'enabled', ?, ?)",
    )
    .bind(new_id())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let permissions = default_permissions();
    for permission in &permissions {
        sqlx::query(
            "INSERT INTO permissions
             (id, parent_id, code, name, permission_type, path, component, icon, sort_order, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'enabled', ?, ?)",
        )
        .bind(permission.id)
        .bind(permission.parent_id)
        .bind(permission.code)
        .bind(permission.name)
        .bind(permission.permission_type)
        .bind(permission.path)
        .bind(permission.component)
        .bind(permission.icon)
        .bind(permission.sort_order)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
            .bind(&role_id)
            .bind(permission.id)
            .execute(pool)
            .await?;
    }

    let dict_type_id = new_id();
    sqlx::query(
        "INSERT INTO dict_types (id, code, name, description, status, sort_order, created_at, updated_at)
         VALUES (?, 'common_status', '通用状态', '启用和禁用状态', 'enabled', 10, ?, ?)",
    )
    .bind(&dict_type_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    for (label, value, sort_order) in [("启用", "enabled", 10), ("禁用", "disabled", 20)] {
        sqlx::query(
            "INSERT INTO dict_items (id, type_id, label, value, status, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'enabled', ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&dict_type_id)
        .bind(label)
        .bind(value)
        .bind(sort_order)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn ensure_default_permissions(pool: &SqlitePool) -> AppResult<()> {
    let now = now_millis();
    let super_role_ids =
        sqlx::query_scalar::<_, String>("SELECT id FROM roles WHERE code = 'super_admin'")
            .fetch_all(pool)
            .await?;
    let ordinary_role_ids =
        sqlx::query_scalar::<_, String>("SELECT id FROM roles WHERE code = 'ordinary_user'")
            .fetch_all(pool)
            .await?;
    let mut actual_ids = HashMap::new();

    for permission in default_permissions() {
        let parent_id = permission
            .parent_id
            .and_then(|seed_id| actual_ids.get(seed_id).cloned());

        sqlx::query(
            "INSERT INTO permissions
             (id, parent_id, code, name, permission_type, path, component, icon, sort_order, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'enabled', ?, ?)
             ON CONFLICT(code) DO UPDATE SET
               parent_id = excluded.parent_id,
               name = excluded.name,
               permission_type = excluded.permission_type,
               path = excluded.path,
               component = excluded.component,
               icon = excluded.icon,
               sort_order = excluded.sort_order,
               status = excluded.status,
               updated_at = excluded.updated_at",
        )
        .bind(permission.id)
        .bind(parent_id)
        .bind(permission.code)
        .bind(permission.name)
        .bind(permission.permission_type)
        .bind(permission.path)
        .bind(permission.component)
        .bind(permission.icon)
        .bind(permission.sort_order)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        let permission_id =
            sqlx::query_scalar::<_, String>("SELECT id FROM permissions WHERE code = ? LIMIT 1")
                .bind(permission.code)
                .fetch_optional(pool)
                .await?;

        if let Some(permission_id) = permission_id {
            actual_ids.insert(permission.id, permission_id.clone());

            for role_id in &super_role_ids {
                sqlx::query(
                    "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                )
                .bind(role_id)
                .bind(&permission_id)
                .execute(pool)
                .await?;
            }

            if ordinary_user_permission_codes().contains(&permission.code) {
                for role_id in &ordinary_role_ids {
                    sqlx::query(
                        "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                    )
                    .bind(role_id)
                    .bind(&permission_id)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

async fn ensure_builtin_roles(pool: &SqlitePool) -> AppResult<()> {
    let now = now_millis();
    for (code, name, description) in [
        ("super_admin", "超级管理员", "内置超级管理员角色"),
        ("ordinary_user", "普通用户", "普通本地用户角色"),
    ] {
        sqlx::query(
            "INSERT INTO roles (id, code, name, description, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'enabled', ?, ?)
             ON CONFLICT(code) DO UPDATE SET
               name = excluded.name,
               description = excluded.description,
               status = excluded.status,
               updated_at = excluded.updated_at",
        )
        .bind(new_id())
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

struct SeedPermission {
    id: &'static str,
    parent_id: Option<&'static str>,
    code: &'static str,
    name: &'static str,
    permission_type: &'static str,
    path: &'static str,
    component: &'static str,
    icon: &'static str,
    sort_order: i64,
}

fn default_permissions() -> Vec<SeedPermission> {
    vec![
        SeedPermission {
            id: "perm-dashboard",
            parent_id: None,
            code: "dashboard",
            name: "概览",
            permission_type: "menu",
            path: "/",
            component: "BasicLayout",
            icon: "lucide:layout-dashboard",
            sort_order: -1,
        },
        SeedPermission {
            id: "perm-dashboard-analytics",
            parent_id: Some("perm-dashboard"),
            code: "dashboard:analytics",
            name: "分析页",
            permission_type: "menu",
            path: "/analytics",
            component: "/dashboard/analytics/index",
            icon: "lucide:area-chart",
            sort_order: 10,
        },
        SeedPermission {
            id: "perm-dashboard-workspace",
            parent_id: Some("perm-dashboard"),
            code: "dashboard:workspace",
            name: "工作台",
            permission_type: "menu",
            path: "/workspace",
            component: "/dashboard/workspace/index",
            icon: "carbon:workspace",
            sort_order: 20,
        },
        SeedPermission {
            id: "perm-system",
            parent_id: None,
            code: "system",
            name: "系统管理",
            permission_type: "menu",
            path: "/system",
            component: "BasicLayout",
            icon: "lucide:settings",
            sort_order: 100,
        },
        SeedPermission {
            id: "perm-system-user",
            parent_id: Some("perm-system"),
            code: "system:user",
            name: "用户管理",
            permission_type: "menu",
            path: "/system/users",
            component: "/system/users/index",
            icon: "lucide:users",
            sort_order: 10,
        },
        SeedPermission {
            id: "perm-system-role",
            parent_id: Some("perm-system"),
            code: "system:role",
            name: "角色管理",
            permission_type: "menu",
            path: "/system/roles",
            component: "/system/roles/index",
            icon: "lucide:shield-check",
            sort_order: 20,
        },
        SeedPermission {
            id: "perm-system-permission",
            parent_id: Some("perm-system"),
            code: "system:permission",
            name: "权限管理",
            permission_type: "menu",
            path: "/system/permissions",
            component: "/system/permissions/index",
            icon: "lucide:key-round",
            sort_order: 30,
        },
        SeedPermission {
            id: "perm-system-dict",
            parent_id: Some("perm-system"),
            code: "system:dict",
            name: "字典管理",
            permission_type: "menu",
            path: "/system/dictionaries",
            component: "/system/dictionaries/index",
            icon: "lucide:book-type",
            sort_order: 40,
        },
        SeedPermission {
            id: "perm-notes",
            parent_id: None,
            code: "assets",
            name: "资产",
            permission_type: "menu",
            path: "/assets",
            component: "BasicLayout",
            icon: "lucide:archive",
            sort_order: 200,
        },
        SeedPermission {
            id: "perm-notes-list",
            parent_id: Some("perm-notes"),
            code: "assets:notes",
            name: "笔记",
            permission_type: "menu",
            path: "/assets/notes",
            component: "/assets/notes/index",
            icon: "lucide:sticky-note",
            sort_order: 10,
        },
        SeedPermission {
            id: "perm-system-skill",
            parent_id: Some("perm-notes"),
            code: "assets:skill",
            name: "技能管理",
            permission_type: "menu",
            path: "/assets/skills",
            component: "/assets/skills/index",
            icon: "lucide:sparkles",
            sort_order: 20,
        },
        SeedPermission {
            id: "perm-assets-agent-preferences",
            parent_id: Some("perm-notes"),
            code: "assets:agent_preferences",
            name: "AGENTS.md 管理",
            permission_type: "menu",
            path: "/assets/agent-preferences",
            component: "/assets/agent-preferences/index",
            icon: "lucide:bot",
            sort_order: 25,
        },
        SeedPermission {
            id: "perm-assets-openai-assistant",
            parent_id: Some("perm-notes"),
            code: "assets:openai_assistant",
            name: "OpenAI 助手",
            permission_type: "menu",
            path: "/assets/openai-assistant",
            component: "/assets/openai-assistant/index",
            icon: "lucide:bot",
            sort_order: 26,
        },
        SeedPermission {
            id: "perm-assets-docker-compose",
            parent_id: Some("perm-notes"),
            code: "assets:docker_compose",
            name: "Docker Compose",
            permission_type: "menu",
            path: "/assets/docker-compose",
            component: "/assets/docker-compose/index",
            icon: "lucide:container",
            sort_order: 30,
        },
        SeedPermission {
            id: "perm-assets-cli",
            parent_id: Some("perm-notes"),
            code: "assets:cli",
            name: "CLI 管理",
            permission_type: "menu",
            path: "/assets/cli",
            component: "/assets/cli/index",
            icon: "lucide:terminal",
            sort_order: 40,
        },
        SeedPermission {
            id: "perm-assets-env-vars",
            parent_id: Some("perm-notes"),
            code: "assets:env_vars",
            name: "环境变量管理",
            permission_type: "menu",
            path: "/assets/env-vars",
            component: "/assets/env-vars/index",
            icon: "lucide:variable",
            sort_order: 50,
        },
        SeedPermission {
            id: "perm-assets-bash-functions",
            parent_id: Some("perm-notes"),
            code: "assets:bash_functions",
            name: "Bash 函数管理",
            permission_type: "menu",
            path: "/assets/bash-functions",
            component: "/assets/bash-functions/index",
            icon: "lucide:square-function",
            sort_order: 60,
        },
        SeedPermission {
            id: "perm-assets-dotfiles",
            parent_id: Some("perm-notes"),
            code: "assets:dotfiles",
            name: "dotfiles 管理",
            permission_type: "menu",
            path: "/assets/dotfiles",
            component: "/assets/dotfiles/index",
            icon: "lucide:file-cog",
            sort_order: 70,
        },
        SeedPermission {
            id: "perm-action-create",
            parent_id: None,
            code: "action:create",
            name: "新增",
            permission_type: "button",
            path: "",
            component: "",
            icon: "",
            sort_order: 1000,
        },
        SeedPermission {
            id: "perm-action-update",
            parent_id: None,
            code: "action:update",
            name: "编辑",
            permission_type: "button",
            path: "",
            component: "",
            icon: "",
            sort_order: 1010,
        },
        SeedPermission {
            id: "perm-action-delete",
            parent_id: None,
            code: "action:delete",
            name: "删除",
            permission_type: "button",
            path: "",
            component: "",
            icon: "",
            sort_order: 1020,
        },
    ]
}

fn ordinary_user_permission_codes() -> &'static [&'static str] {
    &[
        "dashboard",
        "dashboard:analytics",
        "dashboard:workspace",
        "assets",
        "assets:notes",
        "assets:skill",
        "assets:agent_preferences",
        "assets:openai_assistant",
        "assets:docker_compose",
        "assets:cli",
        "assets:env_vars",
        "assets:bash_functions",
        "assets:dotfiles",
    ]
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs};

    use sqlx::SqlitePool;

    use super::{connect, migrate_and_seed};
    use crate::{
        agent_preferences::{
            self, AgentPreferenceInput, AgentPreferencePageRequest, AgentPreferenceToggleInput,
            AgentPreferenceUpdateInput,
        },
        auth::{self, LoginRequest},
        dictionary::{self, DictItemInput, DictItemPageRequest, DictTypeInput},
        error::AppError,
        home_paths::ORDINARY_USER_HOME_PATH,
        notes::{self, NoteInput, NotePageRequest},
        rbac::{self, PageRequest, UserInput},
        skills::{self, SkillInput, SkillPageRequest, SkillToggleInput, SkillUpdateInput},
    };

    fn with_seeded_pool<F, Fut>(test: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(SqlitePool) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn Error>>>,
    {
        tauri::async_runtime::block_on(async {
            let dir = tempfile::tempdir()?;
            let pool = connect(dir.path().join("aio-test.sqlite")).await?;
            migrate_and_seed(&pool).await?;

            let result = test(pool.clone()).await;
            pool.close().await;
            result
        })
    }

    async fn login_admin(pool: &SqlitePool) -> Result<String, Box<dyn Error>> {
        let login = auth::login(
            pool,
            LoginRequest {
                username: "admin".to_string(),
                password: "admin123456".to_string(),
            },
        )
        .await?;
        Ok(login.access_token)
    }

    #[test]
    fn migration_seeds_admin_role_permissions_and_dictionary() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let password_hash: String =
                sqlx::query_scalar("SELECT password_hash FROM users WHERE username = 'admin'")
                    .fetch_one(&pool)
                    .await?;
            assert_ne!(password_hash, "admin123456");
            assert!(password_hash.starts_with("$argon2"));

            let role_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE code = 'super_admin'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(role_count, 1);

            let ordinary_role_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE code = 'ordinary_user'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(ordinary_role_count, 1);

            let system_menu_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE code = 'system:user'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(system_menu_count, 1);

            let skill_menu_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE code = 'assets:skill'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(skill_menu_count, 1);

            let asset_menu_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM permissions WHERE code = 'assets:docker_compose'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(asset_menu_count, 1);

            let agent_preference_menu_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM permissions WHERE code = 'assets:agent_preferences'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(agent_preference_menu_count, 1);

            let dict_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM dict_types WHERE code = 'common_status'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(dict_count, 1);
            Ok(())
        })
    }

    #[test]
    fn auth_session_validation_rejects_logged_out_token() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            auth::require_session(&pool, &token).await?;

            auth::logout(&pool, token.clone()).await?;
            let error = auth::require_session(&pool, &token).await.unwrap_err();
            assert!(matches!(error, AppError::Unauthorized));
            Ok(())
        })
    }

    #[test]
    fn rbac_expands_admin_menus_and_button_codes() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;

            let codes = rbac::access_codes(&pool, token.clone()).await?;
            assert!(codes.iter().any(|code| code == "action:create"));
            assert!(codes.iter().any(|code| code == "action:update"));
            assert!(codes.iter().any(|code| code == "action:delete"));

            let menus = rbac::menus(&pool, token).await?;
            let system = menus.iter().find(|menu| menu.path == "/system").unwrap();
            assert!(system
                .children
                .iter()
                .any(|child| child.path == "/system/users"));
            assert!(!system
                .children
                .iter()
                .any(|child| child.path == "/assets/skills"));

            let assets = menus.iter().find(|menu| menu.path == "/assets").unwrap();
            assert!(assets
                .children
                .iter()
                .any(|child| child.path == "/assets/notes"));
            assert!(assets
                .children
                .iter()
                .any(|child| child.path == "/assets/skills"));
            assert!(assets
                .children
                .iter()
                .any(|child| child.path == "/assets/agent-preferences"));
            assert!(assets
                .children
                .iter()
                .any(|child| child.path == "/assets/openai-assistant"));
            Ok(())
        })
    }

    #[test]
    fn default_permissions_are_repaired_for_existing_databases() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            sqlx::query("DELETE FROM role_permissions WHERE permission_id = 'perm-system-skill'")
                .execute(&pool)
                .await?;
            sqlx::query("DELETE FROM permissions WHERE id = 'perm-system-skill'")
                .execute(&pool)
                .await?;

            super::ensure_default_permissions(&pool).await?;

            let grants: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM role_permissions
                 JOIN roles ON roles.id = role_permissions.role_id
                 JOIN permissions ON permissions.id = role_permissions.permission_id
                 WHERE roles.code = 'super_admin'
                   AND role_permissions.permission_id = 'perm-system-skill'
                   AND permissions.code = 'assets:skill'
                   AND permissions.parent_id = 'perm-notes'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(grants, 1);

            let ordinary_grants: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM role_permissions
                 JOIN roles ON roles.id = role_permissions.role_id
                 JOIN permissions ON permissions.id = role_permissions.permission_id
                 WHERE roles.code = 'ordinary_user'
                   AND permissions.code = 'assets:skill'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(ordinary_grants, 1);

            let agent_preference_grants: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM role_permissions
                 JOIN roles ON roles.id = role_permissions.role_id
                 JOIN permissions ON permissions.id = role_permissions.permission_id
                 WHERE roles.code = 'ordinary_user'
                   AND permissions.code = 'assets:agent_preferences'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(agent_preference_grants, 1);

            let assistant_grants: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM role_permissions
                 JOIN roles ON roles.id = role_permissions.role_id
                 JOIN permissions ON permissions.id = role_permissions.permission_id
                 WHERE roles.code = 'ordinary_user'
                   AND permissions.code = 'assets:openai_assistant'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(assistant_grants, 1);
            Ok(())
        })
    }

    #[test]
    fn default_permissions_reuse_existing_ids_when_codes_match() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            sqlx::query("DELETE FROM role_permissions WHERE permission_id = 'perm-system-skill'")
                .execute(&pool)
                .await?;
            sqlx::query(
                "UPDATE permissions SET id = 'custom-skill-id' WHERE code = 'assets:skill'",
            )
            .execute(&pool)
            .await?;

            super::ensure_default_permissions(&pool).await?;

            let permission_id: String =
                sqlx::query_scalar("SELECT id FROM permissions WHERE code = 'assets:skill'")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(permission_id, "custom-skill-id");

            let grants: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM role_permissions
                 JOIN roles ON roles.id = role_permissions.role_id
                 WHERE roles.code IN ('super_admin', 'ordinary_user')
                   AND role_permissions.permission_id = 'custom-skill-id'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(grants, 2);
            Ok(())
        })
    }

    #[test]
    fn ordinary_user_can_see_asset_menus() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let admin_token = login_admin(&pool).await?;
            let roles = rbac::role_page(
                &pool,
                admin_token.clone(),
                PageRequest {
                    o: Some(0),
                    s: Some(20),
                    keyword: Some("ordinary_user".to_string()),
                },
            )
            .await?;
            let role_id = roles
                .d
                .iter()
                .find(|role| role.code == "ordinary_user")
                .map(|role| role.id.clone())
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "missing ordinary_user role")
                })?;

            let created_user = rbac::create_user(
                &pool,
                admin_token,
                UserInput {
                    username: "ordinary01".to_string(),
                    password: "user123456".to_string(),
                    real_name: "普通用户".to_string(),
                    avatar: None,
                    home_path: None,
                    status: None,
                    role_ids: vec![role_id],
                },
            )
            .await?;
            assert_eq!(created_user.home_path, ORDINARY_USER_HOME_PATH);

            let token = auth::login(
                &pool,
                LoginRequest {
                    username: "ordinary01".to_string(),
                    password: "user123456".to_string(),
                },
            )
            .await?
            .access_token;
            let current_user = auth::current_user(&pool, token.clone()).await?;
            assert_eq!(current_user.home_path, ORDINARY_USER_HOME_PATH);

            let menus = rbac::menus(&pool, token).await?;
            assert!(menus.iter().any(|menu| menu.path == "/assets"));
            assert!(!menus.iter().any(|menu| menu.path == "/system"));
            Ok(())
        })
    }

    #[test]
    fn dictionary_enforces_unique_type_code_and_item_value() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let dict_type = dictionary::create_type(
                &pool,
                token.clone(),
                DictTypeInput {
                    code: "priority".to_string(),
                    name: "优先级".to_string(),
                    description: None,
                    status: None,
                    sort_order: None,
                },
            )
            .await?;

            let duplicate_type = dictionary::create_type(
                &pool,
                token.clone(),
                DictTypeInput {
                    code: "priority".to_string(),
                    name: "重复优先级".to_string(),
                    description: None,
                    status: None,
                    sort_order: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(duplicate_type, AppError::Conflict(_)));

            dictionary::create_item(
                &pool,
                token.clone(),
                DictItemInput {
                    type_id: dict_type.id.clone(),
                    label: "高".to_string(),
                    value: "high".to_string(),
                    status: None,
                    sort_order: None,
                },
            )
            .await?;

            let duplicate_item = dictionary::create_item(
                &pool,
                token,
                DictItemInput {
                    type_id: dict_type.id,
                    label: "高优先级".to_string(),
                    value: "high".to_string(),
                    status: None,
                    sort_order: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(duplicate_item, AppError::Conflict(_)));
            Ok(())
        })
    }

    #[test]
    fn skills_support_crud_filtering_uniqueness_and_toggle() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let skill = skills::create(
                &pool,
                token.clone(),
                SkillInput {
                    code: "rust-best-practices".to_string(),
                    name: "Rust 最佳实践".to_string(),
                    category: Some("rust".to_string()),
                    description: Some("Rust 后端代码规范".to_string()),
                    prompt: Some("遵循错误链、模块边界和测试约束".to_string()),
                    tags: Some(vec![
                        "rust".to_string(),
                        "后端".to_string(),
                        "rust".to_string(),
                    ]),
                    status: None,
                    sort_order: Some(10),
                },
            )
            .await?;
            assert_eq!(skill.tags, vec!["rust".to_string(), "后端".to_string()]);

            let duplicate = skills::create(
                &pool,
                token.clone(),
                SkillInput {
                    code: "rust-best-practices".to_string(),
                    name: "重复技能".to_string(),
                    category: None,
                    description: None,
                    prompt: None,
                    tags: None,
                    status: None,
                    sort_order: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(duplicate, AppError::Conflict(_)));

            let page = skills::page(
                &pool,
                token.clone(),
                SkillPageRequest {
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("后端".to_string()),
                    category: Some("rust".to_string()),
                    categories: None,
                    status: Some("enabled".to_string()),
                },
            )
            .await?;
            assert_eq!(page.t, 1);

            let updated = skills::update(
                &pool,
                token.clone(),
                SkillUpdateInput {
                    id: skill.id.clone(),
                    code: "rust-best-practices".to_string(),
                    name: "Rust 工程规范".to_string(),
                    category: Some("rust".to_string()),
                    description: Some("Rust 后端代码规范".to_string()),
                    prompt: Some("补充 SQLx 和 Tauri 命令边界".to_string()),
                    tags: Some(vec!["rust".to_string(), "tauri".to_string()]),
                    status: Some("enabled".to_string()),
                    sort_order: Some(20),
                },
            )
            .await?;
            assert_eq!(updated.name, "Rust 工程规范");
            assert_eq!(updated.sort_order, 20);

            let disabled = skills::toggle(
                &pool,
                token.clone(),
                SkillToggleInput {
                    id: skill.id.clone(),
                    status: "disabled".to_string(),
                },
            )
            .await?;
            assert_eq!(disabled.status, "disabled");

            let disabled_page = skills::page(
                &pool,
                token.clone(),
                SkillPageRequest {
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("tauri".to_string()),
                    category: Some("rust".to_string()),
                    categories: None,
                    status: Some("disabled".to_string()),
                },
            )
            .await?;
            assert_eq!(disabled_page.t, 1);

            skills::delete(&pool, token, skill.id).await?;
            Ok(())
        })
    }

    #[test]
    fn asset_items_support_crud_filtering_uniqueness_and_toggle() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let item = crate::asset_items::create(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemInput {
                    kind: "docker_compose".to_string(),
                    code: "blog-stack".to_string(),
                    name: "博客栈".to_string(),
                    category: Some("infra".to_string()),
                    description: Some("本机博客依赖栈".to_string()),
                    content: Some("services: {}".to_string()),
                    tags: Some(vec!["docker".to_string(), "compose".to_string()]),
                    source_path: None,
                    file_name: None,
                    source_mtime: None,
                    source_size: None,
                    content_hash: None,
                    last_synced_at: None,
                    service_count: None,
                    services: None,
                    images: None,
                    ports: None,
                    volumes: None,
                    validation_status: None,
                    validation_issues: None,
                    variable_candidates: None,
                    status: None,
                    sort_order: Some(10),
                },
            )
            .await?;
            assert_eq!(item.kind, "docker_compose");

            let duplicate = crate::asset_items::create(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemInput {
                    kind: "docker_compose".to_string(),
                    code: "blog-stack".to_string(),
                    name: "重复栈".to_string(),
                    category: None,
                    description: None,
                    content: None,
                    tags: None,
                    source_path: None,
                    file_name: None,
                    source_mtime: None,
                    source_size: None,
                    content_hash: None,
                    last_synced_at: None,
                    service_count: None,
                    services: None,
                    images: None,
                    ports: None,
                    volumes: None,
                    validation_status: None,
                    validation_issues: None,
                    variable_candidates: None,
                    status: None,
                    sort_order: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(duplicate, AppError::Conflict(_)));

            let page = crate::asset_items::page(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemPageRequest {
                    kind: "docker_compose".to_string(),
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("博客".to_string()),
                    category: Some("infra".to_string()),
                    categories: None,
                    status: Some("enabled".to_string()),
                },
            )
            .await?;
            assert_eq!(page.t, 1);

            let updated = crate::asset_items::update(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemUpdateInput {
                    id: item.id.clone(),
                    kind: "docker_compose".to_string(),
                    code: "blog-stack".to_string(),
                    name: "博客基础栈".to_string(),
                    category: Some("infra".to_string()),
                    description: Some("更新后的说明".to_string()),
                    content: Some("services: { app: {} }".to_string()),
                    tags: Some(vec!["docker".to_string(), "dev".to_string()]),
                    source_path: None,
                    file_name: None,
                    source_mtime: None,
                    source_size: None,
                    content_hash: None,
                    last_synced_at: None,
                    service_count: None,
                    services: None,
                    images: None,
                    ports: None,
                    volumes: None,
                    validation_status: None,
                    validation_issues: None,
                    variable_candidates: None,
                    status: Some("enabled".to_string()),
                    sort_order: Some(20),
                },
            )
            .await?;
            assert_eq!(updated.name, "博客基础栈");

            let disabled = crate::asset_items::toggle(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemToggleInput {
                    id: item.id.clone(),
                    status: "disabled".to_string(),
                },
            )
            .await?;
            assert_eq!(disabled.status, "disabled");

            crate::asset_items::delete(&pool, token, item.id).await?;
            Ok(())
        })
    }

    #[test]
    fn agent_preferences_support_tabbed_crud_filtering_uniqueness_and_toggle(
    ) -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let preference = agent_preferences::create(
                &pool,
                token.clone(),
                AgentPreferenceInput {
                    code: "rust-error-boundary".to_string(),
                    section: "domain".to_string(),
                    domain: Some("rust".to_string()),
                    title: "Rust 错误边界".to_string(),
                    content: Some("保留 source，只在 Tauri 命令边界映射。".to_string()),
                    rationale: Some("便于排查真实失败原因".to_string()),
                    tags: Some(vec![
                        "rust".to_string(),
                        "error".to_string(),
                        "rust".to_string(),
                    ]),
                    status: None,
                    sort_order: Some(10),
                },
            )
            .await?;
            assert_eq!(
                preference.tags,
                vec!["rust".to_string(), "error".to_string()]
            );

            let duplicate = agent_preferences::create(
                &pool,
                token.clone(),
                AgentPreferenceInput {
                    code: "rust-error-boundary".to_string(),
                    section: "domain".to_string(),
                    domain: Some("java".to_string()),
                    title: "重复偏好".to_string(),
                    content: None,
                    rationale: None,
                    tags: None,
                    status: None,
                    sort_order: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(duplicate, AppError::Conflict(_)));

            let page = agent_preferences::page(
                &pool,
                token.clone(),
                AgentPreferencePageRequest {
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("source".to_string()),
                    section: Some("domain".to_string()),
                    domain: Some("rust".to_string()),
                    status: Some("enabled".to_string()),
                },
            )
            .await?;
            assert_eq!(page.t, 1);

            let updated = agent_preferences::update(
                &pool,
                token.clone(),
                AgentPreferenceUpdateInput {
                    id: preference.id.clone(),
                    code: "rust-error-boundary".to_string(),
                    section: "domain".to_string(),
                    domain: Some("rust".to_string()),
                    title: "Rust 错误处理边界".to_string(),
                    content: Some("内部用 AppResult，边界统一映射。".to_string()),
                    rationale: Some("减少重复错误转换".to_string()),
                    tags: Some(vec!["rust".to_string(), "tauri".to_string()]),
                    status: Some("enabled".to_string()),
                    sort_order: Some(20),
                },
            )
            .await?;
            assert_eq!(updated.title, "Rust 错误处理边界");
            assert_eq!(updated.sort_order, 20);

            let disabled = agent_preferences::toggle(
                &pool,
                token.clone(),
                AgentPreferenceToggleInput {
                    id: preference.id.clone(),
                    status: "disabled".to_string(),
                },
            )
            .await?;
            assert_eq!(disabled.status, "disabled");

            agent_preferences::delete(&pool, token, preference.id).await?;
            Ok(())
        })
    }

    #[test]
    fn docker_compose_import_keeps_all_yaml_and_extracts_compose_fields(
    ) -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let dir = tempfile::tempdir()?;
            let stack_dir = dir.path().join("infra");
            fs::create_dir_all(&stack_dir)?;
            fs::write(
                stack_dir.join("compose.yml"),
                r#"
services:
  web:
    image: nginx:latest
    ports:
      - "8080:80"
    volumes:
      - ./html:/usr/share/nginx/html
  redis:
    image: redis:7
"#,
            )?;
            fs::write(
                stack_dir.join("config.yaml"),
                r#"
debug: true
port: 8080
"#,
            )?;

            let imported = crate::asset_items::import_directory(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemImportRequest {
                    kind: "docker_compose".to_string(),
                    root_path: dir.path().to_string_lossy().to_string(),
                },
            )
            .await?;
            assert_eq!(imported.scanned, 2);
            assert_eq!(imported.imported, 2);
            assert_eq!(imported.skipped, 0);

            let page = crate::asset_items::page(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemPageRequest {
                    kind: "docker_compose".to_string(),
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("nginx".to_string()),
                    category: Some("infra".to_string()),
                    categories: None,
                    status: Some("enabled".to_string()),
                },
            )
            .await?;
            assert_eq!(page.t, 1);
            assert_eq!(page.d[0].service_count, 2);
            assert!(page.d[0].services.contains(&"web".to_string()));
            assert!(page.d[0].images.contains(&"nginx:latest".to_string()));
            assert!(page.d[0].ports.contains(&"8080:80".to_string()));

            let config_page = crate::asset_items::page(
                &pool,
                token.clone(),
                crate::asset_items::AssetItemPageRequest {
                    kind: "docker_compose".to_string(),
                    o: Some(0),
                    s: Some(10),
                    keyword: Some("config.yaml".to_string()),
                    category: Some("infra".to_string()),
                    categories: None,
                    status: Some("enabled".to_string()),
                },
            )
            .await?;
            assert_eq!(config_page.t, 1);
            assert_eq!(config_page.d[0].service_count, 0);

            fs::write(
                stack_dir.join("compose.yml"),
                r#"
services:
  web:
    image: nginx:1.27
"#,
            )?;
            let updated = crate::asset_items::import_directory(
                &pool,
                token,
                crate::asset_items::AssetItemImportRequest {
                    kind: "docker_compose".to_string(),
                    root_path: dir.path().to_string_lossy().to_string(),
                },
            )
            .await?;
            assert_eq!(updated.updated, 1);
            Ok(())
        })
    }

    #[test]
    fn notes_are_filtered_by_owner() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let admin_token = login_admin(&pool).await?;
            let admin_note = notes::create(
                &pool,
                admin_token.clone(),
                NoteInput {
                    title: "管理员笔记".to_string(),
                    content: Some("admin only".to_string()),
                    category: Some("work".to_string()),
                    tags: Some(vec!["private".to_string()]),
                },
            )
            .await?;

            let roles = rbac::role_page(
                &pool,
                admin_token.clone(),
                PageRequest {
                    o: Some(0),
                    s: Some(1),
                    keyword: Some("super_admin".to_string()),
                },
            )
            .await?;
            let role_id = roles.d[0].id.clone();
            rbac::create_user(
                &pool,
                admin_token,
                UserInput {
                    username: "note_user".to_string(),
                    password: "user123456".to_string(),
                    real_name: "笔记用户".to_string(),
                    avatar: None,
                    home_path: None,
                    status: None,
                    role_ids: vec![role_id],
                },
            )
            .await?;

            let user_token = auth::login(
                &pool,
                LoginRequest {
                    username: "note_user".to_string(),
                    password: "user123456".to_string(),
                },
            )
            .await?
            .access_token;
            let user_note = notes::create(
                &pool,
                user_token.clone(),
                NoteInput {
                    title: "用户笔记".to_string(),
                    content: Some("user only".to_string()),
                    category: Some("work".to_string()),
                    tags: None,
                },
            )
            .await?;

            let admin_page = notes::page(
                &pool,
                auth::login(
                    &pool,
                    LoginRequest {
                        username: "admin".to_string(),
                        password: "admin123456".to_string(),
                    },
                )
                .await?
                .access_token,
                NotePageRequest {
                    o: Some(0),
                    s: Some(20),
                    keyword: None,
                    category: None,
                    archived: Some(false),
                },
            )
            .await?;
            assert!(admin_page.d.iter().any(|note| note.id == admin_note.id));
            assert!(!admin_page.d.iter().any(|note| note.id == user_note.id));

            let user_page = notes::page(
                &pool,
                user_token,
                NotePageRequest {
                    o: Some(0),
                    s: Some(20),
                    keyword: None,
                    category: None,
                    archived: Some(false),
                },
            )
            .await?;
            assert!(user_page.d.iter().any(|note| note.id == user_note.id));
            assert!(!user_page.d.iter().any(|note| note.id == admin_note.id));
            Ok(())
        })
    }

    #[test]
    fn dictionary_item_page_can_filter_by_type() -> Result<(), Box<dyn Error>> {
        with_seeded_pool(|pool| async move {
            let token = login_admin(&pool).await?;
            let page = dictionary::item_page(
                &pool,
                token,
                DictItemPageRequest {
                    type_id: None,
                    o: Some(0),
                    s: Some(20),
                    keyword: Some("enabled".to_string()),
                },
            )
            .await?;

            assert!(page.t >= 1);
            assert!(page.d.iter().any(|item| item.value == "enabled"));
            Ok(())
        })
    }
}
