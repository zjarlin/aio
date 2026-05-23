use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use serde_yml::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use walkdir::WalkDir;

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{map_unique_error, normalize_page, PageInfo, PageRequest, PageResult},
};

const ASSET_KINDS: &[&str] = &[
    "docker_compose",
    "cli",
    "env_vars",
    "bash_functions",
    "dotfiles",
];
const DOCKER_COMPOSE_PAGE_VARIABLE_CATEGORY: &str = "docker_compose_common";
const PAGE_GLOBAL_VARIABLE_DESCRIPTION_PREFIX: &str = "Docker Compose 公共变量";
const PAGE_GLOBAL_VARIABLE_MIN_OCCURRENCES: i64 = 2;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemRecord {
    pub id: String,
    pub kind: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_path: String,
    pub file_name: String,
    pub source_mtime: i64,
    pub source_size: i64,
    pub content_hash: String,
    pub last_synced_at: i64,
    pub service_count: i64,
    pub services: Vec<String>,
    pub images: Vec<String>,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub validation_status: String,
    pub validation_issues: Vec<ValidationIssue>,
    pub variable_candidates: Vec<VariableCandidate>,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
struct AssetItemRow {
    id: String,
    kind: String,
    code: String,
    name: String,
    category: String,
    description: String,
    content: String,
    tags_json: String,
    source_path: String,
    file_name: String,
    source_mtime: i64,
    source_size: i64,
    content_hash: String,
    last_synced_at: i64,
    service_count: i64,
    services_json: String,
    images_json: String,
    ports_json: String,
    volumes_json: String,
    validation_status: String,
    validation_issues_json: String,
    variable_candidates_json: String,
    status: String,
    sort_order: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, FromRow)]
