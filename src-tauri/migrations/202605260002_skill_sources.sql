ALTER TABLE skills ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE skills ADD COLUMN last_synced_at INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS skill_sources (
  id TEXT PRIMARY KEY NOT NULL,
  skill_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  host TEXT NOT NULL,
  root TEXT NOT NULL DEFAULT '',
  path TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE,
  UNIQUE (kind, host, path)
);

CREATE INDEX IF NOT EXISTS idx_skill_sources_skill_id ON skill_sources(skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_sources_host_path ON skill_sources(host, path);
CREATE INDEX IF NOT EXISTS idx_skills_content_hash ON skills(content_hash);
