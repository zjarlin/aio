DO $$
BEGIN
    LOCK TABLE engine_applications, engine_application_drafts,
        engine_application_revisions IN ACCESS EXCLUSIVE MODE;

    IF (SELECT COUNT(*) FROM engine_applications) <> 1 THEN
        RAISE EXCEPTION '单一 Program 迁移要求数据库中恰好存在一条旧 Application 记录';
    END IF;
    IF (SELECT COUNT(*) FROM engine_application_drafts) <> 1 THEN
        RAISE EXCEPTION '单一 Program 迁移要求数据库中恰好存在一条 Draft';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM engine_applications program
        LEFT JOIN engine_application_revisions revision
            ON revision.id = program.active_revision_id
           AND revision.application_id = program.id
        WHERE program.active_revision_id IS NOT NULL
          AND revision.id IS NULL
    ) THEN
        RAISE EXCEPTION '旧 Application 的活动 Revision 归属无效';
    END IF;
END;
$$;

DROP TRIGGER engine_program_activation_notify ON engine_applications;
DROP TRIGGER engine_application_revisions_immutable ON engine_application_revisions;
DROP TRIGGER engine_program_images_immutable ON engine_program_images;

ALTER TABLE engine_applications RENAME TO engine_programs;
ALTER TABLE engine_application_drafts RENAME TO engine_program_drafts;
ALTER TABLE engine_application_revisions RENAME TO engine_program_revisions;

ALTER TABLE engine_program_drafts RENAME COLUMN application_id TO program_id;
ALTER TABLE engine_program_revisions RENAME COLUMN application_id TO program_id;
ALTER TABLE engine_revision_runs RENAME COLUMN application_id TO program_id;
ALTER TABLE engine_vibe_sessions RENAME COLUMN application_id TO program_id;
ALTER TABLE engine_program_expression_indexes RENAME COLUMN application_id TO program_id;

ALTER TABLE engine_programs
    ADD COLUMN singleton BOOLEAN NOT NULL DEFAULT TRUE CHECK (singleton);
CREATE UNIQUE INDEX engine_programs_singleton_uidx ON engine_programs (singleton);

ALTER TABLE engine_programs
    RENAME CONSTRAINT engine_applications_pkey TO engine_programs_pkey;
ALTER TABLE engine_programs
    RENAME CONSTRAINT engine_applications_active_revision_fk TO engine_programs_active_revision_fk;
ALTER TABLE engine_program_drafts
    RENAME CONSTRAINT engine_application_drafts_pkey TO engine_program_drafts_pkey;
ALTER TABLE engine_program_drafts
    RENAME CONSTRAINT engine_application_drafts_application_id_fkey TO engine_program_drafts_program_id_fkey;
ALTER TABLE engine_program_revisions
    RENAME CONSTRAINT engine_application_revisions_pkey TO engine_program_revisions_pkey;
ALTER TABLE engine_program_revisions
    RENAME CONSTRAINT engine_application_revisions_application_id_fkey TO engine_program_revisions_program_id_fkey;
ALTER TABLE engine_revision_runs
    RENAME CONSTRAINT engine_revision_runs_application_id_fkey TO engine_revision_runs_program_id_fkey;
ALTER TABLE engine_vibe_sessions
    RENAME CONSTRAINT engine_vibe_sessions_application_id_fkey TO engine_vibe_sessions_program_id_fkey;
ALTER TABLE engine_program_expression_indexes
    RENAME CONSTRAINT engine_program_expression_indexes_application_id_fkey TO engine_program_expression_indexes_program_id_fkey;

ALTER INDEX engine_applications_name_uidx RENAME TO engine_programs_name_uidx;
ALTER INDEX engine_application_revisions_number_uidx RENAME TO engine_program_revisions_number_uidx;
ALTER INDEX engine_application_revisions_created_idx RENAME TO engine_program_revisions_created_idx;
ALTER INDEX engine_revision_runs_application_idx RENAME TO engine_revision_runs_program_idx;
ALTER INDEX engine_vibe_sessions_application_idx RENAME TO engine_vibe_sessions_program_idx;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

CREATE OR REPLACE FUNCTION engine_notify_program_activation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.active_revision_id IS DISTINCT FROM OLD.active_revision_id
       AND NEW.active_revision_id IS NOT NULL THEN
        PERFORM pg_notify(
            'engine_program_activated',
            json_build_object(
                'revision_id', NEW.active_revision_id
            )::text
        );
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER engine_program_activation_notify
AFTER UPDATE OF active_revision_id ON engine_programs
FOR EACH ROW EXECUTE FUNCTION engine_notify_program_activation();

DO $$
BEGIN
    IF (SELECT COUNT(*) FROM engine_programs WHERE singleton) <> 1
       OR (SELECT COUNT(*) FROM engine_program_drafts) <> 1
       OR EXISTS (
            SELECT 1
            FROM engine_program_drafts draft
            LEFT JOIN engine_programs program ON program.id = draft.program_id
            WHERE program.id IS NULL
       ) THEN
        RAISE EXCEPTION '单一 Program 迁移后的完整性校验失败';
    END IF;
END;
$$;
