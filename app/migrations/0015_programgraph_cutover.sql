DO $$
DECLARE
    blockers JSONB;
BEGIN
    SELECT jsonb_agg(item ORDER BY item->>'kind', item->>'id')
    INTO blockers
    FROM (
        SELECT jsonb_build_object(
            'kind', 'hook',
            'id', id,
            'name', model_name || ':' || trigger_event
        ) AS item
        FROM engine_hook_definitions
        WHERE is_active
        UNION ALL
        SELECT jsonb_build_object(
            'kind', 'operation',
            'id', definition.id,
            'name', definition.operation_key
        ) AS item
        FROM engine_operation_definitions definition
        WHERE definition.state = 'published'
          AND definition.active_revision_id IS NOT NULL
        UNION ALL
        SELECT jsonb_build_object(
            'kind', 'nature_deployment',
            'id', id,
            'name', domain_code
        ) AS item
        FROM nature_application_deployments
        WHERE state = 'active'
    ) active_objects;

    IF blockers IS NOT NULL THEN
        RAISE EXCEPTION 'ProgramGraph migration blocked by active legacy objects: %', blockers;
    END IF;
END;
$$;

CREATE FUNCTION engine_convert_legacy_component(node JSONB, identity TEXT)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    children JSONB;
    component_name TEXT;
BEGIN
    IF jsonb_typeof(node) <> 'object' OR node ->> 'component' IS NULL THEN
        RAISE EXCEPTION 'legacy PageDefinition component is invalid at %: %', identity, node;
    END IF;

    component_name := lower(
        replace(
            regexp_replace(
                regexp_replace(node ->> 'component', '^.*::', ''),
                '([a-z0-9])([A-Z])',
                '\1-\2',
                'g'
            ),
            '_',
            '-'
        )
    );
    SELECT COALESCE(
        jsonb_agg(
            engine_convert_legacy_component(child.value, identity || ':' || child.ordinality)
            ORDER BY child.ordinality
        ),
        '[]'::jsonb
    )
    INTO children
    FROM jsonb_array_elements(COALESCE(node -> 'children', '[]'::jsonb))
        WITH ORDINALITY AS child(value, ordinality);

    RETURN jsonb_strip_nulls(jsonb_build_object(
        'id', (md5('aio.legacy.component:' || identity))::uuid::text,
        'component', 'ui.' || component_name,
        'state', jsonb_build_object('kind', 'known'),
        'properties', COALESCE(node -> 'properties', '{}'::jsonb),
        'content', node -> 'content',
        'events', '{}'::jsonb,
        'children', children,
        'style', jsonb_build_object('responsive', '{}'::jsonb)
    ));
END;
$$;

DO $$
DECLARE
    seed_application_id CONSTANT TEXT := 'a1000000-0000-4000-8000-000000000001';
    program_id CONSTANT TEXT := 'b1000000-0000-4000-8000-000000000001';
    system_context_id CONSTANT TEXT := 'c1000000-0000-4000-8000-000000000001';
    resource_context_id CONSTANT TEXT := 'c1000000-0000-4000-8000-000000000002';
    infrastructure_context_id CONSTANT TEXT := 'c1000000-0000-4000-8000-000000000003';
    now_ms BIGINT := floor(extract(epoch FROM clock_timestamp()) * 1000);
    specs JSONB := '[
      {"context":"system","name":"api-keys","title":"API 密钥","route":"/system/account/api-keys"},
      {"context":"system","name":"users","title":"用户管理","route":"/system/identity/users"},
      {"context":"system","name":"roles","title":"角色权限","route":"/system/permission/roles"},
      {"context":"system","name":"departments","title":"组织部门","route":"/system/organization/departments"},
      {"context":"system","name":"dictionary","title":"字典管理","route":"/system/dictionary/note-types"},
      {"context":"system","name":"menus","title":"菜单挂载","route":"/system/menu/mounting"},
      {"context":"system","name":"audit","title":"审计事件","route":"/system/audit/events"},
      {"context":"system","name":"sessions","title":"认证会话","route":"/system/auth/sessions"},
      {"context":"system","name":"tenants","title":"租户管理","route":"/system/tenant/tenants"},
      {"context":"system","name":"messages","title":"消息模板","route":"/system/messaging/templates"},
      {"context":"system","name":"oauth-clients","title":"OAuth2 客户端","route":"/system/oauth2/clients"},
      {"context":"system","name":"social-clients","title":"社交客户端","route":"/system/social/clients"},
      {"context":"system","name":"areas","title":"地区树","route":"/system/area/tree"},
      {"context":"system","name":"config","title":"配置中心","route":"/config"},
      {"context":"resource","name":"assets","title":"资产中心","route":"/assets"},
      {"context":"resource","name":"drive","title":"网盘中心","route":"/drive"},
      {"context":"resource","name":"software","title":"软件中心","route":"/software"},
      {"context":"infrastructure","name":"gateway","title":"边缘网关","route":"/gateway"},
      {"context":"infrastructure","name":"linux","title":"Linux 管理","route":"/linux"},
      {"context":"infrastructure","name":"algorithms","title":"算法中心","route":"/algorithms"},
      {"context":"infrastructure","name":"iot","title":"物联网中心","route":"/iot"},
      {"context":"infrastructure","name":"ssh","title":"SSH 运维","route":"/ssh"}
    ]'::jsonb;
    item JSONB;
    page_id TEXT;
    root_id TEXT;
    title_id TEXT;
    description_id TEXT;
    menu_id TEXT;
    route_id TEXT;
    pages JSONB := '[]'::jsonb;
    routes JSONB := '[]'::jsonb;
    system_menus JSONB := '[]'::jsonb;
    resource_menus JSONB := '[]'::jsonb;
    infrastructure_menus JSONB := '[]'::jsonb;
    menu JSONB;
    definition JSONB;
    legacy_page RECORD;
    legacy_definition JSONB;
    legacy_route TEXT;
