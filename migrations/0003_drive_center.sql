CREATE TABLE biz_drive_center_drive_tasks (
    id TEXT PRIMARY KEY,
    drive_path TEXT NOT NULL,
    action TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_drive_center_drive_tasks_drive_path_idx
    ON biz_drive_center_drive_tasks (drive_path);
