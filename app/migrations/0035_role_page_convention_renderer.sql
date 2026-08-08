CREATE FUNCTION engine_use_role_convention_renderer(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        definition,
        '{pages}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN page.value ->> 'name' IN ('roles', 'role')
                            OR page.value ->> 'title' IN ('角色权限', '角色管理')
                        THEN page.value || jsonb_build_object(
                            'renderer', jsonb_build_object('kind', 'convention_file')
                        )
                        ELSE page.value
                    END
                    ORDER BY page.ordinality
                )
                FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb))
                    WITH ORDINALITY AS page(value, ordinality)
            ),
            '[]'::jsonb
        )
    );
$$;

UPDATE engine_program_drafts
SET definition = engine_use_role_convention_renderer(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_use_role_convention_renderer(definition),
    content_hash = 'migrated-role-convention:' || md5(
        engine_use_role_convention_renderer(definition)::TEXT
    )
WHERE definition ->> 'name' = 'aio-first-party';

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_use_role_convention_renderer(JSONB);
