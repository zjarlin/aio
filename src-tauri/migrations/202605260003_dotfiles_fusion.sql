CREATE TABLE IF NOT EXISTS computers (
  id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  name TEXT NOT NULL,
  host TEXT NOT NULL DEFAULT '',
  username TEXT NOT NULL DEFAULT '',
  os TEXT NOT NULL DEFAULT '',
  arch TEXT NOT NULL DEFAULT '',
  site TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT 'local',
  status TEXT NOT NULL DEFAULT 'enabled',
  last_scanned_at INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE,
  UNIQUE (owner_id, name)
);

CREATE TABLE IF NOT EXISTS dotfile_entries (
  id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  code TEXT NOT NULL,
  item_type TEXT NOT NULL DEFAULT 'file',
  local_source TEXT NOT NULL DEFAULT '',
  repo_path TEXT NOT NULL DEFAULT '',
  deploy_target TEXT NOT NULL DEFAULT '',
  condition_expr TEXT NOT NULL DEFAULT '',
  sync_mode TEXT NOT NULL DEFAULT 'symlink',
  adopt_strategy TEXT NOT NULL DEFAULT 'adopt_local',
  description TEXT NOT NULL DEFAULT '',
  position_marker TEXT NOT NULL DEFAULT '',
  tags_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'enabled',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE,
  UNIQUE (owner_id, deploy_target)
);

CREATE TABLE IF NOT EXISTS dotfile_snapshots (
  id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  computer_id TEXT NOT NULL,
  entry_id TEXT,
  path TEXT NOT NULL,
  relative_path TEXT NOT NULL DEFAULT '',
  item_type TEXT NOT NULL DEFAULT 'file',
  exists_flag INTEGER NOT NULL DEFAULT 1,
  size INTEGER NOT NULL DEFAULT 0,
  mtime INTEGER NOT NULL DEFAULT 0,
  content_hash TEXT NOT NULL DEFAULT '',
  preview TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'tracked',
  scanned_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE,
  FOREIGN KEY (computer_id) REFERENCES computers(id) ON DELETE CASCADE,
  FOREIGN KEY (entry_id) REFERENCES dotfile_entries(id) ON DELETE SET NULL,
  UNIQUE (computer_id, path)
);

CREATE TABLE IF NOT EXISTS environment_entries (
  id TEXT PRIMARY KEY NOT NULL,
  owner_id TEXT NOT NULL,
  code TEXT NOT NULL,
  os TEXT NOT NULL DEFAULT '',
  arch TEXT NOT NULL DEFAULT '',
  define_type TEXT NOT NULL DEFAULT 'export',
  name TEXT NOT NULL,
  value TEXT NOT NULL DEFAULT '',
  condition_expr TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1,
  file_path TEXT NOT NULL DEFAULT '',
  position_marker TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE,
  UNIQUE (owner_id, name, os, arch, condition_expr)
);

CREATE INDEX IF NOT EXISTS idx_computers_owner_id ON computers(owner_id);
CREATE INDEX IF NOT EXISTS idx_dotfile_entries_owner_id ON dotfile_entries(owner_id);
CREATE INDEX IF NOT EXISTS idx_dotfile_snapshots_owner_computer ON dotfile_snapshots(owner_id, computer_id);
CREATE INDEX IF NOT EXISTS idx_dotfile_snapshots_hash ON dotfile_snapshots(content_hash);
CREATE INDEX IF NOT EXISTS idx_environment_entries_owner_id ON environment_entries(owner_id);
