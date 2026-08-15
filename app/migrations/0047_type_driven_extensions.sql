CREATE FUNCTION engine_program_v15(definition JSONB)
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
                        CASE
                            WHEN page.value #>> '{renderer,kind}' = 'extension'
                                THEN jsonb_set(
                                    page.value,
                                    '{renderer}',
                                    '{"kind":"convention_file"}'::JSONB
                                )
                            ELSE page.value
                        END
                        ORDER BY page.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB))
                        WITH ORDINALITY AS page(value, ordinality)
                ),
                '[]'::JSONB
            )
        ),
        '{schema_version}',
        '15'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v15(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 15;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v15(definition),
    content_hash = 'migrated-v15:' || md5(engine_program_v15(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 15;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v15(JSONB);
