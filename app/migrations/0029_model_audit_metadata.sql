ALTER TABLE engine_meta_models
    ADD COLUMN audit_metadata_json TEXT NOT NULL DEFAULT '{"fields":[]}';

CREATE FUNCTION engine_program_v9(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            definition,
            '{models}',
            COALESCE(
                (
                    SELECT jsonb_agg(
                        model.value || jsonb_build_object(
                            'audit',
                            COALESCE(model.value -> 'audit', '{"fields":[]}'::JSONB)
                        )
                        ORDER BY model.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::JSONB))
                        WITH ORDINALITY AS model(value, ordinality)
                ),
                '[]'::JSONB
            )
        ),
        '{schema_version}',
        '9'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v9(definition)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 9;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v9(definition),
    content_hash = 'migrated-v9:' || md5(engine_program_v9(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 9;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v9(JSONB);
