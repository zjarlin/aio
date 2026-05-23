CREATE TABLE IF NOT EXISTS asset_items (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL,
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL DEFAULT '',
  tags_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'enabled',
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (kind, code)
);

CREATE INDEX IF NOT EXISTS idx_asset_items_kind ON asset_items(kind);
CREATE INDEX IF NOT EXISTS idx_asset_items_status ON asset_items(status);
