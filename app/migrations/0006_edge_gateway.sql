CREATE TABLE biz_edge_gateway_gateway_flows (
    id TEXT PRIMARY KEY,
    route TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_edge_gateway_gateway_flows_route_idx
    ON biz_edge_gateway_gateway_flows (route);

CREATE TABLE biz_edge_gateway_gateway_route_definitions (
    id TEXT PRIMARY KEY,
    route TEXT NOT NULL,
    method TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    auth_required TEXT NOT NULL,
    script_language TEXT NOT NULL,
    script_code TEXT NOT NULL,
    request_example TEXT NOT NULL,
    response_template TEXT NOT NULL,
    notes TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_edge_gateway_gateway_route_definitions_route_idx
    ON biz_edge_gateway_gateway_route_definitions (route);
CREATE INDEX biz_edge_gateway_gateway_route_definitions_method_idx
    ON biz_edge_gateway_gateway_route_definitions (method);

CREATE TABLE biz_edge_gateway_edge_api_token_records (
    id TEXT PRIMARY KEY,
    token_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    allowed_routes_json TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at_epoch_secs TEXT NOT NULL,
    last_used_at_epoch_secs TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX biz_edge_gateway_edge_api_token_records_token_hash_idx
    ON biz_edge_gateway_edge_api_token_records (token_hash);

CREATE TABLE biz_edge_gateway_edge_usage_record_rows (
    id TEXT PRIMARY KEY,
    token_id TEXT NOT NULL,
    route TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    status_code TEXT NOT NULL,
    request_units TEXT NOT NULL,
    duration_ms TEXT NOT NULL,
    created_at_epoch_secs TEXT NOT NULL
);
CREATE INDEX biz_edge_gateway_edge_usage_record_rows_token_id_idx
    ON biz_edge_gateway_edge_usage_record_rows (token_id);
CREATE INDEX biz_edge_gateway_edge_usage_record_rows_route_idx
    ON biz_edge_gateway_edge_usage_record_rows (route);
