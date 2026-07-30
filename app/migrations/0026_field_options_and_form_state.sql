CREATE FUNCTION engine_program_v6(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    model_item RECORD;
    field_item RECORD;
    page_item RECORD;
    model_value JSONB;
    field_value JSONB;
    page_value JSONB;
    renderer_value JSONB;
    models_value JSONB := '[]'::JSONB;
    fields_value JSONB;
    pages_value JSONB := '[]'::JSONB;
    model_id TEXT;
    field_id TEXT;
    has_explicit_columns BOOLEAN;
    list_visible BOOLEAN;
    filterable BOOLEAN;
BEGIN
    FOR model_item IN
        SELECT value, ordinality
        FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::JSONB))
            WITH ORDINALITY
        ORDER BY ordinality
    LOOP
        model_value := model_item.value;
        model_id := model_value ->> 'id';
        fields_value := '[]'::JSONB;
        FOR field_item IN
            SELECT value, ordinality
            FROM jsonb_array_elements(COALESCE(model_value -> 'fields', '[]'::JSONB))
                WITH ORDINALITY
            ORDER BY ordinality
        LOOP
            field_value := field_item.value;
            field_id := field_value ->> 'id';
            SELECT EXISTS (
                SELECT 1
                FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB)) page
                WHERE page -> 'renderer' -> 'table' ->> 'model_id' = model_id
                  AND jsonb_array_length(
                      COALESCE(page -> 'renderer' -> 'table' -> 'columns', '[]'::JSONB)
                  ) > 0
            ) INTO has_explicit_columns;
            IF has_explicit_columns THEN
                SELECT EXISTS (
                    SELECT 1
                    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB)) page,
                         jsonb_array_elements_text(
                             COALESCE(page -> 'renderer' -> 'table' -> 'columns', '[]'::JSONB)
                         ) column_id
                    WHERE page -> 'renderer' -> 'table' ->> 'model_id' = model_id
                      AND column_id = field_id
                ) INTO list_visible;
            ELSE
                list_visible := TRUE;
            END IF;
            SELECT EXISTS (
                SELECT 1
                FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB)) page,
                     jsonb_array_elements_text(
                         COALESCE(page -> 'renderer' -> 'table' -> 'filters', '[]'::JSONB)
                     ) filter_id
                WHERE page -> 'renderer' -> 'table' ->> 'model_id' = model_id
                  AND filter_id = field_id
            ) INTO filterable;
            field_value := field_value || jsonb_build_object(
                'options',
                jsonb_build_object(
                    'list_visible', list_visible,
                    'detail_visible', TRUE,
                    'form_visible', TRUE,
                    'form_editable', TRUE,
                    'filterable', filterable,
                    'sortable', FALSE,
                    'unique', FALSE,
                    'excel_import', TRUE,
                    'excel_export', TRUE,
                    'ai_extract', TRUE,
                    'validation', '{}'::JSONB
                )
            );
            fields_value := fields_value || jsonb_build_array(field_value);
        END LOOP;
        model_value := jsonb_set(model_value, '{fields}', fields_value);
        models_value := models_value || jsonb_build_array(model_value);
    END LOOP;

    FOR page_item IN
        SELECT value, ordinality
        FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::JSONB))
            WITH ORDINALITY
        ORDER BY ordinality
    LOOP
        page_value := page_item.value;
        renderer_value := page_value -> 'renderer';
        IF renderer_value ? 'table' THEN
            renderer_value := jsonb_set(
                renderer_value,
                '{table}',
                (renderer_value -> 'table') - 'columns' - 'filters'
            );
            page_value := jsonb_set(page_value, '{renderer}', renderer_value);
        END IF;
        pages_value := pages_value || jsonb_build_array(page_value);
    END LOOP;

    RETURN jsonb_set(
        jsonb_set(
            jsonb_set(definition, '{models}', models_value),
            '{pages}', pages_value
        ),
        '{schema_version}', '6'::JSONB
    );
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_program_v6(definition)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 6;

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_program_v6(definition),
    content_hash = 'migrated-v6:' || md5(engine_program_v6(definition)::TEXT)
WHERE COALESCE((definition ->> 'schema_version')::INTEGER, 0) < 6;

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_program_v6(JSONB);
