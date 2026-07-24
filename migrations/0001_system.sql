CREATE TABLE biz_system_admin_system_page_records (
    id TEXT PRIMARY KEY,
    route TEXT NOT NULL,
    label TEXT NOT NULL,
    status TEXT NOT NULL,
    pg_tables TEXT NOT NULL,
    operations TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_system_admin_system_page_records_route_idx
    ON biz_system_admin_system_page_records (route);

CREATE TABLE biz_system_admin_system_operation_records (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL,
    page_id TEXT NOT NULL,
    method TEXT NOT NULL,
    api_path TEXT NOT NULL,
    cli TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX biz_system_admin_system_operation_records_operation_id_idx
    ON biz_system_admin_system_operation_records (operation_id);

CREATE TABLE biz_system_admin_system_data_records (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL,
    row_key TEXT NOT NULL,
    cells_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_system_admin_system_data_records_page_id_idx
    ON biz_system_admin_system_data_records (page_id);

CREATE TABLE biz_system_admin_system_api_key_records (
    id TEXT PRIMARY KEY,
    key_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL,
    scope TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);
CREATE INDEX biz_system_admin_system_api_key_records_key_hash_idx
    ON biz_system_admin_system_api_key_records (key_hash);

CREATE TABLE sys_dict_type (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    scope TEXT NOT NULL,
    raw_value_kind TEXT NOT NULL,
    open_enum BOOLEAN NOT NULL,
    sort_index BIGINT NOT NULL,
    status TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX sys_dict_type_code_uq ON sys_dict_type (code);
CREATE INDEX sys_dict_type_scope_idx ON sys_dict_type (scope);

CREATE TABLE sys_dict_data (
    id TEXT PRIMARY KEY,
    dictionary_type_id TEXT NOT NULL,
    code TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    raw_value TEXT NOT NULL,
    sort_index BIGINT NOT NULL,
    status TEXT NOT NULL,
    meta_json TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE INDEX sys_dict_data_dictionary_type_id_idx ON sys_dict_data (dictionary_type_id);
CREATE UNIQUE INDEX sys_dict_data_type_code_uq ON sys_dict_data (dictionary_type_id, code);
CREATE UNIQUE INDEX sys_dict_data_type_raw_value_uq ON sys_dict_data (dictionary_type_id, raw_value);
