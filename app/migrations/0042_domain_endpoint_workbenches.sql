CREATE FUNCTION engine_endpoint_workbench_endpoint(page_name TEXT, spec JSONB)
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

CREATE FUNCTION engine_endpoint_workbench_pages(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    page_specs CONSTANT JSONB := '[
      {
        "page":"algorithms",
        "endpoints":[
          {"title":"运行状态","method":"GET","path":"/api/algorithm-center/status"},
          {"title":"算法组件","method":"GET","path":"/api/algorithm-center/components"},
          {
            "title":"处理视频","method":"POST","path":"/api/algorithm-center/process",
            "inputs":[
              {"name":"video_url","title":"视频地址","required":true},
              {"name":"algorithms","title":"算法编码数组","kind":"any"}
            ]
          }
        ]
      },
      {
        "page":"drive",
        "endpoints":[
          {"title":"运行状态","method":"GET","path":"/api/drive-center/status"},
          {"title":"网盘任务","method":"GET","path":"/api/drive-center/tasks"},
          {
            "title":"新建网盘任务","method":"POST","path":"/api/drive-center/task",
            "inputs":[
              {"name":"id","title":"任务 ID"},
              {"name":"path","title":"网盘路径","required":true},
              {"name":"action","title":"执行动作","required":true},
              {"name":"status","title":"初始状态"}
            ]
          }
        ]
      },
      {
        "page":"software",
        "endpoints":[
          {"title":"运行状态","method":"GET","path":"/api/software-center/status"},
          {"title":"本机安装包","method":"GET","path":"/api/software-center/installers"},
          {"title":"软件包记录","method":"GET","path":"/api/software-center/packages"},
          {"title":"整理安装包","method":"POST","path":"/api/software-center/organize"},
          {
            "title":"保存软件包","method":"POST","path":"/api/software-center/package",
            "inputs":[
              {"name":"id","title":"软件包 ID"},
              {"name":"name","title":"软件名称","required":true},
              {"name":"source_path","title":"来源路径","required":true},
              {"name":"platform","title":"操作系统","required":true},
              {"name":"arch","title":"处理器架构","required":true},
              {"name":"status","title":"状态"}
            ]
          }
        ]
      }
    ]'::jsonb;
    pages JSONB := COALESCE(definition -> 'pages', '[]'::jsonb);
    page_spec JSONB;
    endpoints JSONB;
BEGIN
    FOR page_spec IN SELECT value FROM jsonb_array_elements(page_specs)
    LOOP
        SELECT COALESCE(
            jsonb_agg(
                engine_endpoint_workbench_endpoint(page_spec ->> 'page', endpoint.value)
                ORDER BY endpoint.ordinality
            ),
            '[]'::jsonb
        )
        INTO endpoints
        FROM jsonb_array_elements(page_spec -> 'endpoints')
            WITH ORDINALITY AS endpoint(value, ordinality);

        SELECT jsonb_agg(
            CASE
                WHEN page.value ->> 'name' = page_spec ->> 'page'
                    THEN page.value || jsonb_build_object('endpoints', endpoints)
                ELSE page.value
            END
            ORDER BY page.ordinality
        )
        INTO pages
        FROM jsonb_array_elements(pages) WITH ORDINALITY AS page(value, ordinality);
    END LOOP;

    RETURN jsonb_set(definition, '{pages}', pages);
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_endpoint_workbench_pages(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_endpoint_workbench_pages(definition),
    content_hash = 'domain-endpoint-workbenches:' || md5(
        engine_endpoint_workbench_pages(definition)::TEXT
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

DROP FUNCTION engine_endpoint_workbench_pages(JSONB);
DROP FUNCTION engine_endpoint_workbench_endpoint(TEXT, JSONB);
