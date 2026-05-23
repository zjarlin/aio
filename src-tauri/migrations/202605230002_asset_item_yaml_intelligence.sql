ALTER TABLE asset_items ADD COLUMN validation_status TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE asset_items ADD COLUMN validation_issues_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE asset_items ADD COLUMN variable_candidates_json TEXT NOT NULL DEFAULT '[]';

CREATE INDEX IF NOT EXISTS idx_asset_items_validation_status
  ON asset_items(validation_status);
