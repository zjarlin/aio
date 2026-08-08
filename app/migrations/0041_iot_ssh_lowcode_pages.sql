CREATE FUNCTION engine_domain_lowcode_field(model_name TEXT, spec JSONB)
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
        'required', COALESCE((spec ->> 'required')::BOOLEAN, false),
        'options', jsonb_strip_nulls(jsonb_build_object(
            'list_visible', COALESCE((spec ->> 'list')::BOOLEAN, true),
            'detail_visible', COALESCE((spec ->> 'detail')::BOOLEAN, true),
            'form_visible', COALESCE((spec ->> 'form')::BOOLEAN, true),
            'form_editable', COALESCE((spec ->> 'edit')::BOOLEAN, true),
            'filterable', COALESCE((spec ->> 'filter')::BOOLEAN, false),
            'sortable', COALESCE((spec ->> 'sort')::BOOLEAN, false),
            'unique', COALESCE((spec ->> 'unique')::BOOLEAN, false),
            'excel_import', COALESCE((spec ->> 'import')::BOOLEAN, true),
            'excel_export', COALESCE((spec ->> 'export')::BOOLEAN, true),
            'ai_extract', COALESCE((spec ->> 'ai')::BOOLEAN, true),
            'default_value', spec -> 'default',
            'placeholder', spec ->> 'placeholder',
            'help_text', spec ->> 'help',
            'validation', COALESCE(spec -> 'validation', '{}'::jsonb)
        ))
    );
$$;

CREATE FUNCTION engine_domain_lowcode_model(spec JSONB)
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
                    engine_domain_lowcode_field(spec ->> 'model', field.value)
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

CREATE FUNCTION engine_domain_lowcode_endpoint(page_name TEXT, spec JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_build_object(
        'id', md5(
            'aio.endpoint:' || page_name || ':' || (spec ->> 'method') || ':' || (spec ->> 'path')
        )::uuid,
        'title', spec ->> 'title',
        'state', jsonb_build_object('kind', 'known'),
        'method', spec ->> 'method',
        'path', spec ->> 'path',
        'inputs', COALESCE(
            (
                SELECT jsonb_agg(
                    jsonb_build_object(
                        'id', md5(
                            'aio.endpoint.input:' || page_name || ':' ||
                            (spec ->> 'method') || ':' || (spec ->> 'path') || ':' ||
                            (input.value ->> 'name')
                        )::uuid,
                        'name', input.value ->> 'name',
                        'title', input.value ->> 'title',
                        'location', COALESCE(input.value ->> 'location', 'body'),
                        'value_type', jsonb_build_object(
                            'kind', COALESCE(NULLIF(input.value ->> 'kind', ''), 'text')
                        ),
                        'required', COALESCE((input.value ->> 'required')::BOOLEAN, false)
                    )
                    ORDER BY input.ordinality
                )
                FROM jsonb_array_elements(COALESCE(spec -> 'inputs', '[]'::jsonb))
                    WITH ORDINALITY AS input(value, ordinality)
            ),
            '[]'::jsonb
        ),
        'outputs', '[]'::jsonb
    );
$$;

CREATE FUNCTION engine_domain_lowcode_patch_menu(
    menu JSONB,
    target_name TEXT,
    actions JSONB,
    child_menu JSONB
)
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
            engine_domain_lowcode_patch_menu(child.value, target_name, actions, child_menu)
            ORDER BY child.ordinality
        ),
        '[]'::jsonb
    )
    INTO children
    FROM jsonb_array_elements(COALESCE(menu -> 'children', '[]'::jsonb))
        WITH ORDINALITY AS child(value, ordinality);

    result := jsonb_set(menu, '{children}', children);
    IF menu ->> 'name' <> target_name THEN
        RETURN result;
    END IF;

    IF actions IS NOT NULL THEN
        result := result || jsonb_build_object('row_actions', actions);
    END IF;
    IF child_menu IS NOT NULL
       AND NOT EXISTS (
           SELECT 1
           FROM jsonb_array_elements(children) AS child(value)
           WHERE child.value ->> 'name' = child_menu ->> 'name'
       )
    THEN
        result := jsonb_set(result, '{children}', children || jsonb_build_array(child_menu));
    END IF;
    RETURN result;
END;
$$;

