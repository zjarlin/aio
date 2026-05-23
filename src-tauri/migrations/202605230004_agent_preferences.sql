CREATE TABLE IF NOT EXISTS agent_preferences (
  id TEXT PRIMARY KEY NOT NULL,
  code TEXT NOT NULL UNIQUE,
  section TEXT NOT NULL,
  domain TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  rationale TEXT NOT NULL DEFAULT '',
  tags_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'enabled',
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_preferences_section ON agent_preferences(section);
CREATE INDEX IF NOT EXISTS idx_agent_preferences_domain ON agent_preferences(domain);
CREATE INDEX IF NOT EXISTS idx_agent_preferences_status ON agent_preferences(status);
