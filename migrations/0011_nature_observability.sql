CREATE TABLE nature_generation_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES nature_generation_runs (id),
    revision_id TEXT NOT NULL REFERENCES nature_revisions (id),
    parent_event_id TEXT NOT NULL,
    sequence BIGINT NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    message TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT NOT NULL,
    duration_ms BIGINT NOT NULL
);
CREATE INDEX nature_generation_events_run_id_idx
    ON nature_generation_events (run_id);
CREATE INDEX nature_generation_events_revision_id_idx
    ON nature_generation_events (revision_id);
CREATE UNIQUE INDEX nature_generation_events_run_sequence_uidx
    ON nature_generation_events (run_id, sequence);
