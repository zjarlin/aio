CREATE TABLE sys_dept (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES sys_dept (id) ON DELETE RESTRICT,
    leader TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    sort_index BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE UNIQUE INDEX sys_dept_code_uq ON sys_dept (code);
CREATE INDEX sys_dept_parent_id_idx ON sys_dept (parent_id);
CREATE INDEX sys_dept_status_idx ON sys_dept (status);

INSERT INTO sys_dept (
    id,
    code,
    name,
    parent_id,
    leader,
    status,
    sort_index,
    created_at_ms,
    updated_at_ms
)
SELECT
    md5('aio.system.department:' || lower(trim(department))),
    'legacy-' || substr(md5(lower(trim(department))), 1, 16),
    trim(department),
    NULL,
    '',
    'active',
    row_number() OVER (ORDER BY lower(trim(department))) * 10,
    (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT,
    (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
FROM (
    SELECT DISTINCT department
    FROM sys_user
    WHERE trim(department) <> ''
) legacy_departments;

ALTER TABLE sys_user ADD COLUMN department_id TEXT;

UPDATE sys_user AS users
SET department_id = departments.id
FROM sys_dept AS departments
WHERE lower(trim(users.department)) = lower(departments.name)
  AND trim(users.department) <> '';

ALTER TABLE sys_user
    ADD CONSTRAINT sys_user_department_id_fk
    FOREIGN KEY (department_id) REFERENCES sys_dept (id) ON DELETE RESTRICT;

CREATE INDEX sys_user_department_id_idx ON sys_user (department_id);

ALTER TABLE sys_user DROP COLUMN department;
