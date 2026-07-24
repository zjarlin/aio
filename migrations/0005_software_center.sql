CREATE TABLE biz_software_center_software_package_records (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_path TEXT NOT NULL,
    platform TEXT NOT NULL,
    arch TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_software_center_software_package_records_name_idx
    ON biz_software_center_software_package_records (name);
