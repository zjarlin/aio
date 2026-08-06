CREATE FUNCTION engine_program_v7(definition JSONB)
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
                            COALESCE(page.value -> 'endpoints', '[]'::JSONB)
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
        '7'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v7(definition)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 7;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v7(definition),
    content_hash = 'migrated-v7:' || md5(engine_program_v7(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 7;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v7(JSONB);
