CREATE FUNCTION engine_program_v5_menu(menu JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT menu
        || jsonb_build_object(
            'row_actions',
            jsonb_build_object(
                'detail', jsonb_build_object('kind', 'hidden'),
                'edit', jsonb_build_object('kind', 'hidden'),
                'delete', jsonb_build_object('kind', 'hidden')
            )
        )
        || jsonb_build_object(
            'children',
            COALESCE(
                (
                    SELECT jsonb_agg(engine_program_v5_menu(child.value) ORDER BY child.ordinality)
                    FROM jsonb_array_elements(COALESCE(menu -> 'children', '[]'::jsonb))
                        WITH ORDINALITY AS child(value, ordinality)
                ),
                '[]'::jsonb
            )
        );
$$;

CREATE FUNCTION engine_program_v5(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            jsonb_set(
                definition,
                '{menus}',
                COALESCE(
                    (
                        SELECT jsonb_agg(engine_program_v5_menu(menu.value) ORDER BY menu.ordinality)
                        FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb))
                            WITH ORDINALITY AS menu(value, ordinality)
                    ),
                    '[]'::jsonb
                )
            ),
            '{pages}',
            COALESCE(
                (
                    SELECT jsonb_agg(
                        (page.value - 'root' - 'page_state' - 'data_sources')
                            || jsonb_build_object(
                                'renderer', jsonb_build_object('kind', 'convention_file')
                            )
                        ORDER BY page.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb))
                        WITH ORDINALITY AS page(value, ordinality)
                ),
                '[]'::jsonb
            )
        ),
        '{schema_version}',
        '5'::jsonb
    );
$$;

UPDATE engine_application_drafts
SET definition = engine_program_v5(definition)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 5;

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_program_v5(definition),
    content_hash = 'migrated-v5:' || md5(engine_program_v5(definition)::text)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 5;

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v5(JSONB);
DROP FUNCTION engine_program_v5_menu(JSONB);
