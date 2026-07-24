CREATE TABLE engine_operation_definitions (
    id TEXT PRIMARY KEY,
    operation_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    method TEXT NOT NULL CHECK (method IN ('GET', 'POST')),
    state TEXT NOT NULL CHECK (state IN ('draft', 'published', 'disabled')),
    active_revision_id TEXT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX engine_operation_definitions_key_uidx
    ON engine_operation_definitions (operation_key);

CREATE TABLE engine_operation_revisions (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES engine_operation_definitions (id),
    revision INTEGER NOT NULL,
    executor_kind TEXT NOT NULL CHECK (executor_kind = 'rhai'),
    source_text TEXT NOT NULL,
    input_schema TEXT NOT NULL,
    output_schema TEXT NOT NULL,
    capability_policy TEXT NOT NULL,
    timeout_ms BIGINT NOT NULL CHECK (timeout_ms BETWEEN 1 AND 30000),
    generated_by_model TEXT NULL,
    created_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_operation_revisions_operation_id_idx
    ON engine_operation_revisions (operation_id);
CREATE UNIQUE INDEX engine_operation_revisions_number_uidx
    ON engine_operation_revisions (operation_id, revision);

ALTER TABLE engine_operation_definitions
    ADD CONSTRAINT engine_operation_definitions_active_revision_fk
    FOREIGN KEY (active_revision_id) REFERENCES engine_operation_revisions (id);

CREATE TABLE engine_operation_runs (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES engine_operation_definitions (id),
    revision_id TEXT NOT NULL REFERENCES engine_operation_revisions (id),
    request_context TEXT NOT NULL,
    response TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('succeeded', 'failed')),
    diagnostics TEXT NULL,
    duration_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_operation_runs_operation_id_idx
    ON engine_operation_runs (operation_id);
CREATE INDEX engine_operation_runs_revision_id_idx
    ON engine_operation_runs (revision_id);
