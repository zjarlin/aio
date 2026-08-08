CREATE TABLE sys_user (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    display_name TEXT NOT NULL,
    department TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    last_login_at_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE UNIQUE INDEX sys_user_account_uq ON sys_user (account);
CREATE INDEX sys_user_status_idx ON sys_user (status);

CREATE TABLE sys_user_role (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES sys_user (id) ON DELETE CASCADE,
    role_code TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX sys_user_role_user_id_idx ON sys_user_role (user_id);
CREATE UNIQUE INDEX sys_user_role_user_role_uq ON sys_user_role (user_id, role_code);
