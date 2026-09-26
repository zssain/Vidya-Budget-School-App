-- 0024_v2_substitutes.sql — Phase 16 (v2): substitutes + attendance duty (§10.4, Step 2).
--
-- Additive only. Belongs to the `classroom` module (default ON, §14). A row covers
-- one period the absent teacher would have taught (period_no set), OR that day's
-- class-teacher attendance (period_no NULL, includes_attendance = 1). The grant is
-- by DATE and ends automatically (permissions::may_take_attendance reads it only
-- for its own `date`). leave_record_id links to an approved leave (P17) when the
-- substitution came from a leave; NULL when assigned manually.

CREATE TABLE IF NOT EXISTS substitution (
  id                   TEXT PRIMARY KEY,
  date                 TEXT NOT NULL,                       -- YYYY-MM-DD covered
  absent_teacher_id    TEXT NOT NULL REFERENCES staff(id),
  substitute_teacher_id TEXT NOT NULL REFERENCES staff(id),
  class_id             TEXT NOT NULL REFERENCES class(id),
  period_no            INTEGER,                             -- NULL = attendance duty
  includes_attendance  INTEGER NOT NULL DEFAULT 0 CHECK (includes_attendance IN (0,1)),
  leave_record_id      TEXT,                                -- P17 approved leave, else NULL
  created_by           TEXT REFERENCES staff(id),
  school_id            TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_substitution_sub ON substitution(substitute_teacher_id, date);
CREATE INDEX IF NOT EXISTS idx_substitution_date ON substitution(date);
