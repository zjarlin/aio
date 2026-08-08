CREATE FUNCTION engine_remove_unreferenced_menu_binding(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    menu_binding_id TEXT;
    retained_models JSONB;
    updated_definition JSONB;
BEGIN
    SELECT model.value ->> 'id'
    INTO menu_binding_id
    FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::jsonb)) AS model(value)
    WHERE model.value ->> 'name' = 'menu_binding'
    LIMIT 1;

    IF menu_binding_id IS NULL THEN
        RETURN definition;
    END IF;

    SELECT COALESCE(jsonb_agg(model.value ORDER BY model.ordinality), '[]'::jsonb)
    INTO retained_models
    FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::jsonb))
        WITH ORDINALITY AS model(value, ordinality)
    WHERE model.value ->> 'id' <> menu_binding_id;

    updated_definition := jsonb_set(definition, '{models}', retained_models);
    IF updated_definition::TEXT LIKE '%' || menu_binding_id || '%' THEN
        RAISE EXCEPTION 'menu_binding 仍被 ProgramDefinition 引用: %', menu_binding_id;
    END IF;

    RETURN updated_definition;
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_remove_unreferenced_menu_binding(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party'
  AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::jsonb)) AS model(value)
      WHERE model.value ->> 'name' = 'menu_binding'
  );

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_remove_unreferenced_menu_binding(definition),
    content_hash = 'removed-menu-binding:' || md5(
        engine_remove_unreferenced_menu_binding(definition)::TEXT
    )
WHERE definition ->> 'name' = 'aio-first-party'
  AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements(COALESCE(definition -> 'models', '[]'::jsonb)) AS model(value)
      WHERE model.value ->> 'name' = 'menu_binding'
  );

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_remove_unreferenced_menu_binding(JSONB);
