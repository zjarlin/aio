CREATE TABLE IF NOT EXISTS permission_approval_requests (
  id TEXT PRIMARY KEY NOT NULL,
  user_id TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  capability TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT '*',
  target TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'pending',
  reason TEXT NOT NULL DEFAULT '',
  decision_reason TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  decided_at INTEGER,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_permission_approval_requests_user_status
  ON permission_approval_requests(user_id, status, updated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_permission_approval_requests_pending
  ON permission_approval_requests(user_id, source_id, capability, scope, target)
  WHERE status = 'pending';