struct ImportExisting {
    id: String,
    content_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemInput {
    pub kind: String,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_path: Option<String>,
    pub file_name: Option<String>,
    pub source_mtime: Option<i64>,
    pub source_size: Option<i64>,
    pub content_hash: Option<String>,
    pub last_synced_at: Option<i64>,
    pub service_count: Option<i64>,
    pub services: Option<Vec<String>>,
    pub images: Option<Vec<String>>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub validation_status: Option<String>,
    pub validation_issues: Option<Vec<ValidationIssue>>,
    pub variable_candidates: Option<Vec<VariableCandidate>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemUpdateInput {
    pub id: String,
    pub kind: String,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_path: Option<String>,
    pub file_name: Option<String>,
    pub source_mtime: Option<i64>,
    pub source_size: Option<i64>,
    pub content_hash: Option<String>,
    pub last_synced_at: Option<i64>,
    pub service_count: Option<i64>,
    pub services: Option<Vec<String>>,
    pub images: Option<Vec<String>>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub validation_status: Option<String>,
    pub validation_issues: Option<Vec<ValidationIssue>>,
    pub variable_candidates: Option<Vec<VariableCandidate>>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemToggleInput {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemPageRequest {
    pub kind: String,
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub categories: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemImportRequest {
    pub kind: String,
    pub root_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemDeployPreviewRequest {
    pub id: String,
    pub root_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemDeployInput {
    pub id: String,
    pub root_path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemImportResult {
    pub scanned: i64,
    pub imported: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub skipped: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemDeployPreview {
    pub id: String,
    pub name: String,
    pub target_path: String,
    pub target_relative_path: String,
    pub exists: bool,
    pub has_conflict: bool,
    pub library_content: String,
    pub local_content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVariableRefreshResult {
    pub scanned: i64,
    pub candidates: i64,
    pub inserted: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub protected: i64,
}

#[derive(Debug, Clone)]
struct ComposeSummary {
    services: Vec<String>,
    images: Vec<String>,
    ports: Vec<String>,
    volumes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub severity: String,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableCandidate {
    pub key: String,
    pub value: String,
    pub kind: String,
    pub scope: String,
    pub source: String,
    pub occurrences: i64,
}

pub async fn page(
    pool: &SqlitePool,
    token: String,
    request: AssetItemPageRequest,
) -> AppResult<PageResult<AssetItemRecord>> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&request.kind)?;
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
        "SELECT COUNT(*) FROM asset_items
         WHERE kind = ?
           {tag_filter_sql}
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR name LIKE ? OR description LIKE ?
             OR content LIKE ? OR tags_json LIKE ? OR source_path LIKE ?
             OR file_name LIKE ? OR services_json LIKE ? OR images_json LIKE ?
             OR ports_json LIKE ? OR volumes_json LIKE ?
             OR validation_issues_json LIKE ? OR variable_candidates_json LIKE ?
           )",
    );
    let mut total_query = sqlx::query_scalar::<_, i64>(&total_sql).bind(&kind);
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
        .bind(&keyword)
        .bind(&keyword)
        .bind(&keyword)
        .fetch_one(pool)
        .await?;

    let rows_sql = format!(
        "SELECT * FROM asset_items
         WHERE kind = ?
           {tag_filter_sql}
           AND (? = '' OR status = ?)
           AND (
             code LIKE ? OR name LIKE ? OR description LIKE ?
             OR content LIKE ? OR tags_json LIKE ? OR source_path LIKE ?
             OR file_name LIKE ? OR services_json LIKE ? OR images_json LIKE ?
             OR ports_json LIKE ? OR volumes_json LIKE ?
             OR validation_issues_json LIKE ? OR variable_candidates_json LIKE ?
           )
         ORDER BY sort_order, updated_at DESC
         LIMIT ? OFFSET ?",
    );
    let mut rows_query = sqlx::query_as::<_, AssetItemRow>(&rows_sql).bind(&kind);
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

pub async fn create(
    pool: &SqlitePool,
    token: String,
    input: AssetItemInput,
) -> AppResult<AssetItemRecord> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&input.kind)?;
    validate_code_name(&input.code, &input.name)?;
    let status = normalize_status(input.status)?;
    let category = input.category.unwrap_or_default();
    let content = input.content.unwrap_or_default();
    let analysis = analyze_yaml(&content, input.file_name.as_deref());
    let derive_yaml_fields = kind == "docker_compose";
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let services = if derive_yaml_fields {
        analysis.summary.services.clone()
    } else {
        input.services.unwrap_or_default()
    };
    let images = if derive_yaml_fields {
        analysis.summary.images.clone()
    } else {
        input.images.unwrap_or_default()
    };
    let ports = if derive_yaml_fields {
        analysis.summary.ports.clone()
    } else {
        input.ports.unwrap_or_default()
    };
    let volumes = if derive_yaml_fields {
        analysis.summary.volumes.clone()
    } else {
        input.volumes.unwrap_or_default()
    };
    let service_count = if derive_yaml_fields {
        analysis.summary.services.len() as i64
    } else {
        input.service_count.unwrap_or(services.len() as i64)
    };
    let validation_status = if derive_yaml_fields {
        analysis.validation_status.clone()
    } else {
        input
            .validation_status
            .unwrap_or_else(|| "unknown".to_string())
    };
    let validation_issues = if derive_yaml_fields {
        analysis.validation_issues.clone()
    } else {
        input.validation_issues.unwrap_or_default()
    };
    let variable_candidates = if derive_yaml_fields {
        merge_variable_candidates(
            analysis.variable_candidates,
            input.variable_candidates.unwrap_or_default(),
        )
    } else {
        input.variable_candidates.unwrap_or_default()
    };
    let services_json = encode_tags(services)?;
    let images_json = encode_tags(images)?;
    let ports_json = encode_tags(ports)?;
    let volumes_json = encode_tags(volumes)?;
    let validation_issues_json = encode_validation_issues(validation_issues)?;
    let variable_candidates_json = encode_variable_candidates(variable_candidates.clone())?;
    let now = now_millis();
    let id = new_id();

    sqlx::query(
        "INSERT INTO asset_items
         (id, kind, code, name, category, description, content, tags_json,
          source_path, file_name, source_mtime, source_size, content_hash, last_synced_at,
          service_count, services_json, images_json, ports_json, volumes_json,
          validation_status, validation_issues_json, variable_candidates_json,
          status, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&kind)
    .bind(input.code)
    .bind(input.name)
    .bind(&category)
    .bind(input.description.unwrap_or_default())
    .bind(&content)
    .bind(tags_json)
    .bind(input.source_path.unwrap_or_default())
    .bind(input.file_name.unwrap_or_default())
    .bind(input.source_mtime.unwrap_or(0))
    .bind(input.source_size.unwrap_or(0))
    .bind(input.content_hash.unwrap_or_else(|| content_hash(&content)))
    .bind(input.last_synced_at.unwrap_or(0))
    .bind(service_count)
    .bind(services_json)
    .bind(images_json)
    .bind(ports_json)
    .bind(volumes_json)
    .bind(validation_status)
    .bind(validation_issues_json)
    .bind(variable_candidates_json)
    .bind(status)
    .bind(input.sort_order.unwrap_or(0))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;

    sync_file_variables(pool, &kind, &id, &category, &variable_candidates).await?;
    if derive_yaml_fields {
        refresh_page_global_variables(pool).await?;
    }
    find(pool, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    token: String,
    input: AssetItemUpdateInput,
) -> AppResult<AssetItemRecord> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&input.kind)?;
    validate_code_name(&input.code, &input.name)?;
    let status = normalize_status(input.status)?;
    let category = input.category.unwrap_or_default();
    let content = input.content.unwrap_or_default();
    let analysis = analyze_yaml(&content, input.file_name.as_deref());
    let derive_yaml_fields = kind == "docker_compose";
    let tags_json = encode_tags(input.tags.unwrap_or_default())?;
    let services = if derive_yaml_fields {
        Some(analysis.summary.services.clone())
    } else {
        input.services
    };
    let images = if derive_yaml_fields {
        Some(analysis.summary.images.clone())
    } else {
        input.images
    };
    let ports = if derive_yaml_fields {
        Some(analysis.summary.ports.clone())
    } else {
        input.ports
    };
    let volumes = if derive_yaml_fields {
        Some(analysis.summary.volumes.clone())
    } else {
        input.volumes
    };
    let service_count = if derive_yaml_fields {
        Some(analysis.summary.services.len() as i64)
    } else {
        input.service_count
    };
    let validation_status = if derive_yaml_fields {
        Some(analysis.validation_status.clone())
    } else {
        input.validation_status
    };
    let validation_issues = if derive_yaml_fields {
        Some(analysis.validation_issues.clone())
    } else {
        input.validation_issues
    };
    let variable_candidates = if derive_yaml_fields {
        Some(merge_variable_candidates(
            analysis.variable_candidates,
            input.variable_candidates.unwrap_or_default(),
        ))
    } else {
        input.variable_candidates
    };
    let services_json = encode_optional_list(services)?;
    let images_json = encode_optional_list(images)?;
    let ports_json = encode_optional_list(ports)?;
    let volumes_json = encode_optional_list(volumes)?;
    let validation_issues_json = encode_optional_validation_issues(validation_issues)?;
    let variable_candidates_for_sync = variable_candidates.clone().unwrap_or_default();
    let variable_candidates_json = encode_optional_variable_candidates(variable_candidates)?;
    let now = now_millis();

    let affected = sqlx::query(
        "UPDATE asset_items
         SET kind = ?, code = ?, name = ?, category = ?, description = ?, content = ?,
             tags_json = ?, source_path = COALESCE(?, source_path),
             file_name = COALESCE(?, file_name),
             source_mtime = COALESCE(?, source_mtime),
             source_size = COALESCE(?, source_size),
             content_hash = COALESCE(?, content_hash),
             last_synced_at = COALESCE(?, last_synced_at),
             service_count = COALESCE(?, service_count),
             services_json = COALESCE(?, services_json),
             images_json = COALESCE(?, images_json),
             ports_json = COALESCE(?, ports_json),
             volumes_json = COALESCE(?, volumes_json),
             validation_status = COALESCE(?, validation_status),
             validation_issues_json = COALESCE(?, validation_issues_json),
             variable_candidates_json = COALESCE(?, variable_candidates_json),
             status = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&kind)
    .bind(input.code)
    .bind(input.name)
    .bind(&category)
    .bind(input.description.unwrap_or_default())
    .bind(&content)
    .bind(tags_json)
    .bind(input.source_path)
    .bind(input.file_name)
    .bind(input.source_mtime)
    .bind(input.source_size)
    .bind(input.content_hash.or_else(|| Some(content_hash(&content))))
    .bind(input.last_synced_at)
    .bind(service_count)
    .bind(services_json)
    .bind(images_json)
    .bind(ports_json)
    .bind(volumes_json)
    .bind(validation_status)
    .bind(validation_issues_json)
    .bind(variable_candidates_json)
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

    if derive_yaml_fields {
        sync_file_variables(
            pool,
            &kind,
            &input.id,
            &category,
            &variable_candidates_for_sync,
        )
        .await?;
        refresh_page_global_variables(pool).await?;
    }
    find(pool, &input.id).await
}

pub async fn delete(pool: &SqlitePool, token: String, id: String) -> AppResult<()> {
    require_session(pool, &token).await?;
    let affected = sqlx::query("DELETE FROM asset_items WHERE id = ?")
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
    input: AssetItemToggleInput,
) -> AppResult<AssetItemRecord> {
    require_session(pool, &token).await?;
    let status = normalize_status(Some(input.status))?;
    let now = now_millis();
    let affected = sqlx::query("UPDATE asset_items SET status = ?, updated_at = ? WHERE id = ?")
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

pub async fn deploy_preview(
    pool: &SqlitePool,
    token: String,
    request: AssetItemDeployPreviewRequest,
) -> AppResult<AssetItemDeployPreview> {
    require_session(pool, &token).await?;
    let record = find(pool, &request.id).await?;
    ensure_docker_compose_item(&record)?;
    let root = deploy_root(&request.root_path)?;
    let relative_path = deploy_relative_path(&record)?;
    let target = root.join(&relative_path);
    let local_content = if target.is_file() {
        fs::read_to_string(&target)?
    } else {
        String::new()
    };
    let exists = !local_content.is_empty() || target.exists();
    let has_conflict = exists && local_content != record.content;

    Ok(AssetItemDeployPreview {
        id: record.id,
        name: record.name,
        target_path: path_to_slash_string(&target),
        target_relative_path: path_to_slash_string(&relative_path),
        exists,
        has_conflict,
        library_content: record.content,
        local_content,
    })
}

pub async fn deploy_save(
    pool: &SqlitePool,
    token: String,
    input: AssetItemDeployInput,
) -> AppResult<AssetItemRecord> {
    require_session(pool, &token).await?;
    let record = find(pool, &input.id).await?;
    ensure_docker_compose_item(&record)?;
    let root = deploy_root(&input.root_path)?;
    let relative_path = deploy_relative_path(&record)?;
    let target = root.join(&relative_path);
    let Some(parent) = target.parent() else {
        return Err(AppError::BadRequest("部署路径不合法".to_string()));
    };
    fs::create_dir_all(parent)?;
    fs::write(&target, input.content.as_bytes())?;
    let metadata = fs::metadata(&target)?;
    let now = now_millis();
    let analysis = analyze_yaml(
        &input.content,
        target.file_name().and_then(|name| name.to_str()),
    );
    let services = analysis.summary.services;
    let images = analysis.summary.images;
    let ports = analysis.summary.ports;
    let volumes = analysis.summary.volumes;
    let service_count = services.len() as i64;
    let services_json = encode_tags(services)?;
    let images_json = encode_tags(images)?;
    let ports_json = encode_tags(ports)?;
    let volumes_json = encode_tags(volumes)?;
    let validation_status = analysis.validation_status;
    let validation_issues_json = encode_validation_issues(analysis.validation_issues)?;
    let variable_candidates = analysis.variable_candidates;
    let variable_candidates_json = encode_variable_candidates(variable_candidates.clone())?;
    let source_path = path_to_slash_string(&target);
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "compose.yml".to_string());

    let affected = sqlx::query(
        "UPDATE asset_items
         SET content = ?, source_path = ?, file_name = ?, source_mtime = ?,
             source_size = ?, content_hash = ?, last_synced_at = ?,
             service_count = ?, services_json = ?, images_json = ?, ports_json = ?,
             volumes_json = ?, validation_status = ?, validation_issues_json = ?,
             variable_candidates_json = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&input.content)
    .bind(&source_path)
    .bind(&file_name)
    .bind(metadata_modified_millis(&metadata))
    .bind(metadata.len() as i64)
    .bind(content_hash(&input.content))
    .bind(now)
    .bind(service_count)
    .bind(services_json)
    .bind(images_json)
    .bind(ports_json)
    .bind(volumes_json)
    .bind(validation_status)
    .bind(validation_issues_json)
    .bind(variable_candidates_json)
    .bind(now)
    .bind(&record.id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    sync_file_variables(
        pool,
        "docker_compose",
        &record.id,
        &record.category,
        &variable_candidates,
    )
    .await?;
    refresh_page_global_variables(pool).await?;
    find(pool, &record.id).await
}

pub async fn import_directory(
    pool: &SqlitePool,
    token: String,
    request: AssetItemImportRequest,
) -> AppResult<AssetItemImportResult> {
    require_session(pool, &token).await?;
    let kind = normalize_kind(&request.kind)?;
    if kind != "docker_compose" {
        return Err(AppError::BadRequest(
            "当前仅支持导入 Docker Compose YAML".to_string(),
        ));
    }

    let root = expand_user_path(request.root_path.trim());
    if !root.is_dir() {
        return Err(AppError::BadRequest("导入目录不存在".to_string()));
    }

    let mut result = AssetItemImportResult {
        scanned: 0,
        imported: 0,
        updated: 0,
        unchanged: 0,
        skipped: 0,
    };
    let now = now_millis();
    let mut tx = pool.begin().await?;
    let mut variable_syncs = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| is_yaml_file(entry.path()))
    {
        result.scanned += 1;

        let path = entry.path();
        let Ok(content) = fs::read_to_string(path) else {
            result.skipped += 1;
            continue;
        };
        let Ok(metadata) = fs::metadata(path) else {
            result.skipped += 1;
            continue;
        };
        let (summary, parse_error) = match compose_summary(&content) {
            Ok(Some(summary)) => (summary, None),
            Ok(None) => (empty_compose_summary(), None),
            Err(error) => (empty_compose_summary(), Some(error.to_string())),
        };

        let source_path = path.to_string_lossy().to_string();
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let source_mtime = metadata_modified_millis(&metadata);
        let source_size = metadata.len() as i64;
        let content_hash = content_hash(&content);
        let category = compose_category(&root, path);
        let code = compose_code(&root, path);
        let name = compose_name(&root, path);
        let mut tags = vec!["yaml".to_string(), category.clone()];
        if summary.services.is_empty() {
            if parse_error.is_some() {
                tags.push("yaml-error".to_string());
            }
        } else {
            tags.push("docker".to_string());
            tags.push("compose".to_string());
        }
        let tags = encode_tags(tags)?;
        let services_json = encode_tags(summary.services.clone())?;
        let images_json = encode_tags(summary.images.clone())?;
        let ports_json = encode_tags(summary.ports.clone())?;
        let volumes_json = encode_tags(summary.volumes.clone())?;
        let description = yaml_description(&summary, parse_error.as_deref());
        let validation_issues =
            validate_yaml_content(file_name.as_str(), &summary, parse_error.as_deref());
        let validation_status = validation_status(&validation_issues);
        let validation_issues_json = encode_validation_issues(validation_issues)?;
        let variable_candidates = extract_variable_candidates(&content, &summary);
        let variable_candidates_json = encode_variable_candidates(variable_candidates.clone())?;

        let existing = sqlx::query_as::<_, ImportExisting>(
            "SELECT id, content_hash
             FROM asset_items
             WHERE kind = ? AND source_path = ?",
        )
        .bind(&kind)
        .bind(&source_path)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(existing) = existing {
            let changed = existing.content_hash != content_hash;
            let id = existing.id;
            sqlx::query(
                "UPDATE asset_items
                 SET code = ?, name = ?, category = ?, description = ?, content = ?,
                     tags_json = ?, file_name = ?, source_mtime = ?, source_size = ?,
                     content_hash = ?, last_synced_at = ?, service_count = ?,
                     services_json = ?, images_json = ?, ports_json = ?, volumes_json = ?,
                     validation_status = ?, validation_issues_json = ?,
                     variable_candidates_json = ?,
                     updated_at = ?
                 WHERE id = ?",
            )
            .bind(code)
            .bind(name)
            .bind(&category)
            .bind(description)
            .bind(content)
            .bind(tags)
            .bind(file_name)
            .bind(source_mtime)
            .bind(source_size)
            .bind(content_hash)
            .bind(now)
            .bind(summary.services.len() as i64)
            .bind(services_json)
            .bind(images_json)
            .bind(ports_json)
            .bind(volumes_json)
            .bind(validation_status)
            .bind(validation_issues_json)
            .bind(variable_candidates_json)
            .bind(now)
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(map_unique_error)?;
            variable_syncs.push((kind.clone(), id, category, variable_candidates));

            if changed {
                result.updated += 1;
            } else {
                result.unchanged += 1;
            }
        } else {
            let id = new_id();
            sqlx::query(
                "INSERT INTO asset_items
                 (id, kind, code, name, category, description, content, tags_json,
                  source_path, file_name, source_mtime, source_size, content_hash, last_synced_at,
                  service_count, services_json, images_json, ports_json, volumes_json,
                  validation_status, validation_issues_json, variable_candidates_json,
                  status, sort_order, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                         'enabled', 0, ?, ?)",
            )
            .bind(&id)
            .bind(&kind)
            .bind(code)
            .bind(name)
            .bind(&category)
            .bind(description)
            .bind(content)
            .bind(tags)
            .bind(source_path)
            .bind(file_name)
            .bind(source_mtime)
            .bind(source_size)
            .bind(content_hash)
            .bind(now)
            .bind(summary.services.len() as i64)
            .bind(services_json)
            .bind(images_json)
            .bind(ports_json)
            .bind(volumes_json)
            .bind(validation_status)
            .bind(validation_issues_json)
            .bind(variable_candidates_json)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_unique_error)?;
            variable_syncs.push((kind.clone(), id, category, variable_candidates));

            result.imported += 1;
        }
    }

    tx.commit().await?;
    for (kind, asset_item_id, category, candidates) in variable_syncs {
        sync_file_variables(pool, &kind, &asset_item_id, &category, &candidates).await?;
    }
    refresh_page_global_variables(pool).await?;
    Ok(result)
}

pub async fn backfill_file_variables(pool: &SqlitePool) -> AppResult<()> {
    let variable_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM asset_variables WHERE asset_item_id IS NOT NULL")
            .fetch_one(pool)
            .await?;
    if variable_count > 0 {
        return Ok(());
    }

    let rows = sqlx::query_as::<_, AssetItemVariableBackfillRow>(
        "SELECT id, category, variable_candidates_json
         FROM asset_items
         WHERE kind = 'docker_compose'
           AND variable_candidates_json <> '[]'",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let candidates = decode_variable_candidates(&row.variable_candidates_json)?;
        sync_file_variables(pool, "docker_compose", &row.id, &row.category, &candidates).await?;
    }

    Ok(())
}

pub async fn refresh_page_global_variables_for_user(
    pool: &SqlitePool,
    token: String,
) -> AppResult<AssetVariableRefreshResult> {
    require_session(pool, &token).await?;
    refresh_page_global_variables(pool).await
}

pub async fn refresh_page_global_variables(
    pool: &SqlitePool,
) -> AppResult<AssetVariableRefreshResult> {
    let rows = sqlx::query_as::<_, AssetItemGlobalVariableScanRow>(
        "SELECT content
         FROM asset_items
         WHERE kind = 'docker_compose'
           AND status = 'enabled'
           AND service_count > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut aggregate = PageGlobalVariableAggregate::default();
    for row in &rows {
        collect_page_global_variables(&row.content, &mut aggregate);
    }

    let candidates = aggregate.finish();
    let mut result = AssetVariableRefreshResult {
        scanned: rows.len() as i64,
        candidates: candidates.len() as i64,
        inserted: 0,
        updated: 0,
        unchanged: 0,
        protected: 0,
    };
    let now = now_millis();

    for (index, candidate) in candidates.iter().enumerate() {
        upsert_page_global_variable(pool, candidate, index as i64, now, &mut result).await?;
    }

    Ok(result)
}

async fn upsert_page_global_variable(
    pool: &SqlitePool,
    candidate: &VariableCandidate,
    index: i64,
    now: i64,
    result: &mut AssetVariableRefreshResult,
) -> AppResult<()> {
    let key = sanitize_variable_key(&candidate.key, "VALUE");
    let value = candidate.value.trim().to_string();
    if key.is_empty() || value.is_empty() {
        return Ok(());
    }

    let value_kind = candidate.kind.trim().to_ascii_lowercase();
    let description = format!(
        "{PAGE_GLOBAL_VARIABLE_DESCRIPTION_PREFIX}：{key}，从 {} 处环境变量或挂载路径归纳",
        candidate.occurrences.max(1)
    );
    let sort_order = page_global_variable_sort_order(&key, index);

    let existing = sqlx::query_as::<_, ExistingPageGlobalVariable>(
        "SELECT id, source, value, default_value, value_kind, description, sort_order
         FROM asset_variables
         WHERE kind = 'docker_compose'
           AND asset_item_id IS NULL
           AND category = ?
           AND key = ?
         LIMIT 1",
    )
    .bind(DOCKER_COMPOSE_PAGE_VARIABLE_CATEGORY)
    .bind(&key)
    .fetch_optional(pool)
    .await?;

    if let Some(existing) = existing {
        if !is_refresh_owned_page_global_variable(&existing.source, &existing.description) {
            result.protected += 1;
            return Ok(());
        }

        if existing.value == value
            && existing.default_value == value
            && existing.value_kind == value_kind
            && existing.description == description
            && existing.sort_order == sort_order
        {
            result.unchanged += 1;
            return Ok(());
        }

        sqlx::query(
            "UPDATE asset_variables
             SET value = ?, default_value = ?, description = ?, value_kind = ?,
                 source = 'ai', status = 'enabled', sort_order = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&value)
        .bind(&value)
        .bind(&description)
        .bind(&value_kind)
        .bind(sort_order)
        .bind(now)
        .bind(existing.id)
        .execute(pool)
        .await?;
        result.updated += 1;
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO asset_variables
         (id, kind, asset_item_id, category, key, value, default_value, description,
          value_kind, source, status, sort_order, created_at, updated_at)
         VALUES (?, 'docker_compose', NULL, ?, ?, ?, ?, ?, ?, 'ai', 'enabled', ?, ?, ?)",
    )
    .bind(new_id())
    .bind(DOCKER_COMPOSE_PAGE_VARIABLE_CATEGORY)
    .bind(&key)
    .bind(&value)
    .bind(&value)
    .bind(description)
    .bind(value_kind)
    .bind(sort_order)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_unique_error)?;
    result.inserted += 1;
    Ok(())
}

fn is_refresh_owned_page_global_variable(source: &str, description: &str) -> bool {
    source == "ai" && description.starts_with(PAGE_GLOBAL_VARIABLE_DESCRIPTION_PREFIX)
}

fn page_global_variable_sort_order(key: &str, index: i64) -> i64 {
    match key {
        "MOUNT_DIR" => 10,
        "DATA_DIR" => 20,
        "CONFIG_DIR" => 30,
        "CONFIG_FILE" => 40,
        "CONF_D_DIR" => 50,
        "LOG_DIR" => 60,
        "CACHE_DIR" => 70,
        "WORK_DIR" => 80,
        "HTML_DIR" => 90,
        "MEDIA_DIR" => 100,
        "DOCKER_SOCK_PATH" => 110,
        "DEVICE_PATH" => 120,
        "TZ" => 200,
        "LANG" => 210,
        "LANGUAGE" => 220,
        "LC_ALL" => 230,
        "PUID" => 240,
        "PGID" => 250,
        "USERNAME" => 260,
        "PASSWORD" => 270,
        _ => 500 + index,
    }
}

async fn find(pool: &SqlitePool, id: &str) -> AppResult<AssetItemRecord> {
    let row = sqlx::query_as::<_, AssetItemRow>("SELECT * FROM asset_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    to_record(row)
}

async fn sync_file_variables(
    pool: &SqlitePool,
    kind: &str,
    asset_item_id: &str,
    category: &str,
    candidates: &[VariableCandidate],
) -> AppResult<()> {
    let now = now_millis();
    let mut candidate_keys = HashSet::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.scope == "file")
        .filter(|candidate| !candidate.key.trim().is_empty())
    {
        let key = sanitize_variable_key(&candidate.key, "VALUE");
        candidate_keys.insert(key.clone());
        let default_value = candidate.value.trim().to_string();
        let value_kind = candidate.kind.trim().to_ascii_lowercase();
        let source = candidate.source.trim().to_ascii_lowercase();

        let existing = sqlx::query_as::<_, ExistingAssetVariable>(
            "SELECT id, source, key
             FROM asset_variables
             WHERE kind = ? AND asset_item_id = ? AND key = ?",
        )
        .bind(kind)
        .bind(asset_item_id)
        .bind(&key)
        .fetch_optional(pool)
        .await?;

        if let Some(existing) = existing {
            if is_protected_variable_source(&existing.source) {
                sqlx::query(
                    "UPDATE asset_variables
                     SET category = ?, default_value = ?, value_kind = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(category)
                .bind(&default_value)
                .bind(&value_kind)
                .bind(now)
                .bind(existing.id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE asset_variables
                     SET category = ?, value = ?, default_value = ?, value_kind = ?,
                         source = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(category)
                .bind(&default_value)
                .bind(&default_value)
                .bind(&value_kind)
                .bind(&source)
                .bind(now)
                .bind(existing.id)
                .execute(pool)
                .await?;
            }
            continue;
        }

        sqlx::query(
            "INSERT INTO asset_variables
             (id, kind, asset_item_id, category, key, value, default_value, description,
              value_kind, source, status, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, '', ?, ?, 'enabled', 0, ?, ?)",
        )
        .bind(new_id())
        .bind(kind)
        .bind(asset_item_id)
        .bind(category)
        .bind(&key)
        .bind(&default_value)
        .bind(&default_value)
        .bind(&value_kind)
        .bind(&source)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    let existing_rows = sqlx::query_as::<_, ExistingAssetVariable>(
        "SELECT id, source, key
         FROM asset_variables
         WHERE kind = ? AND asset_item_id = ?",
    )
    .bind(kind)
    .bind(asset_item_id)
    .fetch_all(pool)
    .await?;
    for row in existing_rows
        .into_iter()
        .filter(|row| !is_protected_variable_source(&row.source))
        .filter(|row| !candidate_keys.contains(&row.key))
    {
        sqlx::query("DELETE FROM asset_variables WHERE id = ?")
            .bind(row.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct ExistingAssetVariable {
    id: String,
    source: String,
    key: String,
}

#[derive(Debug, FromRow)]
struct ExistingPageGlobalVariable {
    id: String,
    source: String,
    value: String,
    default_value: String,
    value_kind: String,
    description: String,
    sort_order: i64,
}

#[derive(Debug, FromRow)]
struct AssetItemVariableBackfillRow {
    id: String,
    category: String,
    variable_candidates_json: String,
}

#[derive(Debug, FromRow)]
struct AssetItemGlobalVariableScanRow {
    content: String,
}

fn is_protected_variable_source(source: &str) -> bool {
    matches!(source, "manual" | "selection")
}

fn to_record(row: AssetItemRow) -> AppResult<AssetItemRecord> {
    let tags = serde_json::from_str::<Vec<String>>(&row.tags_json)
        .map_err(|source| AppError::Json { source })?;
    let services = decode_list(&row.services_json)?;
    let images = decode_list(&row.images_json)?;
    let ports = decode_list(&row.ports_json)?;
    let volumes = decode_list(&row.volumes_json)?;
    let validation_issues = decode_validation_issues(&row.validation_issues_json)?;
    let variable_candidates = decode_variable_candidates(&row.variable_candidates_json)?;

    Ok(AssetItemRecord {
        id: row.id,
        kind: row.kind,
        code: row.code,
        name: row.name,
        category: row.category,
        description: row.description,
        content: row.content,
        tags,
        source_path: row.source_path,
        file_name: row.file_name,
        source_mtime: row.source_mtime,
        source_size: row.source_size,
        content_hash: row.content_hash,
        last_synced_at: row.last_synced_at,
        service_count: row.service_count,
        services,
        images,
        ports,
        volumes,
        validation_status: row.validation_status,
        validation_issues,
        variable_candidates,
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

fn encode_optional_list(values: Option<Vec<String>>) -> AppResult<Option<String>> {
    values.map(encode_tags).transpose()
}

fn decode_list(value: &str) -> AppResult<Vec<String>> {
    serde_json::from_str::<Vec<String>>(value).map_err(|source| AppError::Json { source })
}

fn encode_validation_issues(issues: Vec<ValidationIssue>) -> AppResult<String> {
    serde_json::to_string(&issues).map_err(|source| AppError::Json { source })
}

fn encode_optional_validation_issues(
    issues: Option<Vec<ValidationIssue>>,
) -> AppResult<Option<String>> {
    issues.map(encode_validation_issues).transpose()
}

fn decode_validation_issues(value: &str) -> AppResult<Vec<ValidationIssue>> {
    serde_json::from_str::<Vec<ValidationIssue>>(value).map_err(|source| AppError::Json { source })
}

fn encode_variable_candidates(candidates: Vec<VariableCandidate>) -> AppResult<String> {
    serde_json::to_string(&candidates).map_err(|source| AppError::Json { source })
}

fn encode_optional_variable_candidates(
    candidates: Option<Vec<VariableCandidate>>,
) -> AppResult<Option<String>> {
    candidates.map(encode_variable_candidates).transpose()
}

fn decode_variable_candidates(value: &str) -> AppResult<Vec<VariableCandidate>> {
    serde_json::from_str::<Vec<VariableCandidate>>(value)
        .map_err(|source| AppError::Json { source })
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "yml" | "yaml"))
        .unwrap_or(false)
}

fn expand_user_path(value: &str) -> PathBuf {
    let value = value.trim();
    if value == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(value)
}

fn ensure_docker_compose_item(record: &AssetItemRecord) -> AppResult<()> {
    if record.kind == "docker_compose" {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "当前仅支持部署 Docker Compose YAML".to_string(),
        ))
    }
}

fn deploy_root(root_path: &str) -> AppResult<PathBuf> {
    let root = expand_user_path(root_path.trim());
    if root.as_os_str().is_empty() {
        return Err(AppError::BadRequest("部署目录不能为空".to_string()));
    }
    Ok(root)
}

fn deploy_relative_path(record: &AssetItemRecord) -> AppResult<PathBuf> {
    let name = record.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("名称不能为空".to_string()));
    }

    let relative = PathBuf::from(name);
    if !is_safe_relative_path(&relative) {
        return Err(AppError::BadRequest(
            "名称不能包含绝对路径或 ..".to_string(),
        ));
    }
    if is_yaml_file(&relative) {
        return Ok(relative);
    }
    Ok(relative.join("compose.yml"))
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

struct YamlAnalysis {
    summary: ComposeSummary,
    validation_status: String,
    validation_issues: Vec<ValidationIssue>,
    variable_candidates: Vec<VariableCandidate>,
}

fn analyze_yaml(content: &str, file_name: Option<&str>) -> YamlAnalysis {
    let (summary, parse_error) = match compose_summary(content) {
        Ok(Some(summary)) => (summary, None),
        Ok(None) => (empty_compose_summary(), None),
        Err(error) => (empty_compose_summary(), Some(error.to_string())),
    };
    let validation_issues = validate_yaml_content(
        file_name.unwrap_or_default(),
        &summary,
        parse_error.as_deref(),
    );
    let validation_status = validation_status(&validation_issues);
    let variable_candidates = extract_variable_candidates(content, &summary);

    YamlAnalysis {
        summary,
        validation_status,
        validation_issues,
        variable_candidates,
    }
}

fn compose_summary(content: &str) -> Result<Option<ComposeSummary>, serde_yml::Error> {
    let value = serde_yml::from_str::<Value>(content)?;
    let Some(services) = value.get("services").and_then(Value::as_mapping) else {
        return Ok(None);
    };
    if services.is_empty() {
        return Ok(None);
    }

    let mut service_names = Vec::new();
    let mut images = Vec::new();
    let mut ports = Vec::new();
    let mut volumes = Vec::new();

    for (service_key, service_value) in services {
        if let Some(service_name) = value_to_string(service_key) {
            service_names.push(service_name);
        }

        let Some(service) = service_value.as_mapping() else {
            continue;
        };

        if let Some(image) = map_value(service, "image").and_then(value_to_string) {
            images.push(image);
        }
        collect_list_field(service, "ports", &mut ports);
        collect_list_field(service, "volumes", &mut volumes);
    }

    Ok(Some(ComposeSummary {
        services: unique_sorted(service_names),
        images: unique_sorted(images),
        ports: unique_sorted(ports),
        volumes: unique_sorted(volumes),
    }))
}

fn empty_compose_summary() -> ComposeSummary {
    ComposeSummary {
        services: Vec::new(),
        images: Vec::new(),
        ports: Vec::new(),
        volumes: Vec::new(),
    }
}

fn validate_yaml_content(
    file_name: &str,
    summary: &ComposeSummary,
    parse_error: Option<&str>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if let Some(error) = parse_error {
        issues.push(ValidationIssue {
            severity: "error".to_string(),
            message: format!("YAML 解析失败：{error}"),
            path: file_name.to_string(),
        });
        return issues;
    }

    if summary.services.is_empty() && is_compose_file_name(file_name) {
        issues.push(ValidationIssue {
            severity: "error".to_string(),
            message: "Compose 文件缺少顶层 services".to_string(),
            path: "services".to_string(),
        });
    }

    if !summary.services.is_empty() && summary.images.is_empty() {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            message: "未识别到 image 字段，确认服务是否通过 build/extends 定义".to_string(),
            path: "services.*.image".to_string(),
        });
    }

    for port in &summary.ports {
        if port.contains(':') && port.split(':').any(str::is_empty) {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                message: format!("端口映射格式异常：{port}"),
                path: "services.*.ports".to_string(),
            });
        }
    }

    issues
}

fn validation_status(issues: &[ValidationIssue]) -> String {
    if issues.iter().any(|issue| issue.severity == "error") {
        "error".to_string()
    } else if issues.iter().any(|issue| issue.severity == "warning") {
        "warning".to_string()
    } else {
        "valid".to_string()
    }
}

fn is_compose_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    file_name.contains("compose") || file_name.contains("docker-compose")
}

fn extract_variable_candidates(content: &str, summary: &ComposeSummary) -> Vec<VariableCandidate> {
    let mut candidates = Vec::new();

    for image in &summary.images {
        push_candidate(
            &mut candidates,
            image_key(image),
            image,
            "image",
            "file",
            "ai",
        );
    }
    for port in &summary.ports {
        push_candidate(&mut candidates, port_key(port), port, "port", "file", "ai");
    }
    for volume in &summary.volumes {
        if let Some(source) =
            split_compose_volume_source(volume).and_then(|source| normalize_mount_path(&source))
        {
            push_candidate(
                &mut candidates,
                path_key(&source),
                &source,
                "path",
                "file",
                "ai",
            );
        }
        for part in volume.split(':').skip(1).take(1) {
            let value = part.trim();
            if looks_like_path(value) {
                push_candidate(
                    &mut candidates,
                    path_key(value),
                    value,
                    "path",
                    "file",
                    "ai",
                );
            }
        }
    }

    if let Ok(value) = serde_yml::from_str::<Value>(content) {
        collect_variable_candidates_from_value(&value, "", &mut candidates);
    } else {
        collect_regex_variable_candidates(content, &mut candidates);
    }

    merge_variable_candidates(Vec::new(), candidates)
}

#[derive(Debug, Default)]
struct PageGlobalVariableAggregate {
    entries: HashMap<String, PageGlobalVariableEntry>,
}

#[derive(Debug)]
struct PageGlobalVariableEntry {
    kind: String,
    occurrences: i64,
    values: HashMap<String, i64>,
}

impl PageGlobalVariableAggregate {
    fn record(&mut self, key: &str, value: &str, kind: &str) {
        let key = sanitize_variable_key(key, "VALUE");
        let Some(value) = normalize_global_variable_value(value) else {
            return;
        };

        let entry = self
            .entries
            .entry(key)
            .or_insert_with(|| PageGlobalVariableEntry {
                kind: kind.to_string(),
                occurrences: 0,
                values: HashMap::new(),
            });
        if entry.kind != "secret" && kind == "secret" {
            entry.kind = kind.to_string();
        }
        entry.occurrences += 1;
        *entry.values.entry(value).or_insert(0) += 1;
    }

    fn finish(self) -> Vec<VariableCandidate> {
        let mut candidates = self
            .entries
            .into_iter()
            .filter(|(_, entry)| entry.occurrences >= PAGE_GLOBAL_VARIABLE_MIN_OCCURRENCES)
            .filter_map(|(key, entry)| {
                let value = select_page_global_value(&key, entry.values)?;
                Some(VariableCandidate {
                    key,
                    value,
                    kind: entry.kind,
                    scope: "grid".to_string(),
                    source: "ai".to_string(),
                    occurrences: entry.occurrences,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            page_global_variable_sort_order(&left.key, 0)
                .cmp(&page_global_variable_sort_order(&right.key, 0))
                .then(left.key.cmp(&right.key))
        });
        candidates
    }
}

fn collect_page_global_variables(content: &str, aggregate: &mut PageGlobalVariableAggregate) {
    let Ok(value) = serde_yml::from_str::<Value>(content) else {
        return;
    };
    let Some(services) = value.get("services").and_then(Value::as_mapping) else {
        return;
    };

    for (_, service_value) in services {
        let Some(service) = service_value.as_mapping() else {
            continue;
        };
        if let Some(environment) = map_value(service, "environment") {
            collect_page_global_environment(environment, aggregate);
        }
        if let Some(volumes) = map_value(service, "volumes") {
            collect_page_global_mounts(volumes, aggregate);
        }
    }
}

fn collect_page_global_environment(value: &Value, aggregate: &mut PageGlobalVariableAggregate) {
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let Some(key) = value_to_string(key) else {
                    continue;
                };
                let Some(value) = scalar_value_to_string(value) else {
                    continue;
                };
                record_page_global_environment(aggregate, &key, &value);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                match item {
                    Value::String(value) => {
                        if let Some((key, value)) = parse_environment_assignment(value) {
                            record_page_global_environment(aggregate, &key, &value);
                        }
                    }
                    Value::Mapping(mapping) => {
                        for (key, value) in mapping {
                            let Some(key) = value_to_string(key) else {
                                continue;
                            };
                            let Some(value) = scalar_value_to_string(value) else {
                                continue;
                            };
                            record_page_global_environment(aggregate, &key, &value);
                        }
                    }
                    Value::Tagged(tagged) => {
                        collect_page_global_environment(&tagged.value, aggregate)
                    }
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::Sequence(_) => {}
                }
            }
        }
        Value::String(value) => {
            if let Some((key, value)) = parse_environment_assignment(value) {
                record_page_global_environment(aggregate, &key, &value);
            }
        }
        Value::Tagged(tagged) => collect_page_global_environment(&tagged.value, aggregate),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn record_page_global_environment(
    aggregate: &mut PageGlobalVariableAggregate,
    key: &str,
    value: &str,
) {
    let Some(key) = normalize_page_global_environment_key(key) else {
        return;
    };
    let kind = page_global_environment_kind(&key);
    aggregate.record(&key, value, kind);
}

fn normalize_page_global_environment_key(key: &str) -> Option<String> {
    let key = sanitize_variable_key(key, "VALUE");
    match key.as_str() {
        "TZ" | "TIME_ZONE" => Some("TZ".to_string()),
        "LANG" => Some("LANG".to_string()),
        "LANGUAGE" => Some("LANGUAGE".to_string()),
        "LC_ALL" => Some("LC_ALL".to_string()),
        "UID" | "PUID" => Some("PUID".to_string()),
        "GID" | "PGID" => Some("PGID".to_string()),
        _ if is_password_environment_key(&key) => Some("PASSWORD".to_string()),
        _ if is_username_environment_key(&key) => Some("USERNAME".to_string()),
        _ => None,
    }
}

fn is_password_environment_key(key: &str) -> bool {
    key == "PASSWORD"
        || key == "PASS"
        || key == "PASSWD"
        || key.contains("PASSWORD")
        || key.ends_with("_PASS")
        || key.ends_with("_PASSWD")
}

fn is_username_environment_key(key: &str) -> bool {
    key == "USER"
        || key == "USERNAME"
        || key == "CUSTOM_USER"
        || key.ends_with("_USER")
        || key.ends_with("_USERNAME")
}

fn page_global_environment_kind(key: &str) -> &'static str {
    match key {
        "PASSWORD" => "secret",
        "TZ" => "timezone",
        _ => "text",
    }
}

fn collect_page_global_mounts(value: &Value, aggregate: &mut PageGlobalVariableAggregate) {
    match value {
        Value::Sequence(items) => {
            for item in items {
                match item {
                    Value::String(value) => {
                        if let Some(source) = split_compose_volume_source(value) {
                            record_page_global_mount_path(aggregate, &source);
                        }
                    }
                    Value::Mapping(mapping) => {
                        for source_key in ["source", "src", "host"] {
                            if let Some(source) =
                                map_value(mapping, source_key).and_then(scalar_value_to_string)
                            {
                                record_page_global_mount_path(aggregate, &source);
                                break;
                            }
                        }
                    }
                    Value::Tagged(tagged) => collect_page_global_mounts(&tagged.value, aggregate),
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::Sequence(_) => {}
                }
            }
        }
        Value::String(value) => {
            if let Some(source) = split_compose_volume_source(value) {
                record_page_global_mount_path(aggregate, &source);
            }
        }
        Value::Tagged(tagged) => collect_page_global_mounts(&tagged.value, aggregate),
        Value::Mapping(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn record_page_global_mount_path(aggregate: &mut PageGlobalVariableAggregate, value: &str) {
    let Some(path) = normalize_mount_path(value) else {
        return;
    };
    if is_noise_mount_path(&path) {
        return;
    }
    if let Some(root) = page_global_mount_root(&path) {
        aggregate.record("MOUNT_DIR", &root, "path");
    }
    if let Some(key) = page_global_mount_key(&path) {
        aggregate.record(&key, &path, "path");
    }
}

fn parse_environment_assignment(value: &str) -> Option<(String, String)> {
    let value = strip_wrapping_quotes(value.trim());
    let (key, value) = value.split_once('=')?;
    let key = sanitize_variable_key(key, "VALUE");
    if key == "VALUE" {
        return None;
    }
    Some((key, value.trim().to_string()))
}

fn split_compose_volume_source(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let mut interpolation_depth = 0;
    let mut chars = value.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        if character == '$' && matches!(chars.peek(), Some((_, next)) if *next == '{') {
            interpolation_depth += 1;
            chars.next();
            continue;
        }
        if character == '}' && interpolation_depth > 0 {
            interpolation_depth -= 1;
            continue;
        }
        if character == ':' && interpolation_depth == 0 {
            let source = value[..index].trim();
            return (!source.is_empty()).then(|| source.to_string());
        }
    }
    None
}

fn normalize_mount_path(value: &str) -> Option<String> {
    let value = normalize_global_variable_value(value)?;
    if !looks_like_path(&value) {
        return None;
    }
    let normalized = if value != "/" {
        value.trim_end_matches('/').to_string()
    } else {
        value
    };
    (!normalized.is_empty()).then_some(normalized)
}

fn is_noise_mount_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path == "/etc/localtime" || path == "/etc/timezone" || path.starts_with("/usr/share/zoneinfo/")
}

fn page_global_mount_root(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    if let Some(index) = lower.find("/dockercompose") {
        let end = index + "/dockercompose".len();
        return Some(path[..end].to_string());
    }
    if lower.starts_with("/docker-data/") || lower == "/docker-data" {
        return Some("/docker-data".to_string());
    }
    None
}

fn page_global_mount_key(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with("/docker.sock") {
        return Some("DOCKER_SOCK_PATH".to_string());
    }
    if lower == "/dev/dri" || lower.starts_with("/dev/") {
        return Some("DEVICE_PATH".to_string());
    }
    if is_config_file_path(&lower) {
        return Some("CONFIG_FILE".to_string());
    }

    let segments = lower
        .split('/')
        .filter(|part| !part.is_empty() && *part != "~" && *part != ".")
        .collect::<Vec<_>>();
    let tail = segments.last().copied().unwrap_or_default();

    match tail {
        "data" | "db" | "database" | "mysql" | "postgres" | "storage" | "shared" | "files" => {
            Some("DATA_DIR".to_string())
        }
        "config" => Some("CONFIG_DIR".to_string()),
        "conf" => Some("CONFIG_DIR".to_string()),
        "conf.d" => Some("CONF_D_DIR".to_string()),
        "log" | "logs" => Some("LOG_DIR".to_string()),
        "cache" => Some("CACHE_DIR".to_string()),
        "workspace" | "work" => Some("WORK_DIR".to_string()),
        "html" => Some("HTML_DIR".to_string()),
        "media" => Some("MEDIA_DIR".to_string()),
        _ if segments
            .iter()
            .any(|segment| matches!(*segment, "config" | "conf")) =>
        {
            Some("CONFIG_DIR".to_string())
        }
        _ => None,
    }
}

fn is_config_file_path(path: &str) -> bool {
    [
        ".conf",
        ".env",
        ".json",
        ".list",
        ".properties",
        ".toml",
        ".yaml",
        ".yml",
    ]
    .iter()
    .any(|extension| path.ends_with(extension))
}

fn normalize_global_variable_value(value: &str) -> Option<String> {
    let value = strip_wrapping_quotes(value.trim());
    let value = placeholder_default(&value).unwrap_or(value);
    if value.is_empty() || value.len() > 240 || unresolved_placeholder(&value) {
        return None;
    }
    Some(value)
}

fn strip_wrapping_quotes(value: &str) -> String {
    let mut value = value.trim();
    loop {
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = &value[1..value.len() - 1];
        } else {
            break;
        }
    }
    value.trim().to_string()
}

fn placeholder_default(value: &str) -> Option<String> {
    let inner = value.strip_prefix("${")?.strip_suffix('}')?.trim();
    for operator in [":-", ":=", "-"] {
        if let Some((_, default_value)) = inner.split_once(operator) {
            let default_value = strip_wrapping_quotes(default_value.trim());
            return (!default_value.is_empty()).then_some(default_value);
        }
    }
    None
}

fn unresolved_placeholder(value: &str) -> bool {
    value.starts_with("${") && value.ends_with('}')
}

fn select_page_global_value(key: &str, values: HashMap<String, i64>) -> Option<String> {
    if key == "USERNAME" {
        if let Ok(user) = std::env::var("USER") {
            if values.contains_key(&user) {
                return Some(user);
            }
        }
        if values.contains_key("admin") {
            return Some("admin".to_string());
        }
    }

    if matches!(key, "PUID" | "PGID") && values.contains_key("1000") {
        return Some("1000".to_string());
    }

    let mut values = values.into_iter().collect::<Vec<_>>();
    if key == "MOUNT_DIR" {
        values.sort_by(|(left, left_count), (right, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| {
                    right
                        .contains("DockerCompose")
                        .cmp(&left.contains("DockerCompose"))
                })
                .then_with(|| left.len().cmp(&right.len()))
                .then_with(|| left.cmp(right))
        });
    } else {
        values.sort_by(|(left, left_count), (right, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left.len().cmp(&right.len()))
                .then_with(|| left.cmp(right))
        });
    }
    values.into_iter().next().map(|(value, _)| value)
}

fn collect_variable_candidates_from_value(
    value: &Value,
    path: &str,
    candidates: &mut Vec<VariableCandidate>,
) {
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let key = value_to_string(key).unwrap_or_else(|| "value".to_string());
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                collect_variable_candidates_from_value(value, &child_path, candidates);
            }
        }
        Value::Sequence(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_variable_candidates_from_value(
                    item,
                    &format!("{path}[{index}]"),
                    candidates,
                );
            }
        }
        Value::String(value) => {
            collect_environment_candidate(path, value, candidates);
            collect_string_candidate(path, value, candidates);
            if let Some((key, value)) = value.split_once('=') {
                collect_string_candidate(key, value, candidates);
            }
        }
        Value::Number(value) => {
            let string_value = value.to_string();
            if let Some(key) = environment_key_from_path(path) {
                push_environment_candidate(candidates, &key, &string_value);
            }
            if path.to_ascii_lowercase().contains("port") {
                push_candidate(
                    candidates,
                    key_from_path(path, "PORT"),
                    &string_value,
                    "port",
                    "file",
                    "ai",
                );
            }
        }
        Value::Tagged(tagged) => {
            collect_variable_candidates_from_value(&tagged.value, path, candidates);
        }
        Value::Bool(value) => {
            if let Some(key) = environment_key_from_path(path) {
                push_environment_candidate(candidates, &key, &value.to_string());
            }
        }
        Value::Null => {}
    }
}

