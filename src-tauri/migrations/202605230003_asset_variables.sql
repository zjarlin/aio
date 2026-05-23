CREATE TABLE IF NOT EXISTS asset_variables (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL,
  asset_item_id TEXT,
  category TEXT NOT NULL DEFAULT '',
  key TEXT NOT NULL,
  value TEXT NOT NULL DEFAULT '',
  default_value TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  value_kind TEXT NOT NULL DEFAULT 'text',
  source TEXT NOT NULL DEFAULT 'manual',
  status TEXT NOT NULL DEFAULT 'enabled',
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (asset_item_id) REFERENCES asset_items(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_asset_variables_kind
  ON asset_variables(kind);

CREATE INDEX IF NOT EXISTS idx_asset_variables_asset_item_id
  ON asset_variables(asset_item_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_variables_grid_key
  ON asset_variables(kind, category, key)
  WHERE asset_item_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_variables_file_key
  ON asset_variables(kind, asset_item_id, key)
  WHERE asset_item_id IS NOT NULL;