CREATE FUNCTION engine_domain_lowcode_patch_menus(
    menus JSONB,
    target_name TEXT,
    actions JSONB,
    child_menu JSONB
)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT COALESCE(
        jsonb_agg(
            engine_domain_lowcode_patch_menu(menu.value, target_name, actions, child_menu)
            ORDER BY menu.ordinality
        ),
        '[]'::jsonb
    )
    FROM jsonb_array_elements(COALESCE(menus, '[]'::jsonb))
        WITH ORDINALITY AS menu(value, ordinality);
$$;

CREATE FUNCTION engine_domain_lowcode_pages(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    model_specs CONSTANT JSONB := '[
      {
        "model":"iot_product","title":"物联网产品","fields":[
          {"name":"code","title":"产品编码","required":true,"filter":true,"unique":true},
          {"name":"name","title":"产品名称","required":true,"filter":true},
          {"name":"category","title":"产品分类","required":true,"filter":true},
          {"name":"protocol","title":"接入协议","required":true,"filter":true},
          {"name":"enabled","title":"启用","kind":"boolean","required":true,"filter":true,"default":true}
        ]
      },
      {
        "model":"iot_gateway","title":"边缘网关","fields":[
          {"name":"code","title":"网关编码","required":true,"filter":true,"unique":true},
          {"name":"name","title":"网关名称","required":true,"filter":true},
          {"name":"mqtt_client_id","title":"MQTT ClientId","required":true,"filter":true,"unique":true},
          {"name":"connected","title":"连接状态","kind":"boolean","required":true,"filter":true,"default":false},
          {"name":"last_seen_at_ms","title":"最后消息","kind":"timestamp_ms","sort":true,"default":0},
          {"name":"last_heartbeat_at_ms","title":"最后心跳","kind":"timestamp_ms","sort":true,"default":0},
          {"name":"expected_heartbeat_secs","title":"心跳周期秒","kind":"integer","required":true,"default":30,"validation":{"minimum":1}},
          {"name":"location","title":"安装位置"},
          {"name":"enabled","title":"启用","kind":"boolean","required":true,"filter":true,"default":true}
        ]
      },
      {
        "model":"iot_device","title":"物联网设备","fields":[
          {"name":"device_code","title":"设备编码","required":true,"filter":true,"unique":true},
          {"name":"name","title":"设备名称","required":true,"filter":true},
          {"name":"product_code","title":"产品编码","required":true,"filter":true},
          {"name":"product_name","title":"产品名称","required":true},
          {"name":"gateway_code","title":"网关编码","filter":true},
          {"name":"mqtt_client_id","title":"MQTT ClientId","required":true,"filter":true,"unique":true},
          {"name":"location","title":"安装位置"},
          {"name":"enabled","title":"启用","kind":"boolean","required":true,"filter":true,"default":true},
          {"name":"connected","title":"连接状态","kind":"boolean","required":true,"filter":true,"default":false},
          {"name":"last_seen_at_ms","title":"最后消息","kind":"timestamp_ms","list":false,"sort":true,"default":0},
          {"name":"last_heartbeat_at_ms","title":"最后心跳","kind":"timestamp_ms","list":false,"sort":true,"default":0},
          {"name":"last_data_at_ms","title":"最后数据","kind":"timestamp_ms","sort":true,"default":0},
          {"name":"expected_heartbeat_secs","title":"心跳周期秒","kind":"integer","required":true,"list":false,"default":30,"validation":{"minimum":1}},
          {"name":"expected_data_secs","title":"数据周期秒","kind":"integer","required":true,"list":false,"default":60,"validation":{"minimum":1}},
          {"name":"offline_reason","title":"离线原因","default":"尚未接入"}
        ]
      },
      {
        "model":"iot_telemetry","title":"设备遥测","fields":[
          {"name":"device_code","title":"设备编码","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"metric_code","title":"指标编码","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"value","title":"指标值","kind":"decimal","required":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"unit","title":"单位","form":false,"edit":false,"ai":false,"import":false},
          {"name":"quality","title":"数据质量","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"collected_at_ms","title":"采集时间","kind":"timestamp_ms","required":true,"sort":true,"form":false,"edit":false,"ai":false,"import":false}
        ]
      },
      {
        "model":"iot_alarm","title":"设备告警","fields":[
          {"name":"device_code","title":"设备编码","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"level","title":"告警等级","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"message","title":"告警内容","required":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"status","title":"处理状态","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"occurred_at_ms","title":"发生时间","kind":"timestamp_ms","required":true,"sort":true,"form":false,"edit":false,"ai":false,"import":false}
        ]
      },
      {
        "model":"ssh_target","title":"SSH 目标","fields":[
          {"name":"code","title":"目标编码","required":true,"filter":true,"unique":true},
          {"name":"name","title":"显示名称","required":true,"filter":true},
          {"name":"host","title":"主机名或 IP","required":true,"filter":true},
          {"name":"port","title":"SSH 端口","kind":"integer","required":true,"default":22,"validation":{"minimum":1,"maximum":65535}},
          {"name":"username","title":"登录用户","required":true,"filter":true},
          {"name":"auth_type","title":"认证方式","required":true,"filter":true,"default":"private_key"},
          {"name":"private_key_path","title":"私钥路径","list":false},
          {"name":"password_env","title":"密码环境变量","list":false},
          {"name":"passphrase_env","title":"私钥口令环境变量","list":false},
          {"name":"description","title":"备注","list":false},
          {"name":"enabled","title":"启用","kind":"boolean","required":true,"filter":true,"default":true}
        ]
      },
      {
        "model":"ssh_command","title":"SSH 监测命令","fields":[
          {"name":"code","title":"命令编码","required":true,"filter":true,"unique":true},
          {"name":"name","title":"命令名称","required":true,"filter":true},
          {"name":"category","title":"分类","required":true,"filter":true},
          {"name":"hardware_family","title":"硬件族","required":true,"filter":true},
          {"name":"detect_script","title":"适配探测脚本","list":false},
          {"name":"command_script","title":"执行脚本","required":true,"list":false},
          {"name":"kind","title":"命令类型","required":true,"filter":true,"default":"monitor"},
          {"name":"timeout_secs","title":"超时秒数","kind":"integer","required":true,"default":15,"validation":{"minimum":1,"maximum":3600}},
          {"name":"enabled","title":"启用","kind":"boolean","required":true,"filter":true,"default":true},
          {"name":"order_index","title":"排序","kind":"integer","required":true,"sort":true,"default":0}
        ]
      },
      {
        "model":"ssh_command_result","title":"SSH 最近执行结果","fields":[
          {"name":"target_code","title":"目标编码","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"target_name","title":"目标名称","required":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"command_code","title":"命令编码","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"command_name","title":"命令名称","required":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"category","title":"分类","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"hardware_family","title":"硬件族","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"status","title":"执行状态","required":true,"filter":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"exit_code","title":"退出码","kind":"integer","required":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"stdout","title":"标准输出","list":false,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"stderr","title":"标准错误","list":false,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"duration_ms","title":"耗时毫秒","kind":"integer","required":true,"sort":true,"form":false,"edit":false,"ai":false,"import":false},
          {"name":"collected_at_ms","title":"采集时间","kind":"timestamp_ms","required":true,"sort":true,"form":false,"edit":false,"ai":false,"import":false}
        ]
      }
    ]'::jsonb;
    page_specs CONSTANT JSONB := '[
      {
        "page":"iot","title":"物联网设备","model":"iot_device","route":"/iot","menu":"iot","page_size":20,
        "actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},
        "endpoints":[{
          "title":"新建设备","method":"POST","path":"/api/iot/devices","inputs":[
            {"name":"deviceCode","title":"设备编码","required":true},
            {"name":"name","title":"设备名称","required":true},
            {"name":"productCode","title":"产品编码","required":true},
            {"name":"productName","title":"产品名称","required":true},
            {"name":"gatewayCode","title":"网关编码"},
            {"name":"mqttClientId","title":"MQTT ClientId","required":true},
            {"name":"location","title":"安装位置"},
            {"name":"enabled","title":"启用","kind":"boolean"},
            {"name":"expectedHeartbeatSecs","title":"心跳周期秒","kind":"integer"},
            {"name":"expectedDataSecs","title":"数据周期秒","kind":"integer"}
          ]
        }]
      },
      {"page":"iot-products","title":"物联网产品","model":"iot_product","route":"/iot/products","parent_menu":"iot","menu":"iot-products","menu_title":"产品管理","page_size":20,"actions":{"detail":{"kind":"public"},"edit":{"kind":"public"},"delete":{"kind":"public"}},"endpoints":[]},
      {"page":"iot-gateways","title":"边缘网关","model":"iot_gateway","route":"/iot/gateways","parent_menu":"iot","menu":"iot-gateways","menu_title":"网关管理","page_size":20,"actions":{"detail":{"kind":"public"},"edit":{"kind":"public"},"delete":{"kind":"public"}},"endpoints":[]},
      {"page":"iot-telemetry","title":"设备遥测","model":"iot_telemetry","route":"/iot/telemetry","parent_menu":"iot","menu":"iot-telemetry","menu_title":"遥测数据","page_size":50,"actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},"endpoints":[]},
      {"page":"iot-alarms","title":"设备告警","model":"iot_alarm","route":"/iot/alarms","parent_menu":"iot","menu":"iot-alarms","menu_title":"设备告警","page_size":50,"actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},"endpoints":[]},
      {
        "page":"ssh","title":"SSH 目标","model":"ssh_target","route":"/ssh","menu":"ssh","page_size":20,
        "actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},
        "endpoints":[{
          "title":"保存 SSH 目标","method":"POST","path":"/api/ssh/targets","inputs":[
            {"name":"code","title":"目标编码","required":true},
            {"name":"name","title":"显示名称","required":true},
            {"name":"host","title":"主机名或 IP","required":true},
            {"name":"port","title":"SSH 端口","kind":"integer"},
            {"name":"username","title":"登录用户","required":true},
            {"name":"authType","title":"认证方式"},
            {"name":"privateKeyPath","title":"私钥路径"},
            {"name":"passwordEnv","title":"密码环境变量"},
            {"name":"passphraseEnv","title":"私钥口令环境变量"},
            {"name":"description","title":"备注"},
            {"name":"enabled","title":"启用","kind":"boolean"}
          ]
        }]
      },
      {
        "page":"ssh-commands","title":"SSH 监测命令","model":"ssh_command","route":"/ssh/commands","parent_menu":"ssh","menu":"ssh-commands","menu_title":"监测命令","page_size":20,
        "actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},
        "endpoints":[{
          "title":"保存 SSH 命令","method":"POST","path":"/api/ssh/commands","inputs":[
            {"name":"code","title":"命令编码","required":true},
            {"name":"name","title":"命令名称","required":true},
            {"name":"category","title":"分类","required":true},
            {"name":"hardwareFamily","title":"硬件族","required":true},
            {"name":"detectScript","title":"适配探测脚本"},
            {"name":"commandScript","title":"执行脚本","required":true},
            {"name":"kind","title":"命令类型"},
            {"name":"timeoutSecs","title":"超时秒数","kind":"integer"},
            {"name":"enabled","title":"启用","kind":"boolean"},
            {"name":"orderIndex","title":"排序","kind":"integer"}
          ]
        }]
      },
      {
        "page":"ssh-results","title":"SSH 执行结果","model":"ssh_command_result","route":"/ssh/results","parent_menu":"ssh","menu":"ssh-results","menu_title":"执行结果","page_size":50,
        "actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},
        "endpoints":[
          {"title":"采集监测项","method":"POST","path":"/api/ssh/collect","inputs":[{"name":"targetCode","title":"目标编码","required":true}]},
          {"title":"执行指定命令","method":"POST","path":"/api/ssh/execute","inputs":[{"name":"targetCode","title":"目标编码","required":true},{"name":"commandCode","title":"命令编码","required":true}]}
        ]
      },
      {"page":"device-overview","title":"设备运行概览","model":"iot_device","route":"/pages/device-overview","menu":"device-overview","page_size":50,"actions":{"detail":{"kind":"public"},"edit":{"kind":"hidden"},"delete":{"kind":"hidden"}},"endpoints":[]}
    ]'::jsonb;
    models JSONB := COALESCE(definition -> 'models', '[]'::jsonb);
    pages JSONB := COALESCE(definition -> 'pages', '[]'::jsonb);
    routes JSONB := COALESCE(definition -> 'routes', '[]'::jsonb);
    menus JSONB := COALESCE(definition -> 'menus', '[]'::jsonb);
    model_spec JSONB;
    page_spec JSONB;
    model_value JSONB;
    page_value JSONB;
    page_id TEXT;
    endpoints JSONB;
    child_menu JSONB;
BEGIN
    FOR model_spec IN SELECT value FROM jsonb_array_elements(model_specs)
    LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM jsonb_array_elements(models) AS model(value)
            WHERE model.value ->> 'name' = model_spec ->> 'model'
        ) THEN
            models := models || jsonb_build_array(engine_domain_lowcode_model(model_spec));
        END IF;
    END LOOP;

    FOR page_spec IN SELECT value FROM jsonb_array_elements(page_specs)
    LOOP
        SELECT model.value
        INTO model_value
        FROM jsonb_array_elements(models) AS model(value)
        WHERE model.value ->> 'name' = page_spec ->> 'model'
        LIMIT 1;

        SELECT COALESCE(
            jsonb_agg(
                engine_domain_lowcode_endpoint(page_spec ->> 'page', endpoint.value)
                ORDER BY endpoint.ordinality
            ),
            '[]'::jsonb
        )
        INTO endpoints
        FROM jsonb_array_elements(COALESCE(page_spec -> 'endpoints', '[]'::jsonb))
            WITH ORDINALITY AS endpoint(value, ordinality);

        SELECT page.value
        INTO page_value
        FROM jsonb_array_elements(pages) AS page(value)
        WHERE page.value ->> 'name' = page_spec ->> 'page'
        LIMIT 1;

        IF page_value IS NULL THEN
            page_value := jsonb_build_object(
                'id', md5('aio.page:' || (page_spec ->> 'page'))::uuid,
                'name', page_spec ->> 'page',
                'title', page_spec ->> 'title',
                'state', jsonb_build_object('kind', 'known'),
                'renderer', jsonb_build_object(
                    'kind', 'crud_table',
                    'table', jsonb_build_object(
                        'model_id', model_value ->> 'id',
                        'page_size', (page_spec ->> 'page_size')::INTEGER
                    )
                ),
                'endpoints', endpoints
            );
            pages := pages || jsonb_build_array(page_value);
        ELSE
            page_value := page_value || jsonb_build_object(
                'title', page_spec ->> 'title',
                'renderer', jsonb_build_object(
                    'kind', 'crud_table',
                    'table', jsonb_build_object(
                        'model_id', model_value ->> 'id',
                        'page_size', (page_spec ->> 'page_size')::INTEGER
                    )
                ),
                'endpoints', endpoints
            );
            SELECT jsonb_agg(
                CASE
                    WHEN page.value ->> 'name' = page_spec ->> 'page' THEN page_value
                    ELSE page.value
                END
                ORDER BY page.ordinality
            )
            INTO pages
            FROM jsonb_array_elements(pages) WITH ORDINALITY AS page(value, ordinality);
        END IF;

        page_id := page_value ->> 'id';
        IF NOT EXISTS (
            SELECT 1
            FROM jsonb_array_elements(routes) AS route(value)
            WHERE route.value ->> 'name' = page_spec ->> 'page'
               OR route.value ->> 'path' = page_spec ->> 'route'
        ) THEN
            routes := routes || jsonb_build_array(jsonb_build_object(
                'id', md5('aio.route:' || (page_spec ->> 'page'))::uuid,
                'name', page_spec ->> 'page',
                'path', page_spec ->> 'route',
                'page_id', page_id,
                'state', jsonb_build_object('kind', 'known'),
                'required_permissions', '[]'::jsonb
            ));
        END IF;

        IF page_spec ? 'parent_menu' THEN
            child_menu := jsonb_build_object(
                'id', md5('aio.menu:' || (page_spec ->> 'menu'))::uuid,
                'name', page_spec ->> 'menu',
                'title', page_spec ->> 'menu_title',
                'state', jsonb_build_object('kind', 'known'),
                'page_id', page_id,
                'enabled', true,
                'children', '[]'::jsonb,
                'required_permissions', '[]'::jsonb,
                'row_actions', page_spec -> 'actions'
            );
            menus := engine_domain_lowcode_patch_menus(
                menus,
                page_spec ->> 'parent_menu',
                NULL,
                child_menu
            );
        ELSE
            menus := engine_domain_lowcode_patch_menus(
                menus,
                page_spec ->> 'menu',
                page_spec -> 'actions',
                NULL
            );
        END IF;

        model_value := NULL;
        page_value := NULL;
        child_menu := NULL;
    END LOOP;

    RETURN jsonb_set(
        jsonb_set(
            jsonb_set(
                jsonb_set(definition, '{models}', models),
                '{pages}', pages
            ),
            '{routes}', routes
        ),
        '{menus}', menus
    );
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_domain_lowcode_pages(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_domain_lowcode_pages(definition),
    content_hash = 'iot-ssh-lowcode-pages:' || md5(
        engine_domain_lowcode_pages(definition)::TEXT
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

DROP FUNCTION engine_domain_lowcode_pages(JSONB);
DROP FUNCTION engine_domain_lowcode_patch_menus(JSONB, TEXT, JSONB, JSONB);
DROP FUNCTION engine_domain_lowcode_patch_menu(JSONB, TEXT, JSONB, JSONB);
DROP FUNCTION engine_domain_lowcode_endpoint(TEXT, JSONB);
DROP FUNCTION engine_domain_lowcode_model(JSONB);
DROP FUNCTION engine_domain_lowcode_field(TEXT, JSONB);