fn collect_environment_candidate(path: &str, value: &str, candidates: &mut Vec<VariableCandidate>) {
    if let Some(key) = environment_key_from_path(path) {
        push_environment_candidate(candidates, &key, value);
        return;
    }

    if is_environment_sequence_path(path) {
        if let Some((key, value)) = parse_environment_assignment(value) {
            push_environment_candidate(candidates, &key, &value);
        }
    }
}

fn push_environment_candidate(candidates: &mut Vec<VariableCandidate>, key: &str, value: &str) {
    let key = sanitize_variable_key(key, "VALUE");
    if key == "VALUE" {
        return;
    }
    let value = environment_candidate_value(value);
    let kind = environment_candidate_kind(&key, &value);
    push_candidate(candidates, key, &value, kind, "file", "ai");
}

fn environment_key_from_path(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    if !lower.contains("environment.") {
        return None;
    }
    let key = path.rsplit('.').next()?.trim();
    if key.is_empty() || key.contains('[') {
        return None;
    }
    let key = sanitize_variable_key(key, "VALUE");
    (key != "VALUE").then_some(key)
}

fn is_environment_sequence_path(path: &str) -> bool {
    path.to_ascii_lowercase().contains("environment[")
}

fn environment_candidate_value(value: &str) -> String {
    let value = strip_wrapping_quotes(value.trim());
    placeholder_default(&value).unwrap_or(value)
}

