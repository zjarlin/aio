CREATE TABLE engine_page_definitions (
    id TEXT PRIMARY KEY,
    page_key TEXT NOT NULL,
    route TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('draft', 'published', 'disabled')),
    definition TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
CREATE UNIQUE INDEX engine_page_definitions_page_key_uidx
    ON engine_page_definitions (page_key);
CREATE UNIQUE INDEX engine_page_definitions_route_uidx
    ON engine_page_definitions (route);
