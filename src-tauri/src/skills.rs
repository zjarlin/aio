use std::{
    collections::HashSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_yml::Value as YamlValue;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};
use walkdir::WalkDir;

use crate::{
    asset_items::normalize_filter_categories,
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

const DEFAULT_REMOTE_SKILL_HOST: &str = "zjarlin@raw.addzero.site";
const DEFAULT_REMOTE_HOST_LABEL: &str = "raw.addzero.site";

struct TagRule {
    tag: &'static str,
    patterns: &'static [&'static str],
}

const FUNCTIONAL_TAG_RULES: &[TagRule] = &[
    TagRule {
        tag: "rust",
        patterns: &["rust", "cargo", "crate", "sqlx", "axum", "thiserror"],
    },
    TagRule {
        tag: "java",
        patterns: &["java", "jvm", "spring"],
    },
    TagRule {
        tag: "kotlin",
        patterns: &["kotlin", "kmp", "ktor", "koin", "ksp"],
    },
    TagRule {
        tag: "maven",
        patterns: &["maven"],
    },
    TagRule {
        tag: "gradle",
        patterns: &["gradle", "build-logic", "version catalog"],
    },
    TagRule {
        tag: "compose",
        patterns: &[
            "compose multiplatform",
            "jetpack compose",
            "compose ui",
            "composable",
            "shadcn compose",
        ],
    },
    TagRule {
        tag: "tauri",
        patterns: &["tauri"],
    },
    TagRule {
        tag: "vue",
        patterns: &["vue", "vue-router", "pinia", "element plus", "element-plus"],
    },
    TagRule {
        tag: "react",
        patterns: &["react", "next.js", "nextjs"],
    },
    TagRule {
        tag: "shadcn",
        patterns: &["shadcn", "shadcn-ui", "shadcn/vue", "shadcn-vue"],
    },
    TagRule {
        tag: "docker",
        patterns: &["docker", "docker compose", "compose.yml", "container"],
    },
    TagRule {
        tag: "数据库",
        patterns: &[
            "database",
            "postgres",
            "postgresql",
            "sqlite",
            "mysql",
            "redis",
            "sql",
            "数据源",
            "数据库",
        ],
    },
    TagRule {
        tag: "后端",
        patterns: &[
            "backend",
            "server",
            "api",
            "route",
            "endpoint",
            "auth",
            "rbac",
            "权限",
            "鉴权",
            "后端",
            "服务端",
        ],
    },
    TagRule {
        tag: "前端",
        patterns: &[
            "frontend",
            "ui",
            "页面",
            "组件",
            "表单",
            "布局",
            "toolbar",
            "sidebar",
            "dialog",
            "drawer",
            "tabs",
        ],
    },
    TagRule {
        tag: "编程规范",
        patterns: &[
            "best practice",
            "best-practice",
            "convention",
            "coding convention",
            "code style",
            "lint",
            "policy",
            "rules",
            "规范",
            "约定",
            "最佳实践",
        ],
    },
    TagRule {
        tag: "编程",
        patterns: &[
            "code",
            "coding",
            "programming",
            "refactor",
            "implementation",
            "module",
            "source",
            "开发",
            "编程",
            "代码",
            "源码",
            "重构",
        ],
    },
    TagRule {
        tag: "项目管理",
        patterns: &[
            "gsd",
            "roadmap",
            "milestone",
            "backlog",
            "phase",
            "requirements",
            "uat",
            "prd",
            "planning",
            "项目管理",
            "里程碑",
            "需求",
        ],
    },
    TagRule {
        tag: "工作流",
        patterns: &[
            "workflow",
            "automation",
            "pipeline",
            "ci/cd",
            "release",
            "publish",
            "artifact",
            "工作流",
            "自动化",
            "发布",
        ],
    },
    TagRule {
        tag: "测试",
        patterns: &["test", "testing", "tdd", "verify", "validation", "测试", "验证"],
    },
    TagRule {
        tag: "安全",
        patterns: &["security", "secure", "permission", "token", "安全"],
    },
    TagRule {
        tag: "命令行",
        patterns: &["cli", "shell", "bash", "zsh", "terminal", "command line", "命令行"],
    },
    TagRule {
        tag: "文档",
        patterns: &[
            "docs",
            "document",
            "markdown",
            "ppt",
            "presentation",
            "spreadsheet",
            "docx",
            "文档",
        ],
    },
    TagRule {
        tag: "AI",
        patterns: &["openai", "assistant", "llm", "prompt", "agent", "mcp", "ai"],
    },
    TagRule {
        tag: "硬件",
        patterns: &[
            "esp32", "esp-idf", "embedded", "uart", "serial", "modbus", "openocd", "jtag", "硬件",
            "串口",
        ],
    },
    TagRule {
        tag: "依赖注入",
        patterns: &["dependency injection", "di", "ioc", "koin", "依赖注入"],
    },
    TagRule {
        tag: "架构",
        patterns: &["architecture", "package", "workspace", "multi-module", "模块", "架构"],
    },
    TagRule {
        tag: "自媒体",
        patterns: &[
            "自媒体",
            "内容创作",
            "公众号",
            "小红书",
            "抖音",
            "视频号",
            "短视频",
            "bilibili",
            "seo",
            "copywriting",
            "博客",
            "写作",
        ],
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub prompt: String,
    pub content_hash: String,
    pub tags: Vec<String>,
    pub sources: Vec<SkillSourceRef>,
    pub status: String,
    pub sort_order: i64,
    pub last_synced_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Eq, FromRow, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceRef {
    pub kind: String,
    pub host: String,
    pub root: String,
    pub path: String,
}

#[derive(Debug, FromRow)]
struct SkillRow {
    id: String,
    code: String,
    name: String,
    category: String,
    description: String,
    prompt: String,
    content_hash: String,
    tags_json: String,
    status: String,
    sort_order: i64,
    last_synced_at: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, FromRow)]
struct ImportExistingSkill {
    id: String,
    content_hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SkillImportDocument {
    pub source: SkillSourceRef,
    pub content: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSyncResult {
    pub scanned: i64,
    pub imported: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub deduplicated: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

impl SkillSyncResult {
    fn merge(&mut self, other: SkillSyncResult) {
        self.scanned += other.scanned;
        self.imported += other.imported;
        self.updated += other.updated;
        self.unchanged += other.unchanged;
        self.deduplicated += other.deduplicated;
        self.skipped += other.skipped;
        self.errors.extend(other.errors);
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillFrontmatter {
    name: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    keywords: Option<YamlValue>,
    tags: Option<YamlValue>,
}

#[derive(Debug, Default)]
struct ParsedSkill {
    category: String,
    code: String,
    description: String,
    name: String,
    prompt: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteSkillLine {
    root: Option<String>,
    path: Option<String>,
    content: Option<String>,
    error: Option<String>,
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
             OR content_hash LIKE ?
             OR EXISTS (
               SELECT 1
               FROM skill_sources
               WHERE skill_sources.skill_id = skills.id
                 AND (
                   skill_sources.kind LIKE ?
                   OR skill_sources.host LIKE ?
                   OR skill_sources.root LIKE ?
                   OR skill_sources.path LIKE ?
                 )
             )
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
             OR content_hash LIKE ?
             OR EXISTS (
               SELECT 1
               FROM skill_sources
               WHERE skill_sources.skill_id = skills.id
                 AND (
                   skill_sources.kind LIKE ?
                   OR skill_sources.host LIKE ?
                   OR skill_sources.root LIKE ?
                   OR skill_sources.path LIKE ?
                 )
             )
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
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .bind(size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        records.push(to_record(pool, row).await?);
    }

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
    let prompt = input.prompt.unwrap_or_default();
    let content_hash = skill_content_hash(&prompt);
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO skills
         (id, code, name, category, description, prompt, content_hash, tags_json,
          status, sort_order, last_synced_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(input.code)
    .bind(input.name)
    .bind(input.category.unwrap_or_default())
    .bind(input.description.unwrap_or_default())
    .bind(prompt)
    .bind(content_hash)
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
    let prompt = input.prompt.unwrap_or_default();
    let content_hash = skill_content_hash(&prompt);
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE skills
         SET code = ?, name = ?, category = ?, description = ?, prompt = ?,
             content_hash = ?, tags_json = ?, status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(input.code)
    .bind(input.name)
    .bind(input.category.unwrap_or_default())
    .bind(input.description.unwrap_or_default())
    .bind(prompt)
    .bind(content_hash)
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

pub async fn sync_sources(pool: &SqlitePool, token: String) -> AppResult<SkillSyncResult> {
    require_session(pool, &token).await?;
    let (documents, mut result) = tokio::task::spawn_blocking(scan_default_skill_sources)
        .await
        .map_err(|error| AppError::BadRequest(format!("技能扫描任务失败：{error}")))??;
    let import_result = import_documents(pool, documents).await?;
    result.merge(import_result);
    Ok(result)
}

pub(crate) async fn import_documents(
    pool: &SqlitePool,
    documents: Vec<SkillImportDocument>,
) -> AppResult<SkillSyncResult> {
    let mut result = SkillSyncResult::default();
    let now = now_millis();
    let mut tx = pool.begin().await?;

    for document in documents {
        result.scanned += 1;
        let content = normalize_skill_content(&document.content);
        if content.trim().is_empty() {
            result.skipped += 1;
            continue;
        }

        let content_hash = skill_content_hash(&content);
        let parsed = parse_skill_document(&content, &document.source);
        let functional_tags = functional_skill_tags(&parsed, &document.source);
        let tags_json = encode_tags(functional_tags.clone())?;
        let existing_source = find_by_source_tx(&mut tx, &document.source).await?;

        if let Some(existing) = existing_source {
            if existing.content_hash != content_hash {
                if let Some(same_hash) = find_by_content_hash_tx(&mut tx, &content_hash).await? {
                    if same_hash.id != existing.id {
                        upsert_source_tx(&mut tx, &same_hash.id, &document.source, now).await?;
                        merge_skill_tags_tx(&mut tx, &same_hash.id, functional_tags, now).await?;
                        delete_import_orphan_tx(&mut tx, &existing.id).await?;
                        result.deduplicated += 1;
                        continue;
                    }
                }
            }

            let code =
                unique_code_tx(&mut tx, &parsed.code, &content_hash, Some(&existing.id)).await?;
            update_imported_skill_tx(
                &mut tx,
                &existing.id,
                &code,
                &parsed,
                &content_hash,
                &tags_json,
                now,
            )
            .await?;
            upsert_source_tx(&mut tx, &existing.id, &document.source, now).await?;
            if existing.content_hash == content_hash {
                result.unchanged += 1;
            } else {
                result.updated += 1;
            }
            continue;
        }

        if let Some(existing) = find_by_content_hash_tx(&mut tx, &content_hash).await? {
            upsert_source_tx(&mut tx, &existing.id, &document.source, now).await?;
            merge_skill_tags_tx(&mut tx, &existing.id, functional_tags, now).await?;
            result.deduplicated += 1;
            continue;
        }

        let id = new_id();
        let code = unique_code_tx(&mut tx, &parsed.code, &content_hash, None).await?;
        sqlx::query(
            "INSERT INTO skills
             (id, code, name, category, description, prompt, content_hash, tags_json,
              status, sort_order, last_synced_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'enabled', 0, ?, ?, ?)",
        )
        .bind(&id)
        .bind(code)
        .bind(parsed.name)
        .bind(primary_skill_category(&functional_tags))
        .bind(parsed.description)
        .bind(parsed.prompt)
        .bind(content_hash)
        .bind(tags_json)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_unique_error)?;
        upsert_source_tx(&mut tx, &id, &document.source, now).await?;
        result.imported += 1;
    }

    tx.commit().await?;
    Ok(result)
}

async fn find(pool: &SqlitePool, id: &str) -> AppResult<SkillRecord> {
    let row = sqlx::query_as::<_, SkillRow>("SELECT * FROM skills WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    to_record(pool, row).await
}

async fn to_record(pool: &SqlitePool, row: SkillRow) -> AppResult<SkillRecord> {
    let tags = serde_json::from_str::<Vec<String>>(&row.tags_json)
        .map_err(|source| AppError::Json { source })?;
    let sources = load_sources(pool, &row.id).await?;

    Ok(SkillRecord {
        id: row.id,
        code: row.code,
        name: row.name,
        category: row.category,
        description: row.description,
        prompt: row.prompt,
        content_hash: row.content_hash,
        tags,
        sources,
        status: row.status,
        sort_order: row.sort_order,
        last_synced_at: row.last_synced_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn encode_tags(tags: Vec<String>) -> AppResult<String> {
    let mut seen = HashSet::new();
    let normalized = tags
        .into_iter()
        .map(canonicalize_tag)
        .filter(|tag| !tag.is_empty())
        .filter(|tag| !is_source_tag(tag))
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

pub async fn ensure_content_hashes(pool: &SqlitePool) -> AppResult<()> {
    let rows = sqlx::query_as::<_, SkillRow>(
        "SELECT *
         FROM skills
         WHERE content_hash = '' OR content_hash IS NULL",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        sqlx::query("UPDATE skills SET content_hash = ? WHERE id = ?")
            .bind(skill_content_hash(&row.prompt))
            .bind(row.id)
            .execute(pool)
            .await?;
    }

    sqlx::query(
        "DELETE FROM skills
         WHERE content_hash <> ''
           AND id NOT IN (
             SELECT id
             FROM (
               SELECT id, content_hash
               FROM skills
               WHERE content_hash <> ''
               ORDER BY updated_at DESC, created_at DESC, id DESC
             )
             GROUP BY content_hash
           )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skills_content_hash
         ON skills(content_hash)",
    )
    .execute(pool)
    .await?;

    refresh_imported_skill_tags(pool).await?;

    Ok(())
}

pub async fn refresh_imported_skill_tags(pool: &SqlitePool) -> AppResult<()> {
    let rows = sqlx::query_as::<_, SkillRow>(
        "SELECT *
         FROM skills
         WHERE last_synced_at <> 0",
    )
    .fetch_all(pool)
    .await?;
    let now = now_millis();

    for row in rows {
        let sources = load_sources(pool, &row.id).await?;
        let fallback_source = SkillSourceRef {
            kind: String::new(),
            host: String::new(),
            root: row.category.clone(),
            path: row.code.clone(),
        };
        let source = sources.first().unwrap_or(&fallback_source);
        let parsed = parse_skill_document(&row.prompt, source);
        let tags = functional_skill_tags(&parsed, source);
        let category = primary_skill_category(&tags);
        let tags_json = encode_tags(tags)?;
        if row.category == category && row.tags_json == tags_json {
            continue;
        }

        sqlx::query(
            "UPDATE skills
             SET category = ?, tags_json = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(category)
        .bind(tags_json)
        .bind(now)
        .bind(row.id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn load_sources<'a>(
    pool: &'a SqlitePool,
    skill_id: &'a str,
) -> impl std::future::Future<Output = AppResult<Vec<SkillSourceRef>>> + 'a {
    async move {
        let sources = sqlx::query_as::<_, SkillSourceRef>(
            "SELECT kind, host, root, path
             FROM skill_sources
             WHERE skill_id = ?
             ORDER BY updated_at DESC, created_at DESC",
        )
        .bind(skill_id)
        .fetch_all(pool)
        .await?;
        Ok(sources)
    }
}

fn skill_content_hash(content: &str) -> String {
    let normalized = normalize_skill_content(content);
    if normalized.trim().is_empty() {
        return String::new();
    }

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn skill_short_hash(value: &str) -> String {
    value.chars().take(10).collect()
}

fn normalize_skill_content(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn scan_default_skill_sources() -> AppResult<(Vec<SkillImportDocument>, SkillSyncResult)> {
    let mut result = SkillSyncResult::default();
    let mut documents = Vec::new();

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for (root, path) in [
            ("codex", home.join(".codex/skills")),
            ("agents", home.join(".agents/skills")),
        ] {
            scan_local_skill_root(&mut documents, &mut result, root, &path);
        }
    } else {
        result
            .errors
            .push("无法读取本机 HOME 环境变量，跳过本地 skill 扫描".to_string());
    }

    scan_remote_skill_sources(&mut documents, &mut result);
    Ok((documents, result))
}

fn scan_local_skill_root(
    documents: &mut Vec<SkillImportDocument>,
    result: &mut SkillSyncResult,
    root: &str,
    path: &Path,
) {
    if !path.exists() {
        return;
    }

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file() && entry.file_name().to_string_lossy() == "SKILL.md"
        })
    {
        let file_path = entry.path().to_path_buf();
        match fs::read_to_string(&file_path) {
            Ok(content) => documents.push(SkillImportDocument {
                source: SkillSourceRef {
                    kind: "local".to_string(),
                    host: "local".to_string(),
                    root: root.to_string(),
                    path: file_path.to_string_lossy().to_string(),
                },
                content,
            }),
            Err(error) => {
                result.scanned += 1;
                result.skipped += 1;
                result.errors.push(format!(
                    "读取本机 skill 失败：{} - {}",
                    file_path.to_string_lossy(),
                    error
                ));
            }
        }
    }
}

fn scan_remote_skill_sources(
    documents: &mut Vec<SkillImportDocument>,
    result: &mut SkillSyncResult,
) {
    let script = r#"
import json
import os

roots = [
    ("codex", os.path.expanduser("~/.codex/skills")),
    ("agents", os.path.expanduser("~/.agents/skills")),
]

for root_name, root in roots:
    if not os.path.isdir(root):
        continue
    for dirpath, dirnames, filenames in os.walk(root, followlinks=True):
        if "SKILL.md" not in filenames:
            continue
        path = os.path.join(dirpath, "SKILL.md")
        try:
            with open(path, "r", encoding="utf-8") as handle:
                content = handle.read()
            print(json.dumps({
                "root": root_name,
                "path": path,
                "content": content,
            }, ensure_ascii=False))
        except Exception as error:
            print(json.dumps({
                "root": root_name,
                "path": path,
                "error": str(error),
            }, ensure_ascii=False))
"#;

    let output = match Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=8")
        .arg(DEFAULT_REMOTE_SKILL_HOST)
        .arg("python3")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                if let Err(error) = stdin.write_all(script.as_bytes()) {
                    result
                        .errors
                        .push(format!("写入远程 skill 扫描脚本失败：{error}"));
                    return;
                }
            }
            match child.wait_with_output() {
                Ok(output) => output,
                Err(error) => {
                    result
                        .errors
                        .push(format!("执行远程 skill 扫描失败：{error}"));
                    return;
                }
            }
        }
        Err(error) => {
            result
                .errors
                .push(format!("启动远程 skill 扫描失败：{error}"));
            return;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            result.errors.push(format!("远程 skill 扫描失败：{stderr}"));
        } else {
            result.errors.push("远程 skill 扫描失败".to_string());
        }
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let parsed = match serde_json::from_str::<RemoteSkillLine>(line) {
            Ok(parsed) => parsed,
            Err(error) => {
                result.scanned += 1;
                result.skipped += 1;
                result
                    .errors
                    .push(format!("解析远程 skill 响应失败：{error}"));
                continue;
            }
        };

        if let Some(error) = parsed.error {
            result.scanned += 1;
            result.skipped += 1;
            result.errors.push(format!(
                "读取远程 skill 失败：{} - {}",
                parsed.path.unwrap_or_default(),
                error
            ));
            continue;
        }

        let (Some(root), Some(path), Some(content)) = (parsed.root, parsed.path, parsed.content)
        else {
            result.scanned += 1;
            result.skipped += 1;
            result
                .errors
                .push("远程 skill 响应缺少 path 或 content".to_string());
            continue;
        };

        documents.push(SkillImportDocument {
            source: SkillSourceRef {
                kind: "ssh".to_string(),
                host: DEFAULT_REMOTE_HOST_LABEL.to_string(),
                root,
                path,
            },
            content,
        });
    }
}

fn parse_skill_document(content: &str, source: &SkillSourceRef) -> ParsedSkill {
    let normalized = normalize_skill_content(content);
    let (frontmatter, body) = split_skill_frontmatter(&normalized);
    let fallback_name = Path::new(&source.path)
        .parent()
        .and_then(|path| path.file_name())
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "未命名技能".to_string());
    let fallback_code = slugify(&fallback_name);
    let mut parsed = ParsedSkill {
        category: String::new(),
        code: fallback_code,
        description: infer_description(&body),
        name: fallback_name,
        prompt: normalized,
        tags: Vec::new(),
    };

    if let Some(frontmatter) = frontmatter
        .and_then(|frontmatter| serde_yml::from_str::<SkillFrontmatter>(&frontmatter).ok())
    {
        if let Some(name) = frontmatter.name.and_then(|value| normalize_text(value)) {
            parsed.code = slugify(&name);
            if parsed.code.is_empty() {
                parsed.code = "skill".to_string();
            }
            parsed.name = name;
        }

        if let Some(display_name) = frontmatter.display_name.and_then(normalize_text) {
            parsed.name = display_name;
        }

        if let Some(description) = frontmatter.description.and_then(normalize_text) {
            parsed.description = description;
        }

        parsed
            .tags
            .extend(yaml_value_to_strings(frontmatter.keywords));
        parsed.tags.extend(yaml_value_to_strings(frontmatter.tags));
    }

    parsed.tags = normalize_texts(parsed.tags)
        .into_iter()
        .filter(|tag| !is_source_tag(tag))
        .collect();
    let functional_tags = functional_skill_tags(&parsed, source);
    parsed.category = primary_skill_category(&functional_tags);
    if parsed.code.trim().is_empty() {
        parsed.code = "skill".to_string();
    }
    if parsed.name.trim().is_empty() {
        parsed.name = "未命名技能".to_string();
    }
    if parsed.description.trim().is_empty() {
        parsed.description = infer_description(&body);
    }
    parsed
}

fn split_skill_frontmatter(content: &str) -> (Option<String>, String) {
    let normalized = content.trim_start_matches('\u{feff}').to_string();
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return (None, normalized);
    }

    let mut frontmatter = Vec::new();
    let mut body = Vec::new();
    let mut closed = false;

    for line in lines {
        if !closed && line == "---" {
            closed = true;
            continue;
        }

        if closed {
            body.push(line);
        } else {
            frontmatter.push(line);
        }
    }

    if !closed {
        return (None, normalized);
    }

    (Some(frontmatter.join("\n")), body.join("\n"))
}

fn infer_description(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .unwrap_or_default()
}

fn yaml_value_to_strings(value: Option<YamlValue>) -> Vec<String> {
    match value {
        Some(YamlValue::Sequence(items)) => items
            .iter()
            .filter_map(yaml_value_to_string)
            .collect::<Vec<_>>(),
        Some(value) => yaml_value_to_string(&value).into_iter().collect(),
        None => Vec::new(),
    }
}

fn yaml_value_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::Null => None,
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        YamlValue::String(value) => {
            Some(value.trim().to_string()).filter(|value| !value.is_empty())
        }
        YamlValue::Sequence(values) => {
            let values = values
                .iter()
                .filter_map(yaml_value_to_string)
                .collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(values.join(", "))
            }
        }
        YamlValue::Mapping(_) => None,
        YamlValue::Tagged(tagged) => yaml_value_to_string(&tagged.value),
    }
}

fn normalize_text(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn normalize_texts(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter_map(normalize_text)
        .map(canonicalize_tag)
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn canonicalize_tag(value: String) -> String {
    let tag = value.trim().trim_start_matches('#').to_string();
    let lower = tag.to_lowercase();
    match lower.as_str() {
        "js" | "javascript" | "typescript" | "ts" => "typescript".to_string(),
        "postgresql" => "postgres".to_string(),
        "sqlite3" => "sqlite".to_string(),
        "docker-compose" => "docker".to_string(),
        "kmp" | "compose-multiplatform" | "jetpack-compose" => "compose".to_string(),
        "ui" | "ux" | "frontend" => "前端".to_string(),
        "backend" | "server" => "后端".to_string(),
        "coding" | "programming" | "development" => "编程".to_string(),
        "convention" | "best-practices" | "best-practice" | "rules" | "policy" => {
            "编程规范".to_string()
        }
        _ => tag,
    }
}

fn functional_skill_tags(parsed: &ParsedSkill, source: &SkillSourceRef) -> Vec<String> {
    let searchable = format!(
        "{}\n{}\n{}\n{}\n{}",
        parsed.name, parsed.code, parsed.description, parsed.prompt, source.path
    )
    .to_lowercase();
    let mut tags = parsed
        .tags
        .iter()
        .filter(|tag| !is_source_tag(tag))
        .cloned()
        .collect::<Vec<_>>();

    for rule in FUNCTIONAL_TAG_RULES {
        if rule.iter_patterns_match(&searchable) {
            tags.push(rule.tag.to_string());
        }
    }

    if tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "rust"
                | "java"
                | "kotlin"
                | "compose"
                | "tauri"
                | "vue"
                | "react"
                | "gradle"
                | "maven"
                | "docker"
                | "数据库"
        )
    }) {
        tags.push("编程".to_string());
    }

    let normalized = normalize_texts(tags)
        .into_iter()
        .filter(|tag| !is_source_tag(tag))
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        vec!["未分类".to_string()]
    } else {
        normalized
    }
}

fn primary_skill_category(tags: &[String]) -> String {
    for preferred in [
        "自媒体",
        "编程规范",
        "rust",
        "java",
        "kotlin",
        "gradle",
        "maven",
        "compose",
        "tauri",
        "前端",
        "后端",
        "数据库",
        "docker",
        "项目管理",
        "工作流",
        "AI",
        "硬件",
        "编程",
    ] {
        if tags.iter().any(|tag| tag == preferred) {
            return preferred.to_string();
        }
    }

    tags.first()
        .cloned()
        .unwrap_or_else(|| "未分类".to_string())
}

fn is_source_tag(tag: &str) -> bool {
    let normalized = tag.trim().trim_start_matches('#').to_lowercase();
    matches!(
        normalized.as_str(),
        ""
            | "skill"
            | "codex"
            | "agents"
            | "agent"
            | "ssh"
            | "local"
            | "remote"
            | "本机"
            | "远程"
            | "raw.addzero.site"
            | "addzero.site"
            | "openai-bundled"
            | "openai-primary-runtime"
    )
}

impl TagRule {
    fn iter_patterns_match(&self, searchable: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| text_contains_pattern(searchable, pattern))
    }
}

fn text_contains_pattern(searchable: &str, pattern: &str) -> bool {
    let pattern = pattern.to_lowercase();
    if pattern
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
        && pattern.len() <= 3
    {
        return searchable
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| token == pattern);
    }

    searchable.contains(&pattern)
}

async fn find_by_source_tx(
    tx: &mut Transaction<'_, Sqlite>,
    source: &SkillSourceRef,
) -> AppResult<Option<ImportExistingSkill>> {
    let row = sqlx::query_as::<_, ImportExistingSkill>(
        "SELECT skills.id, skills.content_hash
         FROM skills
         INNER JOIN skill_sources ON skill_sources.skill_id = skills.id
         WHERE skill_sources.kind = ?
           AND skill_sources.host = ?
           AND skill_sources.path = ?
         LIMIT 1",
    )
    .bind(&source.kind)
    .bind(&source.host)
    .bind(&source.path)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

async fn find_by_content_hash_tx(
    tx: &mut Transaction<'_, Sqlite>,
    content_hash: &str,
) -> AppResult<Option<ImportExistingSkill>> {
    if content_hash.is_empty() {
        return Ok(None);
    }
    let row = sqlx::query_as::<_, ImportExistingSkill>(
        "SELECT id, content_hash
         FROM skills
         WHERE content_hash = ?
         LIMIT 1",
    )
    .bind(content_hash)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

async fn unique_code_tx(
    tx: &mut Transaction<'_, Sqlite>,
    base: &str,
    content_hash: &str,
    current_id: Option<&str>,
) -> AppResult<String> {
    let base = slugify(base);
    let base = if base.is_empty() {
        "skill".to_string()
    } else {
        base
    };
    let mut candidate = base.clone();
    let suffix = skill_short_hash(content_hash);
    let mut attempt = 0;

    loop {
        let existing = sqlx::query_scalar::<_, String>("SELECT id FROM skills WHERE code = ?")
            .bind(&candidate)
            .fetch_optional(&mut **tx)
            .await?;

        match existing {
            Some(ref id) if current_id.is_some_and(|current| current == id) => {
                return Ok(candidate)
            }
            None => return Ok(candidate),
            _ => {
                attempt += 1;
                candidate = if attempt == 1 {
                    format!("{base}-{suffix}")
                } else {
                    format!("{base}-{suffix}-{attempt}")
                };
            }
        }
    }
}

async fn update_imported_skill_tx(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    code: &str,
    parsed: &ParsedSkill,
    content_hash: &str,
    tags_json: &str,
    now: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE skills
         SET code = ?, name = ?, category = ?, description = ?, prompt = ?,
             content_hash = ?, tags_json = ?, last_synced_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(code)
    .bind(&parsed.name)
    .bind(&parsed.category)
    .bind(&parsed.description)
    .bind(&parsed.prompt)
    .bind(content_hash)
    .bind(tags_json)
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_unique_error)?;
    Ok(())
}

async fn merge_skill_tags_tx(
    tx: &mut Transaction<'_, Sqlite>,
    skill_id: &str,
    tags: Vec<String>,
    now: i64,
) -> AppResult<()> {
    let current = sqlx::query_scalar::<_, String>("SELECT tags_json FROM skills WHERE id = ?")
        .bind(skill_id)
        .fetch_one(&mut **tx)
        .await?;
    let mut merged = serde_json::from_str::<Vec<String>>(&current)
        .map_err(|source| AppError::Json { source })?;
    merged.extend(tags);
    let tags_json = encode_tags(merged)?;
    sqlx::query(
        "UPDATE skills
         SET tags_json = ?, last_synced_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(tags_json)
    .bind(now)
    .bind(now)
    .bind(skill_id)
    .execute(&mut **tx)
    .await
    .map_err(map_unique_error)?;
    Ok(())
}

async fn upsert_source_tx(
    tx: &mut Transaction<'_, Sqlite>,
    skill_id: &str,
    source: &SkillSourceRef,
    now: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO skill_sources
         (id, skill_id, kind, host, root, path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(kind, host, path) DO UPDATE SET
           skill_id = excluded.skill_id,
           root = excluded.root,
           updated_at = excluded.updated_at",
    )
    .bind(new_id())
    .bind(skill_id)
    .bind(&source.kind)
    .bind(&source.host)
    .bind(&source.root)
    .bind(&source.path)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_unique_error)?;
    Ok(())
}

async fn delete_import_orphan_tx(
    tx: &mut Transaction<'_, Sqlite>,
    skill_id: &str,
) -> AppResult<()> {
    let source_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM skill_sources
         WHERE skill_id = ?",
    )
    .bind(skill_id)
    .fetch_one(&mut **tx)
    .await?;

    if source_count == 0 {
        sqlx::query("DELETE FROM skills WHERE id = ? AND last_synced_at <> 0")
            .bind(skill_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

fn slugify(value: &str) -> String {
    let slug = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "skill".to_string()
    } else {
        slug
    }
}
