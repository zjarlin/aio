CREATE FUNCTION engine_permission_identifier_v10(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        definition,
        '{permissions}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN permission.value ->> 'name' ~ '^[a-z][a-z0-9_-]*(:[a-z][a-z0-9_-]*)+$'
                            THEN permission.value
                        ELSE permission.value || jsonb_build_object(
                            'name',
                            'system:permission_' || COALESCE(
                                NULLIF(
                                    regexp_replace(
                                        lower(COALESCE(permission.value ->> 'name', '')),
                                        '[^a-z0-9_-]+',
                                        '_',
                                        'g'
                                    ),
                                    ''
                                ),
                                'unnamed'
                            )
                        )
                    END
                    ORDER BY permission.ordinality
                )
                FROM jsonb_array_elements(COALESCE(definition -> 'permissions', '[]'::JSONB))
                    WITH ORDINALITY AS permission(value, ordinality)
            ),
            '[]'::JSONB
        )
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_permission_identifier_v10(definition)
WHERE definition ? 'permissions';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_permission_identifier_v10(definition),
    content_hash = 'migrated-permissions:' || md5(engine_permission_identifier_v10(definition)::TEXT)
WHERE definition ? 'permissions';

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_permission_identifier_v10(JSONB);
