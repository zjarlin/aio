CREATE TABLE nature_application_deployments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES nature_projects (id),
    revision_id TEXT NOT NULL UNIQUE REFERENCES nature_revisions (id),
    artifact_hash TEXT NOT NULL,
    domain_code TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'inactive')),
    manifest TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    activated_at_ms BIGINT NOT NULL
);
CREATE INDEX nature_application_deployments_project_id_idx
    ON nature_application_deployments (project_id);
CREATE UNIQUE INDEX nature_application_deployments_active_project_uidx
    ON nature_application_deployments (project_id)
    WHERE state = 'active';

CREATE TABLE engine_route_definitions (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES nature_application_deployments (id),
    method TEXT NOT NULL,
    path_template TEXT NOT NULL,
    operation_key TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);
CREATE INDEX engine_route_definitions_deployment_id_idx
    ON engine_route_definitions (deployment_id);
CREATE UNIQUE INDEX engine_route_definitions_deployment_route_uidx
    ON engine_route_definitions (deployment_id, method, path_template);
