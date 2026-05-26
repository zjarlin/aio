use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    auth::require_session,
    db::{new_id, now_millis},
    error::AppResult,
};

const CHUNK_CHAR_LIMIT: usize = 700;
const CHUNK_TARGET_COUNT: usize = 6;
const RETRIEVAL_LIMIT: i64 = 8;
const RETRIEVAL_CANDIDATE_LIMIT: i64 = 32;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocumentRecord {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub owner_id: String,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub content_hash: String,
    pub is_public: bool,
    pub is_archived: bool,
    pub source_updated_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeChunkRecord {
    pub id: String,
    pub document_id: String,
    pub ordinal: i64,
    pub content: String,
    pub searchable_text: String,
    pub char_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResult {
    pub document: KnowledgeDocumentRecord,
    pub chunk: KnowledgeChunkRecord,
    pub score: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchRequest {
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RetrievedKnowledgeChunk {
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub content: String,
    pub score: i64,
    pub is_public: bool,
}

#[derive(Debug, FromRow)]
struct KnowledgeSearchRow {
    document_id: String,
    source_type: String,
    source_id: String,
    owner_id: String,
    title: String,
    category: String,
    tags_json: String,
    content_hash: String,
    is_public: i64,
    is_archived: i64,
    source_updated_at: i64,
    document_created_at: i64,
    document_updated_at: i64,
    chunk_id: String,
    ordinal: i64,
    content: String,
    searchable_text: String,
    char_count: i64,
    chunk_created_at: i64,
    chunk_updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct NoteKnowledgeInput {
    pub note_id: String,
    pub owner_id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub content_hash: String,
    pub is_public: bool,
    pub is_archived: bool,
    pub updated_at: i64,
}

pub async fn sync_note(pool: &SqlitePool, input: NoteKnowledgeInput) -> AppResult<()> {
    let now = now_millis();
    let document_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM knowledge_documents WHERE source_type = 'note' AND source_id = ? LIMIT 1",
    )
    .bind(&input.note_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(new_id);

    let tags_json = serde_json::to_string(&normalize_tags(&input.tags))?;
    let tags_text = normalize_tags_text(&input.tags);

    sqlx::query(
        "INSERT INTO knowledge_documents
         (id, source_type, source_id, owner_id, title, category, tags_json, tags_text, content_hash, is_public, is_archived, source_updated_at, created_at, updated_at)
         VALUES (?, 'note', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(source_type, source_id) DO UPDATE SET
           owner_id = excluded.owner_id,
           title = excluded.title,
           category = excluded.category,
           tags_json = excluded.tags_json,
           tags_text = excluded.tags_text,
           content_hash = excluded.content_hash,
           is_public = excluded.is_public,
           is_archived = excluded.is_archived,
           source_updated_at = excluded.source_updated_at,
           updated_at = excluded.updated_at",
    )
    .bind(&document_id)
    .bind(&input.note_id)
    .bind(&input.owner_id)
    .bind(input.title.trim())
    .bind(input.category.trim())
    .bind(tags_json)
    .bind(tags_text)
    .bind(input.content_hash.trim())
    .bind(input.is_public as i64)
    .bind(input.is_archived as i64)
    .bind(input.updated_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    replace_chunks(pool, &document_id, &input).await?;
    Ok(())
}

pub async fn delete_note(pool: &SqlitePool, note_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM knowledge_documents WHERE source_type = 'note' AND source_id = ?")
        .bind(note_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn rebuild_notes(pool: &SqlitePool) -> AppResult<()> {
    let rows = sqlx::query_as::<_, NoteSyncRow>(
        "SELECT
           notes.id,
           notes.owner_id,
           notes.title,
           notes.content,
           notes.category,
           notes.content_hash,
           notes.is_public,
           notes.is_archived,
           notes.updated_at
         FROM notes
         ORDER BY notes.updated_at DESC, notes.id DESC",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let tags =
            sqlx::query_scalar::<_, String>("SELECT tag FROM note_tags WHERE note_id = ? ORDER BY tag")
                .bind(&row.id)
                .fetch_all(pool)
                .await?;
        sync_note(
            pool,
            NoteKnowledgeInput {
                note_id: row.id,
                owner_id: row.owner_id,
                title: row.title,
                content: row.content,
                category: row.category,
                tags,
                content_hash: row.content_hash,
                is_public: row.is_public == 1,
                is_archived: row.is_archived == 1,
                updated_at: row.updated_at,
            },
        )
        .await?;
    }

    Ok(())
}

pub async fn search(
    pool: &SqlitePool,
    token: String,
    request: KnowledgeSearchRequest,
) -> AppResult<Vec<KnowledgeSearchResult>> {
    let session = require_session(pool, &token).await?;
    let results = search_visible_chunks(pool, &session.user_id, &request.query, request.limit).await?;
    Ok(results
        .into_iter()
        .map(|item| KnowledgeSearchResult {
            document: item.document,
            chunk: item.chunk,
            score: item.score,
        })
        .collect())
}

pub async fn retrieve_for_user(
    pool: &SqlitePool,
    user_id: &str,
    query: &str,
    limit: Option<i64>,
) -> AppResult<Vec<RetrievedKnowledgeChunk>> {
    let results = search_visible_chunks(pool, user_id, query, limit).await?;
    Ok(results
        .into_iter()
        .map(|item| RetrievedKnowledgeChunk {
            title: item.document.title,
            category: item.document.category,
            tags: item.document.tags,
            content: item.chunk.content,
            score: item.score,
            is_public: item.document.is_public,
        })
        .collect())
}

pub fn format_retrieved_chunks(chunks: &[RetrievedKnowledgeChunk]) -> Option<String> {
    if chunks.is_empty() {
        return None;
    }

    let mut lines = vec!["Knowledge base excerpts:".to_string()];
    for (index, chunk) in chunks.iter().enumerate() {
        let category = if chunk.category.is_empty() {
            "未分类".to_string()
        } else {
            chunk.category.clone()
        };
        let tags = if chunk.tags.is_empty() {
            String::new()
        } else {
            format!(" | tags: {}", chunk.tags.join(", "))
        };
        let visibility = if chunk.is_public { "公开" } else { "私有" };
        lines.push(format!(
            "[{}] {} | 分类: {} | 可见性: {} | 相关度: {}{}",
            index + 1,
            chunk.title,
            category,
            visibility,
            chunk.score,
            tags
        ));
        lines.push(chunk.content.trim().to_string());
    }
    Some(lines.join("\n"))
}

async fn replace_chunks(
    pool: &SqlitePool,
    document_id: &str,
    input: &NoteKnowledgeInput,
) -> AppResult<()> {
    let chunks = build_chunks(&input.title, &input.content, &input.category, &input.tags);
    let now = now_millis();
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM knowledge_chunks WHERE document_id = ?")
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

    for (ordinal, chunk) in chunks.into_iter().enumerate() {
        let searchable_text = build_searchable_text(&input.title, &input.category, &input.tags, &chunk);
        sqlx::query(
            "INSERT INTO knowledge_chunks
             (id, document_id, ordinal, content, searchable_text, char_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(document_id)
        .bind(ordinal as i64)
        .bind(&chunk)
        .bind(searchable_text)
        .bind(chunk.chars().count() as i64)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn search_visible_chunks(
    pool: &SqlitePool,
    user_id: &str,
    query: &str,
    limit: Option<i64>,
) -> AppResult<Vec<ScoredKnowledgeRow>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let limit = limit.unwrap_or(RETRIEVAL_LIMIT).clamp(1, 20);
    let tokens = tokenize_query(trimmed);
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let broad = format!("%{}%", trimmed);
    let rows = sqlx::query_as::<_, KnowledgeSearchRow>(
        "SELECT
           documents.id AS document_id,
           documents.source_type,
           documents.source_id,
           documents.owner_id,
           documents.title,
           documents.category,
           documents.tags_json,
           documents.content_hash,
           documents.is_public,
           documents.is_archived,
           documents.source_updated_at,
           documents.created_at AS document_created_at,
           documents.updated_at AS document_updated_at,
           chunks.id AS chunk_id,
           chunks.ordinal,
           chunks.content,
           chunks.searchable_text,
           chunks.char_count,
           chunks.created_at AS chunk_created_at,
           chunks.updated_at AS chunk_updated_at
         FROM knowledge_documents documents
         JOIN knowledge_chunks chunks ON chunks.document_id = documents.id
         WHERE documents.is_archived = 0
           AND (documents.owner_id = ? OR documents.is_public = 1)
           AND (
             documents.title LIKE ?
             OR documents.category LIKE ?
             OR documents.tags_text LIKE ?
             OR chunks.searchable_text LIKE ?
           )
         ORDER BY documents.updated_at DESC, chunks.ordinal ASC
         LIMIT ?",
    )
    .bind(user_id)
    .bind(&broad)
    .bind(&broad)
    .bind(&broad)
    .bind(&broad)
    .bind(RETRIEVAL_CANDIDATE_LIMIT)
    .fetch_all(pool)
    .await?;

    let mut scored = Vec::new();
    for row in rows {
        let score = score_row(&row, trimmed, &tokens);
        if score <= 0 {
            continue;
        }
        scored.push(ScoredKnowledgeRow {
            document: to_document_record(&row)?,
            chunk: KnowledgeChunkRecord {
                id: row.chunk_id,
                document_id: row.document_id.clone(),
                ordinal: row.ordinal,
                content: row.content,
                searchable_text: row.searchable_text,
                char_count: row.char_count,
                created_at: row.chunk_created_at,
                updated_at: row.chunk_updated_at,
            },
            score,
        });
    }

    scored.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.document.updated_at.cmp(&left.document.updated_at))
            .then_with(|| left.chunk.ordinal.cmp(&right.chunk.ordinal))
    });
    scored.truncate(limit as usize);
    Ok(scored)
}

fn to_document_record(row: &KnowledgeSearchRow) -> AppResult<KnowledgeDocumentRecord> {
    Ok(KnowledgeDocumentRecord {
        id: row.document_id.clone(),
        source_type: row.source_type.clone(),
        source_id: row.source_id.clone(),
        owner_id: row.owner_id.clone(),
        title: row.title.clone(),
        category: row.category.clone(),
        tags: deserialize_tags(&row.tags_json)?,
        content_hash: row.content_hash.clone(),
        is_public: row.is_public == 1,
        is_archived: row.is_archived == 1,
        source_updated_at: row.source_updated_at,
        created_at: row.document_created_at,
        updated_at: row.document_updated_at,
    })
}

fn score_row(row: &KnowledgeSearchRow, query: &str, tokens: &[String]) -> i64 {
    let title = row.title.to_lowercase();
    let category = row.category.to_lowercase();
    let tags = row.tags_json.to_lowercase();
    let searchable = row.searchable_text.to_lowercase();
    let query_lower = query.to_lowercase();

    let mut score = 0;
    if title.contains(&query_lower) {
        score += 12;
    }
    if category.contains(&query_lower) {
        score += 8;
    }
    if tags.contains(&query_lower) {
        score += 6;
    }
    if searchable.contains(&query_lower) {
        score += 10;
    }

    for token in tokens {
        if title.contains(token) {
            score += 5;
        }
        if category.contains(token) {
            score += 3;
        }
        if tags.contains(token) {
            score += 2;
        }
        let token_hits = searchable.matches(token).count() as i64;
        score += token_hits.min(8);
    }

    score
}

fn build_chunks(title: &str, content: &str, category: &str, tags: &[String]) -> Vec<String> {
    let normalized = normalize_multiline(content);
    let mut chunks = Vec::new();

    let mut header = vec![title.trim().to_string()];
    if !category.trim().is_empty() {
        header.push(format!("分类: {}", category.trim()));
    }
    if !tags.is_empty() {
        header.push(format!("标签: {}", normalize_tags(tags).join(", ")));
    }

    let body_segments = split_text_blocks(&normalized);
    let mut current = header.join("\n");

    for segment in body_segments {
        if current.chars().count() + 2 + segment.chars().count() <= CHUNK_CHAR_LIMIT {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(&segment);
            continue;
        }

        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }

        if segment.chars().count() <= CHUNK_CHAR_LIMIT {
            current = segment;
            continue;
        }

        for piece in split_long_segment(&segment) {
            if chunks.len() + 1 >= CHUNK_TARGET_COUNT {
                current = piece;
                break;
            }
            chunks.push(piece);
        }
        if current.is_empty() {
            current = String::new();
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    if chunks.is_empty() {
        chunks.push(header.join("\n"));
    }

    chunks.truncate(CHUNK_TARGET_COUNT);
    chunks
}

fn split_text_blocks(text: &str) -> Vec<String> {
    let blocks = text
        .split("\n\n")
        .map(normalize_inline)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if blocks.is_empty() {
        let single = normalize_inline(text);
        if single.is_empty() {
            Vec::new()
        } else {
            vec![single]
        }
    } else {
        blocks
    }
}

fn split_long_segment(segment: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for sentence in split_sentences(segment) {
        if current.chars().count() + sentence.chars().count() + 1 <= CHUNK_CHAR_LIMIT {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&sentence);
            continue;
        }
        if !current.is_empty() {
            parts.push(current.trim().to_string());
        }
        if sentence.chars().count() <= CHUNK_CHAR_LIMIT {
            current = sentence;
            continue;
        }
        let hard_parts = hard_split(&sentence, CHUNK_CHAR_LIMIT);
        for piece in hard_parts.into_iter().take(CHUNK_TARGET_COUNT) {
            parts.push(piece);
        }
        current = String::new();
    }
    if !current.is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '\n' | '。' | '！' | '？' | '.' | '!' | '?') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }
    parts
}

fn hard_split(text: &str, limit: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= limit {
            parts.push(current.trim().to_string());
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn build_searchable_text(title: &str, category: &str, tags: &[String], chunk: &str) -> String {
    [
        title.trim(),
        category.trim(),
        &normalize_tags_text(tags),
        chunk.trim(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
    .to_lowercase()
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '，' | ';' | '；' | '|' | '/' | '\\'))
        .map(str::trim)
        .filter(|token| token.chars().count() >= 2)
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_tags_text(tags: &[String]) -> String {
    normalize_tags(tags).join(" ")
}

fn deserialize_tags(raw: &str) -> AppResult<Vec<String>> {
    Ok(serde_json::from_str(raw).unwrap_or_default())
}

fn normalize_inline(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_multiline(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

struct ScoredKnowledgeRow {
    document: KnowledgeDocumentRecord,
    chunk: KnowledgeChunkRecord,
    score: i64,
}

#[derive(Debug, FromRow)]
struct NoteSyncRow {
    id: String,
    owner_id: String,
    title: String,
    content: String,
    category: String,
    content_hash: String,
    is_public: i64,
    is_archived: i64,
    updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::{build_chunks, format_retrieved_chunks, tokenize_query, RetrievedKnowledgeChunk};

    #[test]
    fn chunks_split_long_text() {
        let chunks = build_chunks(
            "AIO",
            &"第一段内容。".repeat(120),
            "产品",
            &["ai".to_string(), "知识库".to_string()],
        );
        assert!(!chunks.is_empty());
        assert!(chunks.len() <= 6);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 750));
    }

    #[test]
    fn query_tokenization_filters_short_noise() {
        let tokens = tokenize_query("AIO 知识库, 检索 / notes");
        assert!(tokens.contains(&"aio".to_string()));
        assert!(tokens.contains(&"知识库".to_string()));
        assert!(tokens.contains(&"notes".to_string()));
    }

    #[test]
    fn retrieved_chunk_formatter_emits_text() {
        let text = format_retrieved_chunks(&[RetrievedKnowledgeChunk {
            title: "测试".to_string(),
            category: "产品".to_string(),
            tags: vec!["ai".to_string()],
            content: "知识片段".to_string(),
            score: 9,
            is_public: false,
        }])
        .unwrap_or_default();
        assert!(text.contains("Knowledge base excerpts"));
        assert!(text.contains("知识片段"));
    }
}
