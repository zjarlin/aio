CREATE FUNCTION engine_promote_contexts_to_root_menus(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT (definition - 'contexts') || jsonb_build_object(
        'schema_version', 2,
        'menus', COALESCE(
            (
                SELECT jsonb_agg(
                    jsonb_build_object(
                        'id', context.value -> 'id',
                        'name', context.value -> 'name',
                        'title', context.value -> 'title',
                        'state', COALESCE(context.value -> 'state', '{"kind":"known"}'::jsonb),
                        'children', COALESCE(context.value -> 'menus', '[]'::jsonb),
                        'required_permissions', '[]'::jsonb
                    )
                    ORDER BY context.ordinality
                )
                FROM jsonb_array_elements(COALESCE(definition -> 'contexts', '[]'::jsonb))
                    WITH ORDINALITY AS context(value, ordinality)
            ),
            '[]'::jsonb
        )
    );
$$;

UPDATE engine_application_drafts
SET definition = engine_promote_contexts_to_root_menus(definition)
WHERE definition ? 'contexts';

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_promote_contexts_to_root_menus(definition),
    content_hash = 'migrated-v2:' || md5(
        engine_promote_contexts_to_root_menus(definition)::text
    )
WHERE definition ? 'contexts';

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_promote_contexts_to_root_menus(JSONB);
