CREATE FUNCTION engine_program_v13_endpoint(endpoint JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT endpoint - 'implementation';
$$;

CREATE FUNCTION engine_program_v13(definition JSONB)
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
                                        engine_program_v13_endpoint(endpoint.value)
                                        ORDER BY endpoint.ordinality
                                    )
                                    FROM jsonb_array_elements(
                                        COALESCE(page.value -> 'endpoints', '[]'::JSONB)
                                    ) WITH ORDINALITY AS endpoint(value, ordinality)
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
        '13'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v13(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 13;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v13(definition),
    content_hash = 'migrated-v13:' || md5(engine_program_v13(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 13;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TABLE IF EXISTS biz_edge_gateway_edge_usage_record_rows;
DROP TABLE IF EXISTS biz_edge_gateway_edge_api_token_records;
DROP TABLE IF EXISTS biz_edge_gateway_gateway_route_definitions;
DROP TABLE IF EXISTS biz_edge_gateway_gateway_flows;
DROP TABLE IF EXISTS biz_software_center_software_package_records;
DROP TABLE IF EXISTS biz_asset_hub_asset_records;
DROP TABLE IF EXISTS biz_drive_center_drive_tasks;
DROP TABLE IF EXISTS biz_config_center_config_entries;

DROP FUNCTION engine_program_v13(JSONB);
DROP FUNCTION engine_program_v13_endpoint(JSONB);
