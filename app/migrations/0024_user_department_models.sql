-- 为用户管理页面补齐部门树、用户表及两者的关联模型。
CREATE FUNCTION engine_user_department_menu(menu JSONB, users_page_id TEXT)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT menu
        || CASE
            WHEN menu ->> 'page_id' = users_page_id THEN jsonb_build_object(
                'row_actions', jsonb_build_object(
                    'detail', jsonb_build_object('kind', 'public'),
                    'edit', jsonb_build_object('kind', 'public'),
                    'delete', jsonb_build_object('kind', 'public')
                )
            )
            ELSE '{}'::jsonb
        END
        || jsonb_build_object(
            'children', COALESCE(
                (
                    SELECT jsonb_agg(
                        engine_user_department_menu(child.value, users_page_id)
                        ORDER BY child.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(menu -> 'children', '[]'::jsonb))
                        WITH ORDINALITY AS child(value, ordinality)
                ),
                '[]'::jsonb
            )
        );
$$;

CREATE FUNCTION engine_add_user_department_models(definition JSONB)
RETURNS JSONB
LANGUAGE PLPGSQL
IMMUTABLE
AS $$
DECLARE
    department_model_id UUID := md5('aio.model:department')::uuid;
    department_name_id UUID := md5('aio.field:department:name')::uuid;
    department_code_id UUID := md5('aio.field:department:code')::uuid;
    department_parent_id UUID := md5('aio.field:department:parent_id')::uuid;
    user_model_id UUID := md5('aio.model:user')::uuid;
    user_username_id UUID := md5('aio.field:user:username')::uuid;
    user_display_name_id UUID := md5('aio.field:user:display_name')::uuid;
    user_email_id UUID := md5('aio.field:user:email')::uuid;
    user_status_id UUID := md5('aio.field:user:status')::uuid;
    user_department_id UUID := md5('aio.field:user:department_id')::uuid;
    users_page_id TEXT;
    models JSONB := COALESCE(definition -> 'models', '[]'::jsonb);
    pages JSONB;
    menus JSONB;
BEGIN
    SELECT page.value ->> 'id'
    INTO users_page_id
    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb)) AS page(value)
    WHERE page.value ->> 'name' IN ('users', 'user')
       OR page.value ->> 'title' = '用户管理'
    LIMIT 1;

    IF users_page_id IS NULL THEN
        RETURN definition;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM jsonb_array_elements(models) AS model(value)
        WHERE model.value ->> 'name' = 'department'
    ) THEN
        models := models || jsonb_build_array(jsonb_build_object(
            'id', department_model_id,
            'name', 'department',
            'title', '部门',
            'state', jsonb_build_object('kind', 'known'),
            'fields', jsonb_build_array(
                jsonb_build_object(
                    'id', department_name_id,
                    'name', 'name',
                    'title', '部门名称',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true
                ),
                jsonb_build_object(
                    'id', department_code_id,
                    'name', 'code',
                    'title', '部门编码',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true
                ),
                jsonb_build_object(
                    'id', department_parent_id,
                    'name', 'parent_id',
                    'title', '上级部门',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', false
                )
            ),
            'indexes', jsonb_build_array(
                jsonb_build_object(
                    'id', md5('aio.index:department:parent_id')::uuid,
                    'fields', jsonb_build_array(department_parent_id),
                    'purpose', 'relation'
                )
            )
        ));
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM jsonb_array_elements(models) AS model(value)
        WHERE model.value ->> 'name' = 'user'
    ) THEN
        models := models || jsonb_build_array(jsonb_build_object(
            'id', user_model_id,
            'name', 'user',
            'title', '用户',
            'state', jsonb_build_object('kind', 'known'),
            'fields', jsonb_build_array(
                jsonb_build_object(
                    'id', user_username_id,
                    'name', 'username',
                    'title', '用户名',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true
                ),
                jsonb_build_object(
                    'id', user_display_name_id,
                    'name', 'display_name',
                    'title', '姓名',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true
                ),
                jsonb_build_object(
                    'id', user_email_id,
                    'name', 'email',
                    'title', '邮箱',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', false
                ),
                jsonb_build_object(
                    'id', user_status_id,
                    'name', 'status',
                    'title', '状态',
                    'value_type', jsonb_build_object('kind', 'text'),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true
                ),
                jsonb_build_object(
                    'id', user_department_id,
                    'name', 'department_id',
                    'title', '所属部门',
                    'value_type', jsonb_build_object('kind', 'object', 'model_id', department_model_id),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', true,
                    'relation_model_id', department_model_id
                )
            ),
            'indexes', jsonb_build_array(
                jsonb_build_object(
                    'id', md5('aio.index:user:department_id')::uuid,
                    'fields', jsonb_build_array(user_department_id),
                    'purpose', 'relation'
                ),
                jsonb_build_object(
                    'id', md5('aio.index:user:username')::uuid,
                    'fields', jsonb_build_array(user_username_id),
                    'purpose', 'filter'
                )
            )
        ));
    END IF;

    SELECT jsonb_agg(
        CASE
            WHEN page.value ->> 'id' = users_page_id THEN
                page.value || jsonb_build_object(
                    'renderer', jsonb_build_object(
                        'kind', 'tree_table',
                        'tree', jsonb_build_object(
                            'model_id', department_model_id,
                            'label_field_id', department_name_id,
                            'parent_field_id', department_parent_id,
                            'table_relation_field_id', user_department_id
                        ),
                        'table', jsonb_build_object(
                            'model_id', user_model_id,
                            'columns', jsonb_build_array(
                                user_username_id,
                                user_display_name_id,
                                user_email_id,
                                user_status_id
                            ),
                            'filters', jsonb_build_array(user_username_id, user_status_id),
                            'page_size', 20
                        )
                    )
                )
            ELSE page.value
        END
        ORDER BY page.ordinality
    )
    INTO pages
    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb))
        WITH ORDINALITY AS page(value, ordinality);

    SELECT jsonb_agg(
        engine_user_department_menu(menu.value, users_page_id)
        ORDER BY menu.ordinality
    )
    INTO menus
    FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb))
        WITH ORDINALITY AS menu(value, ordinality);

    RETURN jsonb_set(
        jsonb_set(
            jsonb_set(definition, '{models}', models),
            '{pages}', COALESCE(pages, '[]'::jsonb)
        ),
        '{menus}', COALESCE(menus, '[]'::jsonb)
    );
END;
$$;

UPDATE engine_application_drafts
SET definition = engine_add_user_department_models(definition)
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_add_user_department_models(definition),
    content_hash = 'migrated-user-department:' || md5(
        engine_add_user_department_models(definition)::text
    )
WHERE definition ->> 'name' = 'aio-first-party';

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_add_user_department_models(JSONB);
DROP FUNCTION engine_user_department_menu(JSONB, TEXT);
