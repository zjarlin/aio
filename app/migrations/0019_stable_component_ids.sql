CREATE FUNCTION engine_stable_component_id(component_id TEXT)
RETURNS TEXT
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT CASE
        WHEN component_id LIKE 'ui.%' THEN component_id
        ELSE 'ui.' || lower(
            replace(
                regexp_replace(
                    regexp_replace(component_id, '^.*::', ''),
                    '([a-z0-9])([A-Z])',
                    '\1-\2',
                    'g'
                ),
                '_',
                '-'
            )
        )
    END;
$$;

CREATE FUNCTION engine_rewrite_component_ids(value JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    result JSONB;
    item RECORD;
BEGIN
    CASE jsonb_typeof(value)
        WHEN 'object' THEN
            result := '{}'::jsonb;
            FOR item IN
                SELECT entry.key, entry.value AS child
                FROM jsonb_each(value) AS entry(key, value)
            LOOP
                IF item.key = 'component' AND jsonb_typeof(item.child) = 'string' THEN
                    result := result || jsonb_build_object(
                        item.key,
                        engine_stable_component_id(item.child #>> '{}')
                    );
                ELSE
                    result := result || jsonb_build_object(
                        item.key,
                        engine_rewrite_component_ids(item.child)
                    );
                END IF;
            END LOOP;
            RETURN result;
        WHEN 'array' THEN
            SELECT COALESCE(jsonb_agg(engine_rewrite_component_ids(child)), '[]'::jsonb)
            INTO result
            FROM jsonb_array_elements(value) AS children(child);
            RETURN result;
        ELSE
            RETURN value;
    END CASE;
END;
$$;

UPDATE engine_application_drafts
SET definition = jsonb_set(
    engine_rewrite_component_ids(definition),
    '{schema_version}',
    '3'::jsonb
)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 3
   OR definition::text LIKE '%::components::%';

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = jsonb_set(
        engine_rewrite_component_ids(definition),
        '{schema_version}',
        '3'::jsonb
    ),
    content_hash = 'migrated-v3:' || md5(
        jsonb_set(
            engine_rewrite_component_ids(definition),
            '{schema_version}',
            '3'::jsonb
        )::text
    )
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 3
   OR definition::text LIKE '%::components::%';

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_rewrite_component_ids(JSONB);
DROP FUNCTION engine_stable_component_id(TEXT);
