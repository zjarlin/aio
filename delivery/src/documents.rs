use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Result, ensure};
use az_plugin_delivery::{DocumentImage, Documentation};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pulldown_cmark::{Event, Parser, Tag};

pub fn collect(root: &Path) -> Result<Documentation> {
    let root = root.canonicalize()?;
    let path = root.join("README.md");
    if !path.exists() {
        return Ok(Documentation::default());
    }
    ensure!(
        path.canonicalize()?.starts_with(&root),
        "README 必须位于仓库内"
    );
    ensure!(
        fs::metadata(&path)?.len() <= 512 * 1024,
        "README 超过 512 KiB"
    );
    let readme = fs::read_to_string(path)?;
    let mut images = BTreeMap::new();
    let mut remaining = 10 * 1024 * 1024;
    for event in Parser::new(&readme) {
        let Event::Start(Tag::Image { dest_url, .. }) = event else {
            continue;
        };
        let dest = dest_url
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .trim_start_matches("./");
        if dest.starts_with('/') || dest.contains(':') || dest.contains('\\') {
            continue;
        }
        let candidate = root.join(dest);
        if images.contains_key(dest) {
            continue;
        }
        if !candidate.is_file() {
            continue;
        }
        ensure!(
            candidate.canonicalize()?.starts_with(&root),
            "README 图片越界"
        );
        let content_type = match candidate.extension().and_then(|s| s.to_str()) {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => continue,
        };
        let length = usize::try_from(fs::metadata(&candidate)?.len())?;
        ensure!(
            images.len() < 64 && length <= remaining,
            "README 图片超过限额"
        );
        remaining -= length;
        images.insert(
            dest.to_owned(),
            DocumentImage {
                content_base64: STANDARD.encode(fs::read(candidate)?),
                content_type: content_type.into(),
            },
        );
    }
    Ok(Documentation { readme, images })
}
