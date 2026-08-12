ALTER TABLE engine_meta_models
    ADD COLUMN primary_key_generation TEXT NOT NULL DEFAULT 'uuid'
    CHECK (primary_key_generation IN ('uuid', 'auto_increment'));

CREATE SEQUENCE engine_data_record_auto_id_seq AS BIGINT;

SELECT setval(
    'engine_data_record_auto_id_seq',
    COALESCE(
        (
            SELECT MAX(id::BIGINT)
            FROM engine_data_records
            WHERE id ~ '^[0-9]+$'
        ),
        0
    ) + 1,
    FALSE
);

CREATE FUNCTION engine_program_v12(definition JSONB)
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
                            'primary_key',
                            COALESCE(
                                model.value -> 'primary_key',
                                '{"generation":"uuid"}'::JSONB
                            )
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
        '12'::JSONB
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v12(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 12;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v12(definition),
    content_hash = 'migrated-v12:' || md5(engine_program_v12(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 12;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v12(JSONB);