fn environment_candidate_kind(key: &str, value: &str) -> &'static str {
    let key_lower = key.to_ascii_lowercase();
    if is_sensitive_path(&key_lower) || is_password_environment_key(key) {
        return "secret";
    }
    if key == "TZ" || key.contains("TIME_ZONE") {
        return "timezone";
    }
    if looks_like_url(value) {
        return "url";
    }
    if looks_like_port_mapping(value) {
        return "port";
    }
    if looks_like_path(value) {
        return "path";
    }
    "text"
}

fn collect_string_candidate(path: &str, value: &str, candidates: &mut Vec<VariableCandidate>) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    let candidate_value = environment_candidate_value(trimmed);

    if let Some((placeholder, default_value)) = single_placeholder_with_default(trimmed) {
        push_candidate(
            candidates,
            placeholder,
            &default_value,
            "placeholder",
            "file",
            "existing",
        );
    } else {
        for placeholder in existing_placeholders(trimmed) {
            push_candidate(
                candidates,
                placeholder.clone(),
                &format!("${{{placeholder}}}"),
                "placeholder",
                "file",
                "existing",
            );
        }
    }

    let path_lower = path.to_ascii_lowercase();
    if is_sensitive_path(&path_lower) {
        push_candidate(
            candidates,
            key_from_path(path, "SECRET"),
            &candidate_value,
            "secret",
            "file",
            "ai",
        );
        return;
    }
    if path_lower.ends_with("image") {
        push_candidate(
            candidates,
            image_key(&candidate_value),
            &candidate_value,
            "image",
            "file",
            "ai",
        );
        return;
    }
    if path_lower.contains("timezone") || path_lower.ends_with(".tz") || path_lower == "tz" {
        push_candidate(
            candidates,
            "TZ".to_string(),
            &candidate_value,
            "timezone",
            "file",
            "ai",
        );
        return;
    }
    if looks_like_url(&candidate_value) {
        push_candidate(
            candidates,
            key_from_path(path, "URL"),
            &candidate_value,
            "url",
            "file",
            "ai",
        );
        return;
    }
    if looks_like_port_mapping(&candidate_value) {
        push_candidate(
            candidates,
            port_key(&candidate_value),
            &candidate_value,
            "port",
            "file",
            "ai",
        );
        return;
    }
    if looks_like_path(&candidate_value) {
        push_candidate(
            candidates,
            path_key(&candidate_value),
            &candidate_value,
            "path",
            "file",
            "ai",
        );
    }
}

