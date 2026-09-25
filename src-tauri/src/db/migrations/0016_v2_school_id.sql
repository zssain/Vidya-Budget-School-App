-- 0016_v2_school_id.sql — Phase 13 (v2): school_id everywhere (foundation §8.12).
--
-- Additive only; no behaviour change (single school; nothing reads school_id yet).
-- Adds school_id to every mutable school-scoped domain table that lacks it and
-- backfills each row to the single school's id. On a fresh install the tables are
-- empty and the backfill is a no-op; new P13 tables already carry school_id.
--
-- Deliberately EXCLUDED (documented, not an oversight):
--   * append-only tables (payment, reversal, audit_log) — their append-only
--     triggers block the UPDATE backfill; a future multi-branch add-on sets
--     school_id at INSERT. voucher already carries it (set at post time).
--   * per-install / sync bookkeeping (outbox, op_log, applied_ops, sync_cursor,
--     drive_state, backup_run, schema_version, schema_meta, module_setting,
--     number_series, drive_account) — not school-scoped data.
-- Plain TEXT column (no FK) on the retrofit, so an ADD COLUMN never fails on
-- existing rows; P13's own tables use `REFERENCES school(id)`.

ALTER TABLE academic_session  ADD COLUMN school_id TEXT;
ALTER TABLE term              ADD COLUMN school_id TEXT;
ALTER TABLE class             ADD COLUMN school_id TEXT;
ALTER TABLE subject           ADD COLUMN school_id TEXT;
ALTER TABLE class_subject     ADD COLUMN school_id TEXT;
ALTER TABLE staff             ADD COLUMN school_id TEXT;
ALTER TABLE device            ADD COLUMN school_id TEXT;
ALTER TABLE invite            ADD COLUMN school_id TEXT;
ALTER TABLE student           ADD COLUMN school_id TEXT;
ALTER TABLE enrollment        ADD COLUMN school_id TEXT;
ALTER TABLE attendance_sheet  ADD COLUMN school_id TEXT;
ALTER TABLE attendance_mark   ADD COLUMN school_id TEXT;
ALTER TABLE exam              ADD COLUMN school_id TEXT;
ALTER TABLE exam_subject      ADD COLUMN school_id TEXT;
ALTER TABLE marks_sheet       ADD COLUMN school_id TEXT;
ALTER TABLE mark_entry        ADD COLUMN school_id TEXT;
ALTER TABLE grade_scale       ADD COLUMN school_id TEXT;
ALTER TABLE grade_band        ADD COLUMN school_id TEXT;
ALTER TABLE fee_head          ADD COLUMN school_id TEXT;
ALTER TABLE fee_due           ADD COLUMN school_id TEXT;
ALTER TABLE payment_allocation ADD COLUMN school_id TEXT;
ALTER TABLE request           ADD COLUMN school_id TEXT;
ALTER TABLE conflict          ADD COLUMN school_id TEXT;
ALTER TABLE review_flag       ADD COLUMN school_id TEXT;
ALTER TABLE notification      ADD COLUMN school_id TEXT;

-- Backfill to the single school's id (NULL when no school exists yet — a fresh
-- install, where every table is empty anyway).
UPDATE academic_session  SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE term              SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE class             SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE subject           SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE class_subject     SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE staff             SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE device            SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE invite            SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE student           SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE enrollment        SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE attendance_sheet  SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE attendance_mark   SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE exam              SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE exam_subject      SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE marks_sheet       SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE mark_entry        SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE grade_scale       SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE grade_band        SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE fee_head          SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE fee_due           SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE payment_allocation SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE request           SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE conflict          SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE review_flag       SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;
UPDATE notification      SET school_id = (SELECT id FROM school LIMIT 1) WHERE school_id IS NULL;

-- Indexes for the high-volume tables a multi-branch query would filter by school.
CREATE INDEX IF NOT EXISTS idx_student_school    ON student(school_id);
CREATE INDEX IF NOT EXISTS idx_enrollment_school ON enrollment(school_id);
CREATE INDEX IF NOT EXISTS idx_fee_due_school    ON fee_due(school_id);
CREATE INDEX IF NOT EXISTS idx_attmark_school    ON attendance_mark(school_id);
CREATE INDEX IF NOT EXISTS idx_markentry_school  ON mark_entry(school_id);
