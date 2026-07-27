UPDATE nature_revisions
SET blueprint_json = ''
WHERE blueprint_json <> ''
  AND NOT (blueprint_json::jsonb ? 'application');