BEGIN
    FOR item IN SELECT value FROM jsonb_array_elements(specs)
    LOOP
        page_id := (md5('aio.page:' || (item->>'route')))::uuid::text;
        root_id := (md5('aio.root:' || (item->>'route')))::uuid::text;
        title_id := (md5('aio.title:' || (item->>'route')))::uuid::text;
        description_id := (md5('aio.description:' || (item->>'route')))::uuid::text;
        menu_id := (md5('aio.menu:' || (item->>'route')))::uuid::text;
        route_id := (md5('aio.route:' || (item->>'route')))::uuid::text;

        pages := pages || jsonb_build_array(jsonb_build_object(
            'id', page_id,
            'name', item->>'name',
            'title', item->>'title',
            'state', jsonb_build_object('kind', 'known'),
            'root', jsonb_build_object(
                'id', root_id,
                'component', 'ui.section',
                'state', jsonb_build_object('kind', 'known'),
                'properties', '{}'::jsonb,
                'events', '{}'::jsonb,
                'children', jsonb_build_array(
                    jsonb_build_object(
                        'id', title_id,
                        'component', 'ui.h1',
                        'state', jsonb_build_object('kind', 'known'),
                        'properties', '{}'::jsonb,
                        'content', jsonb_build_object('source', 'literal', 'value', item->>'title'),
                        'events', '{}'::jsonb,
                        'children', '[]'::jsonb,
                        'style', jsonb_build_object('responsive', '{}'::jsonb)
                    ),
                    jsonb_build_object(
                        'id', description_id,
                        'component', 'ui.p',
                        'state', jsonb_build_object('kind', 'known'),
                        'properties', '{}'::jsonb,
                        'content', jsonb_build_object(
                            'source', 'literal',
                            'value', '此页面已迁移为数据库 ProgramGraph，可在 Studio 中拖拽组件并绑定逻辑。'
                        ),
                        'events', '{}'::jsonb,
                        'children', '[]'::jsonb,
                        'style', jsonb_build_object('responsive', '{}'::jsonb)
                    )
                ),
                'style', jsonb_build_object('responsive', '{}'::jsonb)
            ),
            'page_state', '[]'::jsonb,
            'data_sources', '[]'::jsonb
        ));
        routes := routes || jsonb_build_array(jsonb_build_object(
            'id', route_id,
            'name', item->>'name',
            'path', item->>'route',
            'page_id', page_id,
            'state', jsonb_build_object('kind', 'known'),
            'required_permissions', '[]'::jsonb
        ));
        menu := jsonb_build_object(
            'id', menu_id,
            'name', item->>'name',
            'title', item->>'title',
            'state', jsonb_build_object('kind', 'known'),
            'page_id', page_id,
            'children', '[]'::jsonb,
            'required_permissions', '[]'::jsonb
        );
        CASE item->>'context'
            WHEN 'system' THEN system_menus := system_menus || jsonb_build_array(menu);
            WHEN 'resource' THEN resource_menus := resource_menus || jsonb_build_array(menu);
            WHEN 'infrastructure' THEN
                infrastructure_menus := infrastructure_menus || jsonb_build_array(menu);
        END CASE;
    END LOOP;

    FOR legacy_page IN
        SELECT page.id, page.page_key, page.definition
        FROM engine_page_definitions page
        WHERE page.state = 'published'
        ORDER BY page.page_key, page.id
    LOOP
        legacy_definition := legacy_page.definition::jsonb;
        IF COALESCE((legacy_definition ->> 'schema_version')::INTEGER, 0) <> 1
           OR jsonb_typeof(legacy_definition -> 'root') <> 'object' THEN
            RAISE EXCEPTION 'legacy PageDefinition cannot be converted: % (%)',
                legacy_page.page_key,
                legacy_page.id;
        END IF;

        page_id := (md5('aio.legacy.page:' || legacy_page.id))::uuid::text;
        route_id := (md5('aio.legacy.route:' || legacy_page.id))::uuid::text;
        menu_id := (md5('aio.legacy.menu:' || legacy_page.id))::uuid::text;
        legacy_route := '/pages/' || regexp_replace(
            lower(legacy_page.page_key),
            '[^a-z0-9_-]+',
            '-',
            'g'
        );

        pages := pages || jsonb_build_array(jsonb_build_object(
            'id', page_id,
            'name', legacy_page.page_key,
            'title', COALESCE(legacy_definition ->> 'title', legacy_page.page_key),
            'state', jsonb_build_object('kind', 'known'),
            'root', engine_convert_legacy_component(
                legacy_definition -> 'root',
                legacy_page.id || ':root'
            ),
            'page_state', '[]'::jsonb,
            'data_sources', '[]'::jsonb
        ));
        routes := routes || jsonb_build_array(jsonb_build_object(
            'id', route_id,
            'name', legacy_page.page_key,
            'path', legacy_route,
            'page_id', page_id,
            'state', jsonb_build_object('kind', 'known'),
            'required_permissions', '[]'::jsonb
        ));
        resource_menus := resource_menus || jsonb_build_array(jsonb_build_object(
            'id', menu_id,
            'name', legacy_page.page_key,
            'title', COALESCE(legacy_definition ->> 'title', legacy_page.page_key),
            'state', jsonb_build_object('kind', 'known'),
            'page_id', page_id,
            'children', '[]'::jsonb,
            'required_permissions', '[]'::jsonb
        ));
    END LOOP;

    definition := jsonb_build_object(
        'schema_version', 1,
        'id', program_id,
        'name', 'aio-first-party',
        'title', 'AIO 业务系统',
        'contexts', jsonb_build_array(
            jsonb_build_object(
                'id', system_context_id,
                'name', 'system',
                'title', '管理后台',
                'state', jsonb_build_object('kind', 'known'),
                'menus', system_menus
            ),
            jsonb_build_object(
                'id', resource_context_id,
                'name', 'resources',
                'title', '资源中心',
                'state', jsonb_build_object('kind', 'known'),
                'menus', resource_menus
            ),
            jsonb_build_object(
                'id', infrastructure_context_id,
                'name', 'infrastructure',
                'title', '基础设施',
                'state', jsonb_build_object('kind', 'known'),
                'menus', infrastructure_menus
            )
        ),
        'models', '[]'::jsonb,
        'pages', pages,
        'functions', '[]'::jsonb,
        'routes', routes,
        'permissions', '[]'::jsonb
    );

    INSERT INTO engine_applications
        (id, name, title, active_revision_id, created_at_ms, updated_at_ms)
    VALUES
        (seed_application_id, 'aio-first-party', 'AIO 业务系统', NULL, now_ms, now_ms)
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO engine_application_drafts
        (application_id, version, definition, updated_at_ms)
    VALUES
        (seed_application_id, 0, definition, now_ms)
    ON CONFLICT ON CONSTRAINT engine_application_drafts_pkey DO NOTHING;
END;
$$;

DROP FUNCTION engine_convert_legacy_component(JSONB, TEXT);

DROP TABLE engine_route_definitions;
DROP TABLE nature_application_deployments;
DROP TABLE nature_generation_events;
DROP TABLE nature_generation_runs;
DROP TABLE engine_field_bindings;
DROP TABLE nature_revisions;
DROP TABLE nature_projects;
DROP TABLE engine_operation_runs;
ALTER TABLE engine_operation_definitions DROP CONSTRAINT engine_operation_definitions_active_revision_fk;
DROP TABLE engine_operation_revisions;
DROP TABLE engine_operation_definitions;
DROP TABLE engine_page_definitions;
DROP TABLE engine_hook_definitions;
