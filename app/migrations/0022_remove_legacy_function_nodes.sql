CREATE FUNCTION engine_program_v5_strip_function(function_definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    WITH removed_nodes AS (
        SELECT node.value ->> 'id' AS id
        FROM jsonb_array_elements(
            COALESCE(function_definition -> 'graph' -> 'nodes', '[]'::jsonb)
        ) AS node(value)
        WHERE node.value -> 'kind' ->> 'kind' IN (
            'set_state',
            'open_dialog',
            'close_dialog',
            'refresh'
        )
    ),
    retained_nodes AS (
        SELECT COALESCE(jsonb_agg(node.value ORDER BY node.ordinality), '[]'::jsonb) AS value
        FROM jsonb_array_elements(
            COALESCE(function_definition -> 'graph' -> 'nodes', '[]'::jsonb)
        ) WITH ORDINALITY AS node(value, ordinality)
        WHERE node.value -> 'kind' ->> 'kind' NOT IN (
            'set_state',
            'open_dialog',
            'close_dialog',
            'refresh'
        )
    ),
    retained_edges AS (
        SELECT COALESCE(jsonb_agg(edge.value ORDER BY edge.ordinality), '[]'::jsonb) AS value
        FROM jsonb_array_elements(
            COALESCE(function_definition -> 'graph' -> 'edges', '[]'::jsonb)
        ) WITH ORDINALITY AS edge(value, ordinality)
        WHERE NOT EXISTS (
            SELECT 1
            FROM removed_nodes
            WHERE id = edge.value ->> 'from_node'
               OR id = edge.value ->> 'to_node'
        )
    )
    SELECT jsonb_set(
        jsonb_set(function_definition, '{graph,nodes}', retained_nodes.value),
        '{graph,edges}',
        retained_edges.value
    )
    FROM retained_nodes, retained_edges;
$$;

CREATE FUNCTION engine_program_v5_strip_legacy_nodes(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        definition,
        '{functions}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    engine_program_v5_strip_function(function_definition.value)
                    ORDER BY function_definition.ordinality
                )
                FROM jsonb_array_elements(COALESCE(definition -> 'functions', '[]'::jsonb))
                    WITH ORDINALITY AS function_definition(value, ordinality)
            ),
            '[]'::jsonb
        )
    );
$$;

UPDATE engine_application_drafts
SET definition = engine_program_v5_strip_legacy_nodes(definition)
WHERE jsonb_path_exists(
    definition,
    '$.functions[*].graph.nodes[*].kind.kind ? (@ == "set_state" || @ == "open_dialog" || @ == "close_dialog" || @ == "refresh")'
);

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_program_v5_strip_legacy_nodes(definition),
    content_hash = 'migrated-v5-nodes:'
        || md5(engine_program_v5_strip_legacy_nodes(definition)::text)
WHERE jsonb_path_exists(
    definition,
    '$.functions[*].graph.nodes[*].kind.kind ? (@ == "set_state" || @ == "open_dialog" || @ == "close_dialog" || @ == "refresh")'
);

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v5_strip_legacy_nodes(JSONB);
DROP FUNCTION engine_program_v5_strip_function(JSONB);
