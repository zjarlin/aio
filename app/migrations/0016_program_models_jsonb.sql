ALTER TABLE engine_meta_models
    ADD COLUMN program_symbol_id TEXT NULL;
CREATE UNIQUE INDEX engine_meta_models_program_symbol_uidx
    ON engine_meta_models (program_symbol_id)
    WHERE program_symbol_id IS NOT NULL;

ALTER TABLE engine_meta_fields
    ADD COLUMN program_symbol_id TEXT NULL;
CREATE UNIQUE INDEX engine_meta_fields_program_symbol_uidx
    ON engine_meta_fields (program_symbol_id)
    WHERE program_symbol_id IS NOT NULL;

ALTER TABLE engine_data_records
    ALTER COLUMN payload TYPE JSONB USING payload::jsonb;

CREATE TABLE engine_program_expression_indexes (
    application_id TEXT NOT NULL REFERENCES engine_applications (id) ON DELETE CASCADE,
    index_name TEXT NOT NULL,
    model_symbol_id TEXT NOT NULL,
    field_symbol_ids JSONB NOT NULL,
    created_at_ms BIGINT NOT NULL,
    PRIMARY KEY (application_id, index_name)
);
