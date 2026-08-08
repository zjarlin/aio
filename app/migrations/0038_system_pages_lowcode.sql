CREATE FUNCTION engine_system_lowcode_field(model_name TEXT, spec JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_build_object(
        'id', md5('aio.field:' || model_name || ':' || (spec ->> 'name'))::uuid,
        'name', spec ->> 'name',
        'title', spec ->> 'title',
        'value_type', jsonb_build_object(
            'kind', COALESCE(NULLIF(spec ->> 'kind', ''), 'text')
        ),
        'state', jsonb_build_object('kind', 'known'),
        'required', COALESCE((spec ->> 'required')::boolean, false),
        'options', jsonb_build_object(
            'list_visible', true,
            'detail_visible', true,
            'form_visible', true,
            'form_editable', true,
            'filterable', COALESCE((spec ->> 'filterable')::boolean, false),
            'sortable', false,
            'unique', false,
            'excel_import', true,
            'excel_export', true,
            'ai_extract', true,
            'validation', '{}'::jsonb
        )
    );
$$;

CREATE FUNCTION engine_system_lowcode_model(spec JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_build_object(
        'id', md5('aio.model:' || (spec ->> 'model'))::uuid,
        'name', spec ->> 'model',
        'title', spec ->> 'title',
        'state', jsonb_build_object('kind', 'known'),
        'fields', COALESCE(
            (
                SELECT jsonb_agg(
                    engine_system_lowcode_field(spec ->> 'model', field.value)
                    ORDER BY field.ordinality
                )
                FROM jsonb_array_elements(COALESCE(spec -> 'fields', '[]'::jsonb))
                    WITH ORDINALITY AS field(value, ordinality)
            ),
            '[]'::jsonb
        ),
        'indexes', '[]'::jsonb,
        'queries', '[]'::jsonb,
        'validations', '[]'::jsonb,
        'audit', jsonb_build_object('fields', '[]'::jsonb)
    );
$$;

CREATE FUNCTION engine_system_pages_lowcode(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    specs CONSTANT JSONB := '[
      {
        "page":"api-keys","model":"api_key","title":"API 密钥","aliases":[],
        "fields":[
          {"name":"name","title":"密钥名称","required":true,"filterable":true},
          {"name":"prefix","title":"密钥前缀","required":true},
          {"name":"scope","title":"授权范围","required":true,"filterable":true},
          {"name":"status","title":"状态","required":true,"filterable":true},
          {"name":"last_used_at","title":"最近使用"}
        ]
      },
      {
        "page":"users","model":"user","title":"用户","aliases":[],
        "fields":[
          {"name":"username","title":"用户名","required":true,"filterable":true},
          {"name":"display_name","title":"姓名","required":true,"filterable":true},
          {"name":"email","title":"邮箱"},
          {"name":"status","title":"状态","required":true,"filterable":true},
          {"name":"department","title":"部门","filterable":true}
        ]
      },
      {
        "page":"roles","model":"role","title":"角色","aliases":[],
        "fields":[
          {"name":"name","title":"角色名称","required":true,"filterable":true},
          {"name":"code","title":"角色标识","required":true,"filterable":true},
          {"name":"scope","title":"数据权限","required":true,"filterable":true},
          {"name":"status","title":"状态","required":true,"filterable":true}
        ]
      },
      {
        "page":"departments","model":"dept","title":"部门","aliases":["department"],
        "fields":[
          {"name":"name","title":"部门名称","required":true,"filterable":true},
          {"name":"code","title":"部门编码","required":true,"filterable":true},
          {"name":"parent_id","title":"上级部门"},
          {"name":"leader","title":"负责人"},
          {"name":"status","title":"状态","required":true,"filterable":true}
        ]
      },
      {
        "page":"dictionary","model":"dictionary","title":"字典","aliases":[],
        "fields":[
          {"name":"name","title":"字典类型","required":true,"filterable":true},
          {"name":"code","title":"编码","required":true,"filterable":true},
          {"name":"items","title":"条目数","kind":"integer"},
          {"name":"scope","title":"作用域","filterable":true},
          {"name":"updated_at","title":"更新时间"}
        ]
      },
      {
        "page":"menus","model":"menu_binding","title":"菜单挂载","aliases":[],
        "fields":[
          {"name":"label","title":"菜单节点","required":true,"filterable":true},
          {"name":"route","title":"路由","required":true,"filterable":true},
          {"name":"permission","title":"权限"},
          {"name":"kind","title":"类型","filterable":true},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"audit","model":"audit_event","title":"审计事件","aliases":[],
        "fields":[
          {"name":"event","title":"事件","required":true,"filterable":true},
          {"name":"actor","title":"操作者","filterable":true},
          {"name":"target","title":"对象","filterable":true},
          {"name":"result","title":"结果","filterable":true},
          {"name":"created_at","title":"发生时间"}
        ]
      },
      {
        "page":"sessions","model":"auth_session","title":"认证会话","aliases":[],
        "fields":[
          {"name":"flow","title":"认证流","required":true,"filterable":true},
          {"name":"entry","title":"入口"},
          {"name":"token","title":"令牌模型"},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"tenants","model":"tenant","title":"租户","aliases":[],
        "fields":[
          {"name":"name","title":"租户","required":true,"filterable":true},
          {"name":"package","title":"套餐","filterable":true},
          {"name":"users","title":"用户数","kind":"integer"},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"messages","model":"message_template","title":"消息模板","aliases":[],
        "fields":[
          {"name":"template","title":"模板","required":true,"filterable":true},
          {"name":"channel","title":"通道","required":true,"filterable":true},
          {"name":"sent","title":"发送量","kind":"integer"},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"oauth-clients","model":"oauth_client","title":"OAuth2 客户端","aliases":[],
        "fields":[
          {"name":"client","title":"客户端","required":true,"filterable":true},
          {"name":"grant_types","title":"授权模式"},
          {"name":"redirect_uri","title":"回调地址"},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"social-clients","model":"social_client","title":"社交客户端","aliases":[],
        "fields":[
          {"name":"platform","title":"平台","required":true,"filterable":true},
          {"name":"client","title":"客户端","required":true,"filterable":true},
          {"name":"bindings","title":"绑定数","kind":"integer"},
          {"name":"status","title":"状态","filterable":true}
        ]
      },
      {
        "page":"areas","model":"area","title":"地区","aliases":[],
        "fields":[
          {"name":"name","title":"地区名称","required":true,"filterable":true},
          {"name":"code","title":"地区编码","required":true,"filterable":true},
          {"name":"parent_code","title":"上级编码"},
          {"name":"level","title":"层级","kind":"integer","filterable":true},
          {"name":"status","title":"状态","filterable":true}
        ]
      }
    ]'::jsonb;
    models JSONB := COALESCE(definition -> 'models', '[]'::jsonb);
    pages JSONB := '[]'::jsonb;
    spec JSONB;
    model_value JSONB;
    page_value JSONB;
BEGIN
    FOR spec IN SELECT value FROM jsonb_array_elements(specs)
    LOOP
        SELECT model.value
        INTO model_value
        FROM jsonb_array_elements(models) AS model(value)
        WHERE model.value ->> 'name' = spec ->> 'model'
           OR model.value ->> 'name' IN (
                SELECT jsonb_array_elements_text(COALESCE(spec -> 'aliases', '[]'::jsonb))
           )
        LIMIT 1;

        IF model_value IS NULL THEN
            models := models || jsonb_build_array(engine_system_lowcode_model(spec));
        END IF;
    END LOOP;

    FOR page_value IN
        SELECT value
        FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb))
    LOOP
        SELECT value
        INTO spec
        FROM jsonb_array_elements(specs)
        WHERE value ->> 'page' = page_value ->> 'name'
        LIMIT 1;

        IF spec IS NOT NULL THEN
            SELECT model.value
            INTO model_value
            FROM jsonb_array_elements(models) AS model(value)
            WHERE model.value ->> 'name' = spec ->> 'model'
               OR model.value ->> 'name' IN (
                    SELECT jsonb_array_elements_text(COALESCE(spec -> 'aliases', '[]'::jsonb))
               )
            LIMIT 1;

            page_value := page_value || jsonb_build_object(
                'renderer', jsonb_build_object(
                    'kind', 'crud_table',
                    'table', jsonb_build_object(
                        'model_id', model_value ->> 'id',
                        'page_size', 20
                    )
                )
            );
        END IF;

        pages := pages || jsonb_build_array(page_value);
    END LOOP;

    RETURN jsonb_set(
        jsonb_set(definition, '{models}', models),
        '{pages}',
        pages
    );
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_system_pages_lowcode(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_system_pages_lowcode(definition),
    content_hash = 'migrated-system-lowcode:' || md5(
        engine_system_pages_lowcode(definition)::TEXT
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

DROP FUNCTION engine_system_pages_lowcode(JSONB);
DROP FUNCTION engine_system_lowcode_model(JSONB);
DROP FUNCTION engine_system_lowcode_field(TEXT, JSONB);
