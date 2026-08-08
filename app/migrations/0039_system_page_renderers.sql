CREATE FUNCTION engine_enable_system_page_actions(menu JSONB, page_ids JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    children JSONB;
    result JSONB;
BEGIN
    SELECT COALESCE(
        jsonb_agg(
            engine_enable_system_page_actions(child.value, page_ids)
            ORDER BY child.ordinality
        ),
        '[]'::jsonb
    )
    INTO children
    FROM jsonb_array_elements(COALESCE(menu -> 'children', '[]'::jsonb))
        WITH ORDINALITY AS child(value, ordinality);

    result := jsonb_set(menu, '{children}', children);
    IF COALESCE(menu ->> 'page_id', '') IN (
        SELECT jsonb_array_elements_text(page_ids)
    ) THEN
        result := result || jsonb_build_object(
            'row_actions', jsonb_build_object(
                'detail', jsonb_build_object('kind', 'public'),
                'edit', jsonb_build_object('kind', 'public'),
                'delete', jsonb_build_object('kind', 'public')
            )
        );
    END IF;
    RETURN result;
END;
$$;

CREATE FUNCTION engine_system_page_renderers(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    system_page_names CONSTANT JSONB := '[
      "api-keys", "users", "roles", "departments", "dictionary", "menus",
      "audit", "sessions", "tenants", "messages", "oauth-clients",
      "social-clients", "areas"
    ]'::jsonb;
    models JSONB := COALESCE(definition -> 'models', '[]'::jsonb);
    pages JSONB := '[]'::jsonb;
    menus JSONB;
    page_ids JSONB;
    page_value JSONB;
    department_model JSONB;
    user_model JSONB;
    department_name_field_id TEXT;
    department_parent_field_id TEXT;
    user_department_field_id TEXT;
BEGIN
    SELECT model.value
    INTO department_model
    FROM jsonb_array_elements(models) AS model(value)
    WHERE model.value ->> 'name' IN ('dept', 'department')
    LIMIT 1;

    SELECT model.value
    INTO user_model
    FROM jsonb_array_elements(models) AS model(value)
    WHERE model.value ->> 'name' = 'user'
    LIMIT 1;

    SELECT field.value ->> 'id'
    INTO department_name_field_id
    FROM jsonb_array_elements(department_model -> 'fields') AS field(value)
    WHERE field.value ->> 'name' = 'name'
    LIMIT 1;

    SELECT field.value ->> 'id'
    INTO department_parent_field_id
    FROM jsonb_array_elements(department_model -> 'fields') AS field(value)
    WHERE field.value ->> 'name' = 'parent_id'
    LIMIT 1;

    SELECT field.value ->> 'id'
    INTO user_department_field_id
    FROM jsonb_array_elements(user_model -> 'fields') AS field(value)
    WHERE field.value ->> 'name' = 'department_id'
    LIMIT 1;

    IF department_model IS NULL
        OR user_model IS NULL
        OR department_name_field_id IS NULL
        OR user_department_field_id IS NULL
    THEN
        RAISE EXCEPTION '用户左树右表缺少部门模型或关联字段';
    END IF;

    FOR page_value IN
        SELECT value
        FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb))
    LOOP
        CASE page_value ->> 'name'
            WHEN 'users' THEN
                page_value := page_value || jsonb_build_object(
                    'renderer', jsonb_build_object(
                        'kind', 'tree_table',
                        'tree', jsonb_build_object(
                            'model_id', department_model ->> 'id',
                            'label_field_id', department_name_field_id,
                            'parent_field_id', department_parent_field_id,
                            'table_relation_field_id', user_department_field_id
                        ),
                        'table', jsonb_build_object(
                            'model_id', user_model ->> 'id',
                            'page_size', 20
                        )
                    )
                );
            WHEN 'menus' THEN
                page_value := page_value || jsonb_build_object(
                    'renderer', jsonb_build_object('kind', 'menu_tree')
                );
            ELSE
        END CASE;
        pages := pages || jsonb_build_array(page_value);
    END LOOP;

    SELECT COALESCE(jsonb_agg(page.value ->> 'id'), '[]'::jsonb)
    INTO page_ids
    FROM jsonb_array_elements(pages) AS page(value)
    WHERE page.value ->> 'name' IN (
        SELECT jsonb_array_elements_text(system_page_names)
    );

    SELECT COALESCE(
        jsonb_agg(
            engine_enable_system_page_actions(menu.value, page_ids)
            ORDER BY menu.ordinality
        ),
        '[]'::jsonb
    )
    INTO menus
    FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb))
        WITH ORDINALITY AS menu(value, ordinality);

    RETURN jsonb_set(
        jsonb_set(definition, '{pages}', pages),
        '{menus}',
        menus
    );
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_system_page_renderers(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_system_page_renderers(definition),
    content_hash = 'migrated-system-renderers:' || md5(
        engine_system_page_renderers(definition)::TEXT
    )
WHERE definition ->> 'name' = 'aio-first-party';

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_system_page_renderers(JSONB);
DROP FUNCTION engine_enable_system_page_actions(JSONB, JSONB);