fn collect_regex_variable_candidates(content: &str, candidates: &mut Vec<VariableCandidate>) {
    for token in content.split(|character: char| character.is_whitespace() || character == '"') {
        let value = token.trim_matches(|character| {
            matches!(character, '\'' | '"' | ',' | '[' | ']' | '{' | '}')
        });
        if looks_like_url(value) {
            push_candidate(
                candidates,
                key_from_path("url", "URL"),
                value,
                "url",
                "file",
                "ai",
            );
        } else if looks_like_port_mapping(value) {
            push_candidate(candidates, port_key(value), value, "port", "file", "ai");
        } else if looks_like_path(value) {
            push_candidate(candidates, path_key(value), value, "path", "file", "ai");
        }
    }
}

fn merge_variable_candidates(
    first: Vec<VariableCandidate>,
    second: Vec<VariableCandidate>,
) -> Vec<VariableCandidate> {
    let mut merged: HashMap<(String, String), VariableCandidate> = HashMap::new();
    for candidate in first.into_iter().chain(second) {
        let key = (candidate.key.clone(), candidate.value.clone());
        merged
            .entry(key)
            .and_modify(|existing| existing.occurrences += candidate.occurrences.max(1))
            .or_insert(candidate);
    }
    let mut candidates = merged.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.key.cmp(&right.key).then(left.value.cmp(&right.value)));
    candidates
}

