CREATE TABLE engine_applications (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    active_revision_id TEXT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX engine_applications_name_uidx
    ON engine_applications (name);

CREATE TABLE engine_application_drafts (
    application_id TEXT PRIMARY KEY REFERENCES engine_applications (id) ON DELETE CASCADE,
    version BIGINT NOT NULL DEFAULT 0 CHECK (version >= 0),
    definition JSONB NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE TABLE engine_application_revisions (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES engine_applications (id) ON DELETE CASCADE,
    revision BIGINT NOT NULL CHECK (revision > 0),
    definition JSONB NOT NULL,
    content_hash TEXT NOT NULL,
    origin TEXT NOT NULL CHECK (origin IN ('studio', 'vibe', 'migration', 'rollback')),
    diagnostics JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX engine_application_revisions_number_uidx
    ON engine_application_revisions (application_id, revision);
CREATE INDEX engine_application_revisions_created_idx
    ON engine_application_revisions (application_id, created_at_ms DESC);

ALTER TABLE engine_applications
    ADD CONSTRAINT engine_applications_active_revision_fk
    FOREIGN KEY (active_revision_id) REFERENCES engine_application_revisions (id);

CREATE TABLE engine_program_images (
    content_hash TEXT NOT NULL,
    compiler_version TEXT NOT NULL,
    target TEXT NOT NULL CHECK (target IN ('server', 'wasm', 'universal')),
    revision_id TEXT NOT NULL REFERENCES engine_application_revisions (id) ON DELETE CASCADE,
    image BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL,
    PRIMARY KEY (content_hash, compiler_version, target)
);
CREATE INDEX engine_program_images_revision_id_idx
    ON engine_program_images (revision_id);

CREATE TABLE engine_revision_runs (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES engine_applications (id) ON DELETE CASCADE,
    revision_id TEXT NULL REFERENCES engine_application_revisions (id) ON DELETE SET NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    stage TEXT NOT NULL,
    diagnostics JSONB NOT NULL DEFAULT '[]'::jsonb,
    tests JSONB NOT NULL DEFAULT '[]'::jsonb,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT NOT NULL DEFAULT 0,
    duration_ms BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX engine_revision_runs_application_idx
    ON engine_revision_runs (application_id, started_at_ms DESC);

CREATE TABLE engine_vibe_sessions (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES engine_applications (id) ON DELETE CASCADE,
    base_version BIGINT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    final_revision_id TEXT NULL REFERENCES engine_application_revisions (id) ON DELETE SET NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_vibe_sessions_application_idx
    ON engine_vibe_sessions (application_id, created_at_ms DESC);

CREATE TABLE engine_vibe_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES engine_vibe_sessions (id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    role TEXT NOT NULL CHECK (role IN ('user', 'agent', 'gate')),
    prompt TEXT NOT NULL,
    model TEXT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    patch JSONB NULL,
    diagnostics JSONB NOT NULL DEFAULT '[]'::jsonb,
    tests JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX engine_vibe_messages_sequence_uidx
    ON engine_vibe_messages (session_id, sequence);

CREATE FUNCTION engine_reject_immutable_program_row()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'immutable program row cannot be changed';
END;
$$;

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

CREATE FUNCTION engine_notify_program_activation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.active_revision_id IS DISTINCT FROM OLD.active_revision_id
       AND NEW.active_revision_id IS NOT NULL THEN
        PERFORM pg_notify(
            'engine_program_activated',
            json_build_object(
                'application_id', NEW.id,
                'revision_id', NEW.active_revision_id
            )::text
        );
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER engine_program_activation_notify
AFTER UPDATE OF active_revision_id ON engine_applications
FOR EACH ROW EXECUTE FUNCTION engine_notify_program_activation();
