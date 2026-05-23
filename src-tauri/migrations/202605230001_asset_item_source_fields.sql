ALTER TABLE asset_items ADD COLUMN source_path TEXT NOT NULL DEFAULT '';
ALTER TABLE asset_items ADD COLUMN file_name TEXT NOT NULL DEFAULT '';
ALTER TABLE asset_items ADD COLUMN source_mtime INTEGER NOT NULL DEFAULT 0;
ALTER TABLE asset_items ADD COLUMN source_size INTEGER NOT NULL DEFAULT 0;
ALTER TABLE asset_items ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE asset_items ADD COLUMN last_synced_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE asset_items ADD COLUMN service_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE asset_items ADD COLUMN services_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE asset_items ADD COLUMN images_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE asset_items ADD COLUMN ports_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE asset_items ADD COLUMN volumes_json TEXT NOT NULL DEFAULT '[]';

CREATE INDEX IF NOT EXISTS idx_asset_items_source_path ON asset_items(source_path);
CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_items_kind_source_path
  ON asset_items(kind, source_path)
  WHERE source_path <> '';
