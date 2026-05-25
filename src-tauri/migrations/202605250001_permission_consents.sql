CREATE TABLE IF NOT EXISTS permission_consents (
  id TEXT PRIMARY KEY NOT NULL,
  user_id TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  capability TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'granted',
  reason TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (user_id, source_id, capability, scope),
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_permission_consents_user_source
  ON permission_consents(user_id, source_id);

CREATE INDEX IF NOT EXISTS idx_permission_consents_capability
  ON permission_consents(capability);
