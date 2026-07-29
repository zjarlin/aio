CREATE TABLE engine_meta_models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_meta_models_name_idx ON engine_meta_models (name);

CREATE TABLE engine_meta_fields (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    field_type TEXT NOT NULL,
    is_required BOOLEAN NOT NULL,
    expression TEXT NULL,
    dependency_json TEXT NULL,
    order_index INTEGER NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_meta_fields_model_name_idx ON engine_meta_fields (model_name);

CREATE TABLE engine_hook_definitions (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    trigger_event TEXT NOT NULL,
    script_content TEXT NOT NULL,
    is_active BOOLEAN NOT NULL,
    order_index INTEGER NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_hook_definitions_model_name_idx ON engine_hook_definitions (model_name);

CREATE TABLE engine_data_records (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_data_records_model_name_idx ON engine_data_records (model_name);