fn push_candidate(
    candidates: &mut Vec<VariableCandidate>,
    key: String,
    value: &str,
    kind: &str,
    scope: &str,
    source: &str,
) {
    let value = value.trim();
    if value.is_empty() || value.len() > 240 {
        return;
    }
    candidates.push(VariableCandidate {
        key,
        value: value.to_string(),
        kind: kind.to_string(),
        scope: scope.to_string(),
        source: source.to_string(),
        occurrences: 1,
    });
}

fn existing_placeholders(value: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find('}') else {
            break;
        };
        let placeholder = placeholder_key(after_start[..end].trim());
        if !placeholder.is_empty() {
            placeholders.push(placeholder.to_string());
        }
        rest = &after_start[end + 1..];
    }
    placeholders
}

fn single_placeholder_with_default(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    let inner = value.strip_prefix("${")?.strip_suffix('}')?.trim();
    let placeholder = placeholder_key(inner);
    let default = placeholder_default(value)?;
    (!placeholder.is_empty()).then_some((placeholder, default))
}

fn placeholder_key(value: &str) -> String {
    for operator in [":-", ":=", ":?", "?", "-", "="] {
        if let Some((key, _)) = value.split_once(operator) {
            return key.trim().to_string();
        }
    }
    value.trim().to_string()
}

