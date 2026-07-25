ALTER TABLE engine_meta_fields
    ADD COLUMN domain_metadata_json TEXT NULL,
    ADD COLUMN validation_json TEXT NULL;

CREATE TABLE nature_projects (
    id TEXT PRIMARY KEY,
    native_name TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE TABLE nature_revisions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES nature_projects (id),
    source_text TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'checking', 'succeeded', 'failed', 'published')),
    blueprint_json TEXT NOT NULL,
    inference_decisions_json TEXT NOT NULL,
    defaults_json TEXT NOT NULL,
    diagnostics_json TEXT NOT NULL,
    breaking_changes_json TEXT NOT NULL,
    generated_files_json TEXT NOT NULL,
    artifact_hash TEXT NOT NULL,
    error_message TEXT NOT NULL,
    published_at_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX nature_revisions_project_id_idx ON nature_revisions (project_id);

CREATE TABLE nature_generation_runs (
    id TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL REFERENCES nature_revisions (id),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    stage TEXT NOT NULL,
    artifact_hash TEXT NOT NULL,
    error_message TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT NOT NULL
);
CREATE INDEX nature_generation_runs_revision_id_idx ON nature_generation_runs (revision_id);

CREATE TABLE engine_field_bindings (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES nature_projects (id),
    owner_model_code TEXT NOT NULL,
    field_code TEXT NOT NULL,
    source_name TEXT NOT NULL,
    transform_json TEXT NOT NULL,
    domain_metadata_json TEXT NOT NULL,
    validation_json TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_field_bindings_project_id_idx
    ON engine_field_bindings (project_id);
CREATE INDEX engine_field_bindings_owner_model_code_idx
    ON engine_field_bindings (owner_model_code);
CREATE UNIQUE INDEX engine_field_bindings_project_owner_field_uidx
    ON engine_field_bindings (project_id, owner_model_code, field_code);
