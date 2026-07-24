CREATE TABLE biz_config_center_config_entries (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_config_center_config_entries_namespace_idx
    ON biz_config_center_config_entries (namespace);
CREATE INDEX biz_config_center_config_entries_key_idx
    ON biz_config_center_config_entries (key);