fn is_sensitive_path(path: &str) -> bool {
    [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "access_key",
        "credential",
    ]
    .iter()
    .any(|keyword| path.contains(keyword))
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("jdbc:")
        || value.starts_with("r2dbc:")
        || value.starts_with("postgres://")
        || value.starts_with("mysql://")
}

fn looks_like_port_mapping(value: &str) -> bool {
    let parts = value.split(':').collect::<Vec<_>>();
    matches!(parts.as_slice(), [left, right] if is_numeric(left) && is_numeric(right))
        || matches!(parts.as_slice(), [_, left, right] if is_numeric(left) && is_numeric(right))
}

fn is_numeric(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn looks_like_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains("/DockerCompose/")
}

fn key_from_path(path: &str, fallback: &str) -> String {
    let key = path
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(fallback);
    sanitize_variable_key(key, fallback)
}

fn image_key(value: &str) -> String {
    let image_name = value
        .split('/')
        .last()
        .unwrap_or(value)
        .split(':')
        .next()
        .unwrap_or(value);
    format!("{}_IMAGE", sanitize_variable_key(image_name, "IMAGE"))
}

fn port_key(value: &str) -> String {
    let port = value
        .split(':')
        .find(|part| is_numeric(part))
        .unwrap_or("PORT");
    format!("PORT_{port}")
}

