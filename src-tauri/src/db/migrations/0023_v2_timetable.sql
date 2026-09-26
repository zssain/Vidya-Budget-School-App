-- 0023_v2_timetable.sql — Phase 16 (v2): weekly timetable (§10.4, Step 1).
--
-- Additive only. Belongs to the `classroom` module (default ON, §14):
-- scope::module_of_table maps these tables to "classroom", so they stop syncing
-- to a device when the module is switched off. A teacher receives only the slots
-- of the classes they teach (class-scoped, see sync::scope).
--
-- `period` gives each period a number + bell times for a session; `timetable_slot`
-- places a class-subject + teacher into (weekday, period). `weekday` is ISO
-- (Monday = 1 … Saturday = 6), matching vidya_core::calendar::weekday_iso.
-- effective_from / effective_to allow a mid-year timetable change without deleting
-- history (NULL to = still in effect).

CREATE TABLE IF NOT EXISTS period (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES academic_session(id),
  no         INTEGER NOT NULL CHECK (no >= 1),
  starts_at  TEXT NOT NULL,                 -- HH:MM (24h)
  ends_at    TEXT NOT NULL,                 -- HH:MM (24h)
  school_id  TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (session_id, no)
);

CREATE TABLE IF NOT EXISTS timetable_slot (
  id               TEXT PRIMARY KEY,
  session_id       TEXT NOT NULL REFERENCES academic_session(id),
  class_id         TEXT NOT NULL REFERENCES class(id),
  weekday          INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
  period_no        INTEGER NOT NULL CHECK (period_no >= 1),
  class_subject_id TEXT NOT NULL REFERENCES class_subject(id),
  teacher_id       TEXT NOT NULL REFERENCES staff(id),
  effective_from   TEXT,                    -- YYYY-MM-DD (NULL = from session start)
  effective_to     TEXT,                    -- YYYY-MM-DD (NULL = still in effect)
  school_id        TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_timetable_slot_class ON timetable_slot(class_id, weekday, period_no);
CREATE INDEX IF NOT EXISTS idx_timetable_slot_teacher ON timetable_slot(teacher_id, weekday, period_no);
