CREATE FUNCTION engine_program_v11_endpoint(endpoint JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    plugin_id TEXT;
    implementation JSONB;
BEGIN
    plugin_id := CASE
        WHEN endpoint ->> 'path' LIKE '/api/algorithm-center/%' THEN 'algorithm-center'
        WHEN endpoint ->> 'path' LIKE '/api/software-center/%' THEN 'software-center'
        WHEN endpoint ->> 'path' LIKE '/api/drive-center/%' THEN 'drive-center'
        WHEN endpoint ->> 'path' LIKE '/api/config-center/%' THEN 'config-center'
        WHEN endpoint ->> 'path' LIKE '/api/asset-hub/%' THEN 'asset-hub'
        WHEN endpoint ->> 'path' LIKE '/api/edge-gateway/%' THEN 'edge-gateway'
        WHEN endpoint ->> 'path' LIKE '/api/iot/%' THEN 'iot-center'
        WHEN endpoint ->> 'path' LIKE '/api/ssh/%' THEN 'ssh'
        WHEN endpoint ->> 'path' LIKE '/api/linux/%' THEN 'linux'
        ELSE NULL
    END;
    implementation := CASE
        WHEN plugin_id IS NULL THEN jsonb_build_object('kind', 'convention')
        ELSE jsonb_build_object('kind', 'native', 'plugin_id', plugin_id)
    END;
    RETURN endpoint || jsonb_build_object(
        'description', COALESCE(
            NULLIF(endpoint ->> 'description', ''),
            NULLIF(endpoint ->> 'title', ''),
            endpoint ->> 'path',
            ''
        ),
        'implementation', implementation
    );
END;
$$;

CREATE FUNCTION engine_program_v11(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            definition,
            '{pages}',
            COALESCE(
                (
                    SELECT jsonb_agg(
                        page.value || jsonb_build_object(
                            'endpoints',
                            COALESCE(
                                (
                                    SELECT jsonb_agg(
                                        engine_program_v11_endpoint(endpoint.value)
                                        ORDER BY endpoint.ordinality
                                    )
                                    FROM jsonb_array_elements(
                                        COALESCE(page.value -> 'endpoints', '[]'::JSONB)
                                    ) WITH ORDINALITY AS endpoint(value, ordinality)
                                    WHERE endpoint.value ->> 'path'
                                        NOT LIKE '/api/assets/custom-endpoint-%'
                                ),
                                '[]'::JSONB
                            )
                        )
                        ORDER BY page.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB))
                        WITH ORDINALITY AS page(value, ordinality)
                ),
                '[]'::JSONB
            )
        ),
        '{schema_version}',
        '11'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v11(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 11;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v11(definition),
    content_hash = 'migrated-v11:' || md5(engine_program_v11(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 11;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v11(JSONB);
DROP FUNCTION engine_program_v11_endpoint(JSONB);
