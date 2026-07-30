-- 为管理后台补充系统管理目录，统一收纳原有系统页面。
CREATE FUNCTION engine_add_system_management_menu(definition JSONB)
RETURNS JSONB
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT jsonb_set(
        definition,
        '{menus}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN menu.value ->> 'name' = 'system'
                            OR menu.value ->> 'title' = '管理后台'
                        THEN menu.value || jsonb_build_object(
                            'children', jsonb_build_array(
                                jsonb_build_object(
                                    'id', md5('aio.menu:system-management')::uuid,
                                    'name', 'system-management',
                                    'title', '系统管理',
                                    'state', jsonb_build_object('kind', 'known'),
                                    'icon', '⚙',
                                    'page_id', NULL,
                                    'enabled', true,
                                    'children', COALESCE(menu.value -> 'children', '[]'::jsonb),
                                    'required_permissions', '[]'::jsonb,
                                    'row_actions', jsonb_build_object(
                                        'detail', jsonb_build_object('kind', 'hidden'),
                                        'edit', jsonb_build_object('kind', 'hidden'),
                                        'delete', jsonb_build_object('kind', 'hidden')
                                    )
                                )
                            )
                        )
                        WHEN menu.value ->> 'name' = 'system-management'
                            OR menu.value ->> 'title' = '系统管理'
                        THEN menu.value
                        ELSE menu.value
                    END
                    ORDER BY menu.ordinality
                )
                FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb))
                    WITH ORDINALITY AS menu(value, ordinality)
            ),
            '[]'::jsonb
        )
    );
$$;

-- 草稿直接切换到新的菜单树。
UPDATE engine_application_drafts
SET definition = engine_add_system_management_menu(definition)
WHERE definition ->> 'name' = 'aio-first-party'
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb)) AS menu(value),
           jsonb_array_elements(COALESCE(menu.value -> 'children', '[]'::jsonb)) AS child(value)
      WHERE child.value ->> 'name' = 'system-management'
         OR child.value ->> 'title' = '系统管理'
  );

-- 已发布 revision 需要同步迁移；触发器暂时解除以更新不可变快照。
DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;

UPDATE engine_application_revisions
SET definition = engine_add_system_management_menu(definition),
    content_hash = 'migrated-system-management:' || md5(engine_add_system_management_menu(definition)::text)
WHERE definition ->> 'name' = 'aio-first-party'
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements(COALESCE(definition -> 'menus', '[]'::jsonb)) AS menu(value),
           jsonb_array_elements(COALESCE(menu.value -> 'children', '[]'::jsonb)) AS child(value)
      WHERE child.value ->> 'name' = 'system-management'
         OR child.value ->> 'title' = '系统管理'
  );

CREATE TRIGGER engine_application_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_application_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_add_system_management_menu(JSONB);
