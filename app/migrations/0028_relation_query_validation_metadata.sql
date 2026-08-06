CREATE FUNCTION engine_program_v8(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        jsonb_set(
            definition,
            '{models}',
            COALESCE(
                (
                    SELECT jsonb_agg(
                        (
                            model.value || jsonb_build_object(
                                'fields',
                                COALESCE(
                                    (
                                        SELECT jsonb_agg(
                                            (
                                                field.value - 'relation_model_id'
                                            ) || CASE
                                                WHEN field.value ? 'relation_model_id'
                                                     AND field.value -> 'relation_model_id' <> 'null'::JSONB
                                                THEN jsonb_build_object(
                                                    'state',
                                                    jsonb_build_object(
                                                        'kind', 'invalid',
                                                        'reason', '旧关联缺少对端字段和基数，必须重新配置'
                                                    )
                                                )
                                                ELSE '{}'::JSONB
                                            END || jsonb_build_object(
                                                'options',
                                                COALESCE(field.value -> 'options', '{}'::JSONB) || jsonb_build_object(
                                                    'validation',
                                                    COALESCE(
                                                        field.value -> 'options' -> 'validation',
                                                        '{}'::JSONB
                                                    ) || jsonb_build_object('unique_items', false)
                                                )
                                            )
                                        )
                                        FROM jsonb_array_elements(
                                            COALESCE(model.value -> 'fields', '[]'::JSONB)
                                        ) AS field(value)
                                    ),
                                    '[]'::JSONB
                                ),
                                'indexes',
                                COALESCE(
                                    (
                                        SELECT jsonb_agg(
                                            (item.value - 'purpose') || jsonb_build_object(
                                                'unique',
                                                COALESCE(item.value -> 'unique', 'false'::JSONB)
                                            )
                                        )
                                        FROM jsonb_array_elements(
                                            COALESCE(model.value -> 'indexes', '[]'::JSONB)
                                        ) AS item(value)
                                    ),
                                    '[]'::JSONB
                                ),
                                'queries',
                                COALESCE(model.value -> 'queries', '[]'::JSONB),
                                'validations',
                                COALESCE(model.value -> 'validations', '[]'::JSONB)
                            )
                        )
                        ORDER BY model.ordinality
                    )
                    FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::JSONB))
                        WITH ORDINALITY AS model(value, ordinality)
                ),
                '[]'::JSONB
            )
        ),
        '{schema_version}',
        '8'::JSONB
    );
$$;

CREATE FUNCTION engine_complete_user_department_relation(definition JSONB)
RETURNS JSONB
LANGUAGE PLPGSQL
IMMUTABLE
AS $$
DECLARE
    department_model_id UUID := md5('aio.model:department')::uuid;
    department_users_id UUID := md5('aio.field:department:users')::uuid;
    user_model_id UUID := md5('aio.model:user')::uuid;
    user_department_id UUID := md5('aio.field:user:department_id')::uuid;
    model_value JSONB;
    fields_value JSONB;
    models_value JSONB := '[]'::JSONB;
BEGIN
    FOR model_value IN
        SELECT value
        FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::JSONB))
    LOOP
        SELECT COALESCE(
            jsonb_agg(
                CASE
                    WHEN model_value ->> 'id' = user_model_id::TEXT
                         AND field.value ->> 'id' = user_department_id::TEXT
                    THEN field.value || jsonb_build_object(
                        'state', jsonb_build_object('kind', 'known'),
                        'value_type', jsonb_build_object(
                            'kind', 'object',
                            'model_id', department_model_id
                        ),
                        'relation', jsonb_build_object(
                            'kind', 'many_to_one',
                            'target_model_id', department_model_id,
                            'target_field_id', department_users_id
                        )
                    )
                    ELSE field.value
                END
            ),
            '[]'::JSONB
        )
        INTO fields_value
        FROM jsonb_array_elements(COALESCE(model_value -> 'fields', '[]'::JSONB)) AS field(value);

        IF model_value ->> 'id' = department_model_id::TEXT
           AND NOT EXISTS (
               SELECT 1
               FROM jsonb_array_elements(fields_value) AS field(value)
               WHERE field.value ->> 'id' = department_users_id::TEXT
           ) THEN
            fields_value := fields_value || jsonb_build_array(
                jsonb_build_object(
                    'id', department_users_id,
                    'name', 'users',
                    'title', '用户',
                    'value_type', jsonb_build_object(
                        'kind', 'list',
                        'item', jsonb_build_object(
                            'kind', 'object',
                            'model_id', user_model_id
                        )
                    ),
                    'state', jsonb_build_object('kind', 'known'),
                    'required', false,
                    'options', jsonb_build_object(
                        'list_visible', true,
                        'detail_visible', true,
                        'form_visible', false,
                        'form_editable', false,
                        'filterable', false,
                        'sortable', false,
                        'unique', false,
                        'excel_import', false,
                        'excel_export', true,
                        'ai_extract', false,
                        'validation', jsonb_build_object('unique_items', true)
                    ),
                    'relation', jsonb_build_object(
                        'kind', 'one_to_many',
                        'target_model_id', user_model_id,
                        'target_field_id', user_department_id
                    )
                )
            );
        END IF;
        models_value := models_value || jsonb_build_array(
            model_value || jsonb_build_object('fields', fields_value)
        );
    END LOOP;
    RETURN jsonb_set(definition, '{models}', models_value);
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_complete_user_department_relation(engine_program_v8(definition))
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 8;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_complete_user_department_relation(engine_program_v8(definition)),
    content_hash = 'migrated-v8:' || md5(
        engine_complete_user_department_relation(engine_program_v8(definition))::TEXT
    )
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 8;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v8(JSONB);
DROP FUNCTION engine_complete_user_department_relation(JSONB);