fn path_key(value: &str) -> String {
    let last = value
        .trim_end_matches('/')
        .split('/')
        .filter(|part| !part.is_empty() && *part != "~" && *part != ".")
        .last()
        .unwrap_or("PATH");
    format!("{}_PATH", sanitize_variable_key(last, "PATH"))
}

fn sanitize_variable_key(value: &str, fallback: &str) -> String {
    let key = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if key.is_empty() {
        fallback.to_string()
    } else if key
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("VALUE_{key}")
    } else {
        key
    }
}

fn map_value<'a>(mapping: &'a serde_yml::Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(&Value::String(key.to_string()))
}

fn collect_list_field(mapping: &serde_yml::Mapping, key: &str, target: &mut Vec<String>) {
    let Some(value) = map_value(mapping, key) else {
        return;
    };

    if let Some(items) = value.as_sequence() {
        for item in items {
            if let Some(value) = value_to_string(item) {
                target.push(value);
            }
        }
        return;
    }

    if let Some(value) = value_to_string(value) {
        target.push(value);
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.trim().to_string()).filter(|value| !value.is_empty()),
        Value::Sequence(items) => {
            let values = items.iter().filter_map(value_to_string).collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(values.join(", "))
            }
        }
        Value::Mapping(mapping) => stringify_mapping(mapping),
        Value::Tagged(tagged) => value_to_string(&tagged.value),
    }
}

fn scalar_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.trim().to_string()).filter(|value| !value.is_empty()),
        Value::Tagged(tagged) => scalar_value_to_string(&tagged.value),
        Value::Sequence(_) | Value::Mapping(_) => None,
    }
}

fn stringify_mapping(mapping: &serde_yml::Mapping) -> Option<String> {
    let published = map_value(mapping, "published")
        .or_else(|| map_value(mapping, "host_ip"))
        .and_then(value_to_string);
    let target = map_value(mapping, "target").and_then(value_to_string);
    if let (Some(published), Some(target)) = (published, target) {
        return Some(format!("{published}:{target}"));
    }

    let values = mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = value_to_string(key)?;
            let value = value_to_string(value)?;
            Some(format!("{key}={value}"))
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

fn unique_sorted(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}

fn metadata_modified_millis(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn short_hash(value: &str) -> String {
    content_hash(value).chars().take(10).collect()
}

fn compose_category(root: &Path, path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.strip_prefix(root).ok())
        .map(path_to_slash_string)
        .filter(|category| !category.is_empty())
        .unwrap_or_else(|| "compose".to_string())
}

fn compose_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(path_to_slash_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "compose".to_string())
        })
}

fn compose_code(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .ok()
        .map(path_to_slash_string)
        .unwrap_or_else(|| path_to_slash_string(path));
    let slug = slugify(&relative);
    format!("dc-{}-{}", slug, short_hash(&relative))
}

fn path_to_slash_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
        "compose".to_string()
    } else {
        slug
    }
}

fn yaml_description(summary: &ComposeSummary, parse_error: Option<&str>) -> String {
    if let Some(parse_error) = parse_error {
        return format!("YAML 解析失败：{}", parse_error);
    }
    if summary.services.is_empty() {
        return "普通 YAML 配置".to_string();
    }

    let images = if summary.images.is_empty() {
        "无镜像".to_string()
    } else {
        summary
            .images
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let ports = if summary.ports.is_empty() {
        "无端口".to_string()
    } else {
        summary
            .ports
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{} 个服务；镜像：{}；端口：{}",
        summary.services.len(),
        images,
        ports
    )
}

pub(crate) fn normalize_kind(kind: &str) -> AppResult<String> {
    let normalized = kind.trim();
    if ASSET_KINDS.contains(&normalized) {
        Ok(normalized.to_string())
    } else {
        Err(AppError::BadRequest("资产类型不合法".to_string()))
    }
}

fn validate_code_name(code: &str, name: &str) -> AppResult<()> {
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(AppError::BadRequest("编码和名称不能为空".to_string()));
    }
    Ok(())
}

pub(crate) fn normalize_status(status: Option<String>) -> AppResult<String> {
    let status = status.unwrap_or_else(|| "enabled".to_string());
    match status.as_str() {
        "enabled" | "disabled" => Ok(status),
        _ => Err(AppError::BadRequest("状态只能是启用或禁用".to_string())),
    }
}

pub(crate) fn normalize_filter_categories(
    category: Option<String>,
    categories: Option<Vec<String>>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    category
        .into_iter()
        .chain(categories.unwrap_or_default())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate<'a>(candidates: &'a [VariableCandidate], key: &str) -> &'a VariableCandidate {
        candidates
            .iter()
            .find(|candidate| candidate.key == key)
            .unwrap_or_else(|| panic!("missing candidate {key}"))
    }

    #[test]
    fn expands_tilde_prefixed_import_paths() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };

        assert_eq!(
            expand_user_path("~/DockerCompose"),
            PathBuf::from(home).join("DockerCompose")
        );
        assert_eq!(
            expand_user_path("/tmp/DockerCompose"),
            PathBuf::from("/tmp/DockerCompose")
        );
    }

    #[test]
    fn derives_safe_deploy_relative_paths() {
        let mut record = AssetItemRecord {
            id: "1".to_string(),
            kind: "docker_compose".to_string(),
            code: "dc-demo".to_string(),
            name: "nextcloud/compose.yml".to_string(),
            category: String::new(),
            description: String::new(),
            content: String::new(),
            tags: vec![],
            source_path: String::new(),
            file_name: String::new(),
            source_mtime: 0,
            source_size: 0,
            content_hash: String::new(),
            last_synced_at: 0,
            service_count: 0,
            services: vec![],
            images: vec![],
            ports: vec![],
            volumes: vec![],
            validation_status: "unknown".to_string(),
            validation_issues: vec![],
            variable_candidates: vec![],
            status: "enabled".to_string(),
            sort_order: 0,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(
            deploy_relative_path(&record).unwrap(),
            PathBuf::from("nextcloud/compose.yml")
        );

        record.name = "nextcloud".to_string();
        assert_eq!(
            deploy_relative_path(&record).unwrap(),
            PathBuf::from("nextcloud/compose.yml")
        );

        record.name = "../compose.yml".to_string();
        assert!(deploy_relative_path(&record).is_err());
    }

    #[test]
    fn extracts_environment_keys_from_mapping_and_sequence() {
        let content = r#"
services:
  db:
    image: postgres:16
    environment:
      TZ: ${TZ:-Asia/Shanghai}
      POSTGRES_PASSWORD: zhou9955
      PUID: 1000
  web:
    image: nginx:latest
    environment:
      - CUSTOM_USER=zjarlin
      - PASSWORD=zhou9955
    volumes:
      - ${APP_DATA:-~/DockerCompose/demo/data}:/data
"#;
        let summary = compose_summary(content).unwrap().unwrap();
        let candidates = extract_variable_candidates(content, &summary);

        let tz = candidate(&candidates, "TZ");
        assert_eq!(tz.value, "Asia/Shanghai");
        assert_eq!(tz.kind, "timezone");

        let password = candidate(&candidates, "POSTGRES_PASSWORD");
        assert_eq!(password.value, "zhou9955");
        assert_eq!(password.kind, "secret");

        let custom_user = candidate(&candidates, "CUSTOM_USER");
        assert_eq!(custom_user.value, "zjarlin");
        assert_eq!(custom_user.kind, "text");

        assert!(candidates.iter().any(|candidate| {
            candidate.key == "DATA_PATH" && candidate.value == "~/DockerCompose/demo/data"
        }));
    }

    #[test]
    fn builds_page_global_variables_from_common_env_and_mounts() {
        let first = r#"
services:
  app:
    image: app:latest
    environment:
      - CUSTOM_USER=admin
      - PASSWORD=zhou9955
      - TZ=Asia/Shanghai
    volumes:
      - ~/DockerCompose/app/data:/data
"#;
        let second = r#"
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_USER: admin
      POSTGRES_PASSWORD: zhou9955
      TIME_ZONE: Asia/Shanghai
    volumes:
      - ~/DockerCompose/db/data:/var/lib/postgresql/data
"#;

        let mut aggregate = PageGlobalVariableAggregate::default();
        collect_page_global_variables(first, &mut aggregate);
        collect_page_global_variables(second, &mut aggregate);
        let candidates = aggregate.finish();

        let username = candidate(&candidates, "USERNAME");
        assert_eq!(username.value, "admin");

        let password = candidate(&candidates, "PASSWORD");
        assert_eq!(password.value, "zhou9955");
        assert_eq!(password.kind, "secret");

        let mount_dir = candidate(&candidates, "MOUNT_DIR");
        assert_eq!(mount_dir.value, "~/DockerCompose");

        assert!(candidates
            .iter()
            .any(|candidate| candidate.key == "DATA_DIR"));
    }
}
