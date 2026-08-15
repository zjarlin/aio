CREATE FUNCTION engine_program_v14(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            definition,
            '{application_targets}',
            COALESCE(definition -> 'application_targets', '["web", "desktop"]'::JSONB)
        ),
        '{schema_version}',
        '14'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v14(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 14;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v14(definition),
    content_hash = 'migrated-v14:' || md5(engine_program_v14(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 14;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v14(JSONB);
