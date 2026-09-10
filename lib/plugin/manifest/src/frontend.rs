use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, ensure};

use crate::{PageBody, PageDefinition, RepositoryManifest};

pub const MAX_FRONTEND_FILES: usize = 256;

pub fn validate_frontend_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && path.len() <= 240
            && path.split('/').all(|segment| {
                !segment.is_empty()
                    && segment != "."
                    && segment != ".."
                    && !segment.starts_with('.')
                    && !segment.ends_with('.')
                    && !is_reserved_segment(segment)
                    && segment
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            }),
        "前端资产必须使用规范化的相对路径: {path}"
    );
    Ok(())
}

fn is_reserved_segment(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|number| number.len() == 1 && matches!(number.as_bytes()[0], b'1'..=b'9'))
}

pub fn frontend_files(
    root: &Path,
    manifest: &RepositoryManifest,
) -> Result<BTreeMap<String, PathBuf>> {
    let Some(frontend) = &manifest.plugin.frontend else {
        return Ok(BTreeMap::new());
    };
    validate_frontend_path(&frontend.path)?;
    let root = root.canonicalize().context("解析插件仓库路径失败")?;
    let directory = root.join(&frontend.path);
    ensure!(directory.is_dir(), "前端产物目录不存在: {}", frontend.path);
    let mut cursor = root.clone();
    for part in frontend.path.split('/') {
        cursor.push(part);
        ensure!(
            !fs::symlink_metadata(&cursor)?.file_type().is_symlink(),
            "前端产物路径不能包含符号链接"
        );
    }
    let mut files = BTreeMap::new();
    collect_files(&directory, "", &mut files)?;
    ensure!(!files.is_empty(), "前端产物目录不能为空");
    Ok(files)
}

fn collect_files(
    directory: &Path,
    prefix: &str,
    files: &mut BTreeMap<String, PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory).context("读取前端产物目录失败")? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("前端资产路径必须是 UTF-8"))?;
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        validate_frontend_path(&relative)?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_files(&entry.path(), &relative, files)?;
        } else {
            ensure!(
                kind.is_file(),
                "前端产物不能包含符号链接或特殊文件: {relative}"
            );
            ensure!(
                files.len() < MAX_FRONTEND_FILES,
                "前端资产数量超过 {MAX_FRONTEND_FILES}"
            );
            files.insert(relative, entry.path());
        }
    }
    Ok(())
}

pub fn validate_frontend_pages<'a>(
    manifest: &RepositoryManifest,
    pages: &[PageDefinition],
    files: impl IntoIterator<Item = &'a str>,
) -> Result<()> {
    let files = files.into_iter().collect::<std::collections::BTreeSet<_>>();
    for page in pages {
        if let PageBody::Frontend { entry } = &page.body {
            ensure!(
                manifest.plugin.frontend.is_some(),
                "前端页面必须声明 plugin.frontend"
            );
            validate_frontend_path(entry)?;
            ensure!(entry.ends_with(".html"), "前端页面入口必须是 HTML 文档");
            ensure!(
                files.contains(entry.as_str()),
                "前端页面入口未包含在包中: {entry}"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "frontend_tests.rs"]
mod tests;
