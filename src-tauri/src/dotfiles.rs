use std::{
    collections::HashSet,
    env, fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use walkdir::WalkDir;

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::{AppError, AppResult},
    rbac::{normalize_page, PageInfo, PageRequest, PageResult},
};

const PREVIEW_LIMIT: usize = 4096;
const HASH_SAMPLE_SIZE: usize = 8192;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerInput {
    pub id: Option<String>,
    pub name: String,
    pub host: Option<String>,
    pub username: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub site: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ComputerRecord {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub site: String,
    pub kind: String,
    pub status: String,
    pub last_scanned_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DotfileEntryRecord {
    pub id: String,
    pub owner_id: String,
    pub code: String,
    pub item_type: String,
    pub local_source: String,
    pub repo_path: String,
    pub deploy_target: String,
    pub condition_expr: String,
    pub sync_mode: String,
    pub adopt_strategy: String,
    pub description: String,
    pub position_marker: String,
    pub tags_json: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DotfileSnapshotRecord {
    pub id: String,
    pub owner_id: String,
    pub computer_id: String,
    pub entry_id: Option<String>,
    pub path: String,
    pub relative_path: String,
    pub item_type: String,
    pub exists_flag: i64,
    pub size: i64,
    pub mtime: i64,
    pub content_hash: String,
    pub preview: String,
    pub status: String,
    pub scanned_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfileSnapshotPageRequest {
    pub o: Option<i64>,
    pub s: Option<i64>,
    pub keyword: Option<String>,
    pub computer_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfileScanResult {
    pub computer_id: String,
    pub scanned: i64,
    pub inserted: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub deleted: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfilesMetadataImportResult {
    pub dotfiles: i64,
    pub env_vars: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DotfileFusionRecord {
    pub deploy_target: String,
    pub description: String,
    pub computer_count: i64,
    pub variant_count: i64,
    pub missing_count: i64,
    pub latest_scanned_at: i64,
}

#[derive(Debug, Deserialize)]
struct ScanLine {
    path: Option<String>,
    relative_path: Option<String>,
    item_type: Option<String>,
    size: Option<i64>,
    mtime: Option<i64>,
    content_hash: Option<String>,
    preview: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
struct SnapshotInput {
    path: String,
    relative_path: String,
    item_type: String,
    size: i64,
    mtime: i64,
    content_hash: String,
    preview: String,
}

#[derive(Debug)]
struct MetadataDotfileRow {
    code: String,
    item_type: String,
    local_source: String,
    repo_path: String,
    deploy_target: String,
    condition_expr: String,
    sync_mode: String,
    adopt_strategy: String,
    description: String,
    position_marker: String,
    enabled: bool,
}

#[derive(Debug)]
struct MetadataEnvRow {
    code: String,
    os: String,
    arch: String,
    define_type: String,
    name: String,
    value: String,
    condition_expr: String,
    description: String,
    enabled: bool,
    file_path: String,
    position_marker: String,
}

pub async fn computer_list(pool: &SqlitePool, token: String) -> AppResult<Vec<ComputerRecord>> {
    let owner_id = require_session(pool, &token).await?.user_id;
    let rows = sqlx::query_as::<_, ComputerRecord>(
        "SELECT * FROM computers
         WHERE owner_id = ?
         ORDER BY kind, name",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn computer_upsert(
    pool: &SqlitePool,
    token: String,
    input: ComputerInput,
) -> AppResult<ComputerRecord> {
    let owner_id = require_session(pool, &token).await?.user_id;
    upsert_computer_for_owner(pool, &owner_id, input).await
}

pub async fn metadata_import(
    pool: &SqlitePool,
    token: String,
) -> AppResult<DotfilesMetadataImportResult> {
    let owner_id = require_session(pool, &token).await?.user_id;
    import_metadata_for_owner(pool, &owner_id).await
}

pub async fn scan_computer(
    pool: &SqlitePool,
    token: String,
    computer_id: Option<String>,
) -> AppResult<DotfileScanResult> {
    let owner_id = require_session(pool, &token).await?.user_id;
    ensure_local_computer(pool, &owner_id).await?;
    import_metadata_for_owner(pool, &owner_id).await?;

    let computer = if let Some(computer_id) = computer_id {
        find_computer_for_owner(pool, &owner_id, &computer_id).await?
    } else {
        find_local_computer(pool, &owner_id).await?
    };

    let mut result = DotfileScanResult {
        computer_id: computer.id.clone(),
        scanned: 0,
        inserted: 0,
        updated: 0,
        unchanged: 0,
        deleted: 0,
        errors: Vec::new(),
    };
    let entries = dotfile_entries_for_owner(pool, &owner_id).await?;
    let snapshots = if computer.kind == "ssh" {
        scan_remote_home(&computer, &entries, &mut result)
    } else {
        scan_local_home(&entries, &mut result)
    };

    persist_snapshots(pool, &owner_id, &computer.id, snapshots, &mut result).await?;
    Ok(result)
}

pub async fn snapshot_page(
    pool: &SqlitePool,
    token: String,
    request: DotfileSnapshotPageRequest,
) -> AppResult<PageResult<DotfileSnapshotRecord>> {
    let owner_id = require_session(pool, &token).await?.user_id;
    let page_request = PageRequest {
        o: request.o,
        s: request.s,
        keyword: request.keyword,
    };
    let (offset, size) = normalize_page(&page_request);
    let keyword = format!("%{}%", page_request.keyword.unwrap_or_default());
    let computer_id = request.computer_id.unwrap_or_default();
    let status = request.status.unwrap_or_default();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM dotfile_snapshots
         WHERE owner_id = ?
           AND (? = '' OR computer_id = ?)
           AND (? = '' OR status = ?)
           AND (
             path LIKE ? OR relative_path LIKE ? OR preview LIKE ? OR content_hash LIKE ?
           )",
    )
    .bind(&owner_id)
    .bind(&computer_id)
    .bind(&computer_id)
    .bind(&status)
    .bind(&status)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .bind(&keyword)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, DotfileSnapshotRecord>(
        "SELECT *
         FROM dotfile_snapshots
         WHERE owner_id = ?
           AND (? = '' OR computer_id = ?)
           AND (? = '' OR status = ?)
           AND (
             path LIKE ? OR relative_path LIKE ? OR preview LIKE ? OR content_hash LIKE ?
           )
         ORDER BY scanned_at DESC, path
         LIMIT ? OFFSET ?",
    )
    .bind(&owner_id)
    .bind(&computer_id)
    .bind(&computer_id)
    .bind(&status)
    .bind(&status)
    .bind(&keyword)
    .bind(&keyword)
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

pub async fn fusion_list(pool: &SqlitePool, token: String) -> AppResult<Vec<DotfileFusionRecord>> {
    let owner_id = require_session(pool, &token).await?.user_id;
    let rows = sqlx::query_as::<_, DotfileFusionRecord>(
        "SELECT
           dotfile_entries.deploy_target,
           dotfile_entries.description,
           COUNT(DISTINCT dotfile_snapshots.computer_id) AS computer_count,
           COUNT(DISTINCT dotfile_snapshots.content_hash) FILTER (
             WHERE dotfile_snapshots.content_hash <> ''
           ) AS variant_count,
           (
             SELECT COUNT(*)
             FROM computers
             WHERE computers.owner_id = dotfile_entries.owner_id
               AND computers.status = 'enabled'
           ) - COUNT(DISTINCT dotfile_snapshots.computer_id) AS missing_count,
           COALESCE(MAX(dotfile_snapshots.scanned_at), 0) AS latest_scanned_at
         FROM dotfile_entries
         LEFT JOIN dotfile_snapshots
           ON dotfile_snapshots.entry_id = dotfile_entries.id
          AND dotfile_snapshots.owner_id = dotfile_entries.owner_id
         WHERE dotfile_entries.owner_id = ?
         GROUP BY dotfile_entries.id
         ORDER BY missing_count DESC, variant_count DESC, dotfile_entries.deploy_target",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn ensure_dotfiles_seed(pool: &SqlitePool) -> AppResult<()> {
    let user_ids = sqlx::query_scalar::<_, String>("SELECT id FROM users")
        .fetch_all(pool)
        .await?;
    for owner_id in user_ids {
        ensure_local_computer(pool, &owner_id).await?;
        let _ = import_metadata_for_owner(pool, &owner_id).await?;
    }
    Ok(())
}

async fn import_metadata_for_owner(
    pool: &SqlitePool,
    owner_id: &str,
) -> AppResult<DotfilesMetadataImportResult> {
    let mut result = DotfilesMetadataImportResult {
        dotfiles: 0,
        env_vars: 0,
        errors: Vec::new(),
    };

    let (dotfiles, envs) = match read_metadata_workbook() {
        Ok(rows) => rows,
        Err(error) => {
            result.errors.push(error);
            return Ok(result);
        }
    };
    let now = now_millis();

    for item in dotfiles {
        sqlx::query(
            "INSERT INTO dotfile_entries
             (id, owner_id, code, item_type, local_source, repo_path, deploy_target,
              condition_expr, sync_mode, adopt_strategy, description, position_marker,
              tags_json, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, ?, ?)
             ON CONFLICT(owner_id, deploy_target) DO UPDATE SET
               code = excluded.code,
               item_type = excluded.item_type,
               local_source = excluded.local_source,
               repo_path = excluded.repo_path,
               condition_expr = excluded.condition_expr,
               sync_mode = excluded.sync_mode,
               adopt_strategy = excluded.adopt_strategy,
               description = excluded.description,
               position_marker = excluded.position_marker,
               status = excluded.status,
               updated_at = excluded.updated_at",
        )
        .bind(new_id())
        .bind(owner_id)
        .bind(item.code)
        .bind(item.item_type)
        .bind(item.local_source)
        .bind(item.repo_path)
        .bind(item.deploy_target)
        .bind(item.condition_expr)
        .bind(item.sync_mode)
        .bind(item.adopt_strategy)
        .bind(item.description)
        .bind(item.position_marker)
        .bind(if item.enabled { "enabled" } else { "disabled" })
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        result.dotfiles += 1;
    }

    for item in envs {
        sqlx::query(
            "INSERT INTO environment_entries
             (id, owner_id, code, os, arch, define_type, name, value, condition_expr,
              description, enabled, file_path, position_marker, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(owner_id, name, os, arch, condition_expr) DO UPDATE SET
               code = excluded.code,
               define_type = excluded.define_type,
               value = excluded.value,
               description = excluded.description,
               enabled = excluded.enabled,
               file_path = excluded.file_path,
               position_marker = excluded.position_marker,
               updated_at = excluded.updated_at",
        )
        .bind(new_id())
        .bind(owner_id)
        .bind(item.code)
        .bind(item.os)
        .bind(item.arch)
        .bind(item.define_type)
        .bind(item.name)
        .bind(item.value)
        .bind(item.condition_expr)
        .bind(item.description)
        .bind(if item.enabled { 1 } else { 0 })
        .bind(item.file_path)
        .bind(item.position_marker)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        result.env_vars += 1;
    }

    Ok(result)
}

async fn upsert_computer_for_owner(
    pool: &SqlitePool,
    owner_id: &str,
    input: ComputerInput,
) -> AppResult<ComputerRecord> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("主机名称不能为空".to_string()));
    }
    let now = now_millis();
    let id = input.id.unwrap_or_else(new_id);
    sqlx::query(
        "INSERT INTO computers
         (id, owner_id, name, host, username, os, arch, site, kind, status,
          last_scanned_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
         ON CONFLICT(owner_id, name) DO UPDATE SET
           host = excluded.host,
           username = excluded.username,
           os = excluded.os,
           arch = excluded.arch,
           site = excluded.site,
           kind = excluded.kind,
           status = excluded.status,
           updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(owner_id)
    .bind(name)
    .bind(input.host.unwrap_or_default())
    .bind(input.username.unwrap_or_default())
    .bind(input.os.unwrap_or_default())
    .bind(input.arch.unwrap_or_default())
    .bind(input.site.unwrap_or_default())
    .bind(input.kind.unwrap_or_else(|| "ssh".to_string()))
    .bind(input.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    find_computer_by_name(pool, owner_id, name).await
}

async fn ensure_local_computer(pool: &SqlitePool, owner_id: &str) -> AppResult<ComputerRecord> {
    let name = hostname();
    upsert_computer_for_owner(
        pool,
        owner_id,
        ComputerInput {
            id: None,
            name,
            host: Some("local".to_string()),
            username: env::var("USER").ok(),
            os: Some(env::consts::OS.to_string()),
            arch: Some(env::consts::ARCH.to_string()),
            site: None,
            kind: Some("local".to_string()),
            status: Some("enabled".to_string()),
        },
    )
    .await
}

async fn find_local_computer(pool: &SqlitePool, owner_id: &str) -> AppResult<ComputerRecord> {
    let row = sqlx::query_as::<_, ComputerRecord>(
        "SELECT * FROM computers
         WHERE owner_id = ? AND kind = 'local'
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(owner_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row)
}

async fn find_computer_for_owner(
    pool: &SqlitePool,
    owner_id: &str,
    computer_id: &str,
) -> AppResult<ComputerRecord> {
    let row = sqlx::query_as::<_, ComputerRecord>(
        "SELECT * FROM computers
         WHERE owner_id = ? AND id = ?
         LIMIT 1",
    )
    .bind(owner_id)
    .bind(computer_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row)
}

async fn find_computer_by_name(
    pool: &SqlitePool,
    owner_id: &str,
    name: &str,
) -> AppResult<ComputerRecord> {
    let row = sqlx::query_as::<_, ComputerRecord>(
        "SELECT * FROM computers
         WHERE owner_id = ? AND name = ?
         LIMIT 1",
    )
    .bind(owner_id)
    .bind(name)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row)
}

async fn dotfile_entries_for_owner(
    pool: &SqlitePool,
    owner_id: &str,
) -> AppResult<Vec<DotfileEntryRecord>> {
    let rows = sqlx::query_as::<_, DotfileEntryRecord>(
        "SELECT * FROM dotfile_entries
         WHERE owner_id = ? AND status = 'enabled'
         ORDER BY deploy_target",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

fn scan_local_home(
    entries: &[DotfileEntryRecord],
    result: &mut DotfileScanResult,
) -> Vec<SnapshotInput> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        result.errors.push("无法读取本机 HOME 环境变量".to_string());
        return Vec::new();
    };

    let mut snapshots = Vec::new();
    for entry in entries {
        let path = expand_home_path(&entry.deploy_target, &home);
        scan_one_path(&path, &home, &entry.item_type, result, &mut snapshots);
    }
    snapshots
}

fn scan_one_path(
    path: &Path,
    home: &Path,
    item_type: &str,
    result: &mut DotfileScanResult,
    snapshots: &mut Vec<SnapshotInput>,
) {
    if !path.exists() {
        return;
    }

    if path.is_file() {
        result.scanned += 1;
        match snapshot_file(path, home, item_type) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(error) => result
                .errors
                .push(format!("扫描 dotfile 失败：{} - {error}", path.display())),
        }
        return;
    }

    if path.is_dir() {
        for entry in WalkDir::new(path)
            .follow_links(false)
            .max_depth(8)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            result.scanned += 1;
            match snapshot_file(entry.path(), home, item_type) {
                Ok(snapshot) => snapshots.push(snapshot),
                Err(error) => result.errors.push(format!(
                    "扫描 dotfile 失败：{} - {error}",
                    entry.path().display()
                )),
            }
        }
    }
}

fn scan_remote_home(
    computer: &ComputerRecord,
    entries: &[DotfileEntryRecord],
    result: &mut DotfileScanResult,
) -> Vec<SnapshotInput> {
    if computer.host.trim().is_empty() {
        result.errors.push("远程主机 host 为空".to_string());
        return Vec::new();
    }
    let target = if computer.username.trim().is_empty() {
        computer.host.clone()
    } else {
        format!("{}@{}", computer.username, computer.host)
    };
    let paths = entries
        .iter()
        .map(|entry| entry.deploy_target.clone())
        .collect::<Vec<_>>();
    let paths_json = match serde_json::to_string(&paths) {
        Ok(input) => input,
        Err(error) => {
            result
                .errors
                .push(format!("序列化远程 dotfiles 路径失败：{error}"));
            return Vec::new();
        }
    };
    let script = r#"
import hashlib
import json
import os

paths = json.loads(__PATHS_JSON__)
home = os.path.expanduser("~")

def emit(path, item_type):
    try:
        stat = os.stat(path)
        with open(path, "rb") as handle:
            data = handle.read()
        preview = data[:4096].decode("utf-8", errors="replace")
        print(json.dumps({
            "path": path,
            "relative_path": os.path.relpath(path, home),
            "item_type": item_type,
            "size": stat.st_size,
            "mtime": int(stat.st_mtime * 1000),
            "content_hash": hashlib.sha256(data).hexdigest(),
            "preview": preview,
        }, ensure_ascii=False))
    except Exception as error:
        print(json.dumps({"path": path, "error": str(error)}, ensure_ascii=False))

for raw in paths:
    path = os.path.expanduser(raw)
    if not os.path.exists(path):
        continue
    if os.path.isfile(path):
        emit(path, "file")
    elif os.path.isdir(path):
        for dirpath, dirnames, filenames in os.walk(path):
            for filename in filenames:
                emit(os.path.join(dirpath, filename), "file")
"#
    .replace("__PATHS_JSON__", &format!("{paths_json:?}"));

    let mut child = match Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=8")
        .arg(target)
        .arg("python3")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            result
                .errors
                .push(format!("启动远程 dotfiles 扫描失败：{error}"));
            return Vec::new();
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        if let Err(error) = stdin.write_all(script.as_bytes()) {
            result
                .errors
                .push(format!("写入远程 dotfiles 扫描脚本失败：{error}"));
            return Vec::new();
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            result
                .errors
                .push(format!("执行远程 dotfiles 扫描失败：{error}"));
            return Vec::new();
        }
    };
    if !output.status.success() {
        result.errors.push(format!(
            "远程 dotfiles 扫描失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
        return Vec::new();
    }

    let mut snapshots = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        result.scanned += 1;
        match serde_json::from_str::<ScanLine>(line) {
            Ok(parsed) => {
                if let Some(error) = parsed.error {
                    result.errors.push(format!(
                        "远程 dotfile 读取失败：{} - {}",
                        parsed.path.unwrap_or_default(),
                        error
                    ));
                    continue;
                }
                if let Some(snapshot) = scan_line_to_snapshot(parsed) {
                    snapshots.push(snapshot);
                }
            }
            Err(error) => result
                .errors
                .push(format!("解析远程 dotfiles 扫描结果失败：{error}")),
        }
    }
    snapshots
}

async fn persist_snapshots(
    pool: &SqlitePool,
    owner_id: &str,
    computer_id: &str,
    snapshots: Vec<SnapshotInput>,
    result: &mut DotfileScanResult,
) -> AppResult<()> {
    let now = now_millis();
    let entries = dotfile_entries_for_owner(pool, owner_id).await?;
    let mut seen = HashSet::new();
    for snapshot in snapshots {
        seen.insert(snapshot.path.clone());
        let entry_id = match_entry_id(&entries, &snapshot.path, &snapshot.relative_path);
        let existing_hash: Option<String> = sqlx::query_scalar(
            "SELECT content_hash
             FROM dotfile_snapshots
             WHERE computer_id = ? AND path = ?",
        )
        .bind(computer_id)
        .bind(&snapshot.path)
        .fetch_optional(pool)
        .await?;
        sqlx::query(
            "INSERT INTO dotfile_snapshots
             (id, owner_id, computer_id, entry_id, path, relative_path, item_type,
              exists_flag, size, mtime, content_hash, preview, status, scanned_at,
              created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, 'tracked', ?, ?, ?)
             ON CONFLICT(computer_id, path) DO UPDATE SET
               entry_id = excluded.entry_id,
               relative_path = excluded.relative_path,
               item_type = excluded.item_type,
               exists_flag = 1,
               size = excluded.size,
               mtime = excluded.mtime,
               content_hash = excluded.content_hash,
               preview = excluded.preview,
               status = 'tracked',
               scanned_at = excluded.scanned_at,
               updated_at = excluded.updated_at",
        )
        .bind(new_id())
        .bind(owner_id)
        .bind(computer_id)
        .bind(entry_id)
        .bind(&snapshot.path)
        .bind(snapshot.relative_path)
        .bind(snapshot.item_type)
        .bind(snapshot.size)
        .bind(snapshot.mtime)
        .bind(&snapshot.content_hash)
        .bind(snapshot.preview)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        match existing_hash {
            None => result.inserted += 1,
            Some(hash) if hash == snapshot.content_hash => result.unchanged += 1,
            Some(_) => result.updated += 1,
        }
    }

    let old_paths = sqlx::query_scalar::<_, String>(
        "SELECT path FROM dotfile_snapshots
         WHERE owner_id = ? AND computer_id = ? AND status = 'tracked'",
    )
    .bind(owner_id)
    .bind(computer_id)
    .fetch_all(pool)
    .await?;
    for path in old_paths {
        if !seen.contains(&path) {
            sqlx::query(
                "UPDATE dotfile_snapshots
                 SET exists_flag = 0, status = 'missing', scanned_at = ?, updated_at = ?
                 WHERE owner_id = ? AND computer_id = ? AND path = ?",
            )
            .bind(now)
            .bind(now)
            .bind(owner_id)
            .bind(computer_id)
            .bind(path)
            .execute(pool)
            .await?;
            result.deleted += 1;
        }
    }

    sqlx::query("UPDATE computers SET last_scanned_at = ?, updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(now)
        .bind(computer_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn snapshot_file(path: &Path, home: &Path, item_type: &str) -> Result<SnapshotInput, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    let preview = read_preview(&mut file)?;
    let content_hash = fingerprint_file(&mut file, metadata.len(), mtime)?;
    Ok(SnapshotInput {
        path: path.to_string_lossy().to_string(),
        relative_path: path
            .strip_prefix(home)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string(),
        item_type: item_type.to_string(),
        size: metadata.len() as i64,
        mtime,
        content_hash,
        preview,
    })
}

fn scan_line_to_snapshot(line: ScanLine) -> Option<SnapshotInput> {
    Some(SnapshotInput {
        path: line.path?,
        relative_path: line.relative_path.unwrap_or_default(),
        item_type: line.item_type.unwrap_or_else(|| "file".to_string()),
        size: line.size.unwrap_or(0),
        mtime: line.mtime.unwrap_or(0),
        content_hash: line.content_hash.unwrap_or_default(),
        preview: line.preview.unwrap_or_default(),
    })
}

fn match_entry_id(entries: &[DotfileEntryRecord], path: &str, relative_path: &str) -> Option<String> {
    entries
        .iter()
        .find(|entry| {
            let deploy = entry.deploy_target.trim_start_matches("~/");
            relative_path == deploy
                || path.ends_with(deploy)
                || relative_path.starts_with(&format!("{deploy}/"))
        })
        .map(|entry| entry.id.clone())
}

fn read_metadata_workbook() -> Result<(Vec<MetadataDotfileRow>, Vec<MetadataEnvRow>), String> {
    let python = env::var("AIO_BUNDLED_PYTHON")
        .unwrap_or_else(|_| "/Users/zjarlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3".to_string());
    let script = r#"
import json
import pandas as pd
from pathlib import Path

path = Path('/Users/zjarlin/aio/Dotfiles/dotfiles_metadata.xlsx')
if not path.exists():
    raise SystemExit('metadata workbook not found: ' + str(path))

def read_sheet(name):
    raw = pd.read_excel(path, sheet_name=name, header=None)
    header_idx = None
    for idx, row in raw.iterrows():
        values = [str(value).strip() for value in row.tolist()]
        if '编号' in values:
            header_idx = idx
            break
    if header_idx is None:
        return []
    headers = [str(value).strip() if str(value) != 'nan' else '' for value in raw.iloc[header_idx].tolist()]
    data = raw.iloc[header_idx + 1:].copy()
    data.columns = headers
    data = data.dropna(how='all')
    return json.loads(data.fillna('').to_json(orient='records', force_ascii=False))

print(json.dumps({
    'dotfiles': read_sheet('Dotfiles部署元数据'),
    'envs': read_sheet('环境变量元数据'),
}, ensure_ascii=False))
"#;
    let output = Command::new(python)
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|error| format!("读取 dotfiles Excel 失败：{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "读取 dotfiles Excel 失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .map_err(|error| format!("解析 dotfiles Excel 失败：{error}"))?;
    let dotfiles = value
        .get("dotfiles")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(parse_dotfile_metadata_row)
        .collect::<Vec<_>>();
    let envs = value
        .get("envs")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(parse_env_metadata_row)
        .collect::<Vec<_>>();
    Ok((dotfiles, envs))
}

fn parse_dotfile_metadata_row(value: serde_json::Value) -> Option<MetadataDotfileRow> {
    Some(MetadataDotfileRow {
        code: json_string(&value, "编号")?,
        item_type: json_string(&value, "类型").unwrap_or_else(|| "file".to_string()),
        local_source: json_string(&value, "本地来源").unwrap_or_default(),
        repo_path: json_string(&value, "仓库路径").unwrap_or_default(),
        deploy_target: json_string(&value, "部署目标")?,
        condition_expr: json_string(&value, "条件").unwrap_or_default(),
        sync_mode: json_string(&value, "同步方式").unwrap_or_else(|| "symlink".to_string()),
        adopt_strategy: json_string(&value, "接管策略").unwrap_or_else(|| "adopt_local".to_string()),
        description: json_string(&value, "说明").unwrap_or_default(),
        position_marker: json_string(&value, "位置标记").unwrap_or_default(),
        enabled: json_boolish(&value, "启用"),
    })
}

fn parse_env_metadata_row(value: serde_json::Value) -> Option<MetadataEnvRow> {
    Some(MetadataEnvRow {
        code: json_string(&value, "编号")?,
        os: json_string(&value, "系统").unwrap_or_default(),
        arch: json_string(&value, "架构").unwrap_or_default(),
        define_type: json_string(&value, "定义类型").unwrap_or_else(|| "export".to_string()),
        name: json_string(&value, "名称")?,
        value: json_string(&value, "值").unwrap_or_default(),
        condition_expr: json_string(&value, "条件").unwrap_or_default(),
        description: json_string(&value, "说明").unwrap_or_default(),
        enabled: json_boolish(&value, "启用"),
        file_path: json_string(&value, "文件路径").unwrap_or_default(),
        position_marker: json_string(&value, "位置标记").unwrap_or_default(),
    })
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::String(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(if *value { "1" } else { "0" }.to_string()),
        _ => None,
    })
}

fn json_boolish(value: &serde_json::Value, key: &str) -> bool {
    match json_string(value, key).as_deref() {
        Some("1") | Some("true") | Some("TRUE") | Some("启用") => true,
        Some("0") | Some("false") | Some("FALSE") | Some("停用") => false,
        _ => true,
    }
}

fn expand_home_path(value: &str, home: &Path) -> PathBuf {
    if value == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(value)
}

fn hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local".to_string())
}

fn read_preview(file: &mut fs::File) -> Result<String, String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut buffer = vec![0_u8; PREVIEW_LIMIT];
    let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
    buffer.truncate(read);
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn fingerprint_file(file: &mut fs::File, size: u64, mtime: i64) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(size.to_le_bytes());
    hasher.update(mtime.to_le_bytes());

    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut head = vec![0_u8; HASH_SAMPLE_SIZE.min(size as usize)];
    let head_read = file.read(&mut head).map_err(|error| error.to_string())?;
    head.truncate(head_read);
    hasher.update(&head);

    if size > head_read as u64 {
        let tail_len = HASH_SAMPLE_SIZE.min(size as usize);
        file.seek(SeekFrom::Start(size.saturating_sub(tail_len as u64)))
            .map_err(|error| error.to_string())?;
        let mut tail = vec![0_u8; tail_len];
        let tail_read = file.read(&mut tail).map_err(|error| error.to_string())?;
        tail.truncate(tail_read);
        hasher.update(&tail);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
