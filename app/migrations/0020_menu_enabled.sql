CREATE FUNCTION engine_add_menu_enabled(menu JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT menu
        || jsonb_build_object('enabled', COALESCE(menu -> 'enabled', 'true'::jsonb))
        || jsonb_build_object(
            'children',
            COALESCE(
                (
                    SELECT jsonb_agg(engine_add_menu_enabled(child.value) ORDER BY child.ordinality)
                    FROM jsonb_array_elements(COALESCE(menu -> 'children', '[]'::jsonb))
                        WITH ORDINALITY AS child(value, ordinality)
                ),
                '[]'::jsonb
            )
        );
$$;

CREATE FUNCTION engine_enable_program_menus(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            definition,
            '{menus}',
            COALESCE(
                (
                    SELECT jsonb_agg(engine_add_menu_enabled(menu.value) ORDER BY menu.ordinality)
                    FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb))
                        WITH ORDINALITY AS menu(value, ordinality)
                ),
                '[]'::jsonb
            )
        ),
        '{schema_version}',
        '4'::jsonb
    );
$$;

UPDATE engine_application_drafts
SET definition = engine_enable_program_menus(definition)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 4;

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_enable_program_menus(definition),
    content_hash = 'migrated-v4:' || md5(engine_enable_program_menus(definition)::text)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 4;

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_enable_program_menus(JSONB);
DROP FUNCTION engine_add_menu_enabled(JSONB);
