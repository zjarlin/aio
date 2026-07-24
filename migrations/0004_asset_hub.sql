CREATE TABLE biz_asset_hub_asset_records (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    source TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_asset_hub_asset_records_kind_idx
    ON biz_asset_hub_asset_records (kind);
