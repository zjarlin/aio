CREATE TABLE sys_role (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    data_scope TEXT NOT NULL CHECK (data_scope IN ('all', 'department', 'self', 'custom')),
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE UNIQUE INDEX sys_role_code_uq ON sys_role (code);
CREATE INDEX sys_role_status_idx ON sys_role (status);

CREATE TABLE sys_role_permission (
    id TEXT PRIMARY KEY,
    role_id TEXT NOT NULL REFERENCES sys_role (id) ON DELETE CASCADE,
    permission_code TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX sys_role_permission_role_id_idx ON sys_role_permission (role_id);
CREATE UNIQUE INDEX sys_role_permission_role_code_uq
    ON sys_role_permission (role_id, permission_code);
