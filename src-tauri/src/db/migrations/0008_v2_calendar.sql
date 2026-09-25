-- 0008_v2_calendar.sql — Phase 13 (v2): school calendar backbone (foundation §8.1).
--
-- Additive only; no committed migration is edited; no financial/audit rows touched.
-- Two tables feed vidya-core::calendar, which decides is_working_day / working_days
-- for attendance %, fee due-date notes, salary days and leave counts. Calendar is
-- Core (always synced) administrative data: only the Principal edits it (Settings),
-- every role reads it (server snapshot) — audience `admin`, like the student roster.

-- The weekly working pattern. `weekday` is ISO 1=Mon … 7=Sun. When a weekday has
-- no row, vidya-core's SchoolWeek::default() applies (Mon–Sat working, Sunday off),
-- so an empty table on a fresh install already behaves correctly; the Settings UI
-- upserts all seven rows (ids `wk-1`..`wk-7`) once the Principal edits weekly offs.
-- The PK is a text `id` (like every synced table, so the sync engine's id-keyed
-- read/apply works); `weekday` is a UNIQUE lookup column.
CREATE TABLE IF NOT EXISTS school_week (
  id         TEXT PRIMARY KEY,
  weekday    INTEGER NOT NULL UNIQUE CHECK (weekday BETWEEN 1 AND 7),
  is_working INTEGER NOT NULL DEFAULT 1 CHECK (is_working IN (0,1)),
  school_id  TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

-- Dated events: holidays / exams / events. starts_on/ends_on are inclusive
-- (a single-day event has starts_on == ends_on). Only is_non_working=1 events flip
-- a day to non-working. title_hi/title_te are optional localised titles.
-- circular_id links to a circular (P14); no FK yet as the table arrives later.
CREATE TABLE IF NOT EXISTS calendar_event (
  id             TEXT PRIMARY KEY,
  session_id     TEXT REFERENCES academic_session(id),
  starts_on      TEXT NOT NULL,           -- YYYY-MM-DD (inclusive)
  ends_on        TEXT NOT NULL,           -- YYYY-MM-DD (inclusive)
  kind           TEXT NOT NULL CHECK (kind IN ('holiday','exam','event')),
  title          TEXT NOT NULL,
  title_hi       TEXT,
  title_te       TEXT,
  is_non_working INTEGER NOT NULL DEFAULT 0 CHECK (is_non_working IN (0,1)),
  circular_id    TEXT,
  school_id      TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE INDEX IF NOT EXISTS idx_calendar_event_session ON calendar_event(session_id);
CREATE INDEX IF NOT EXISTS idx_calendar_event_starts  ON calendar_event(starts_on);
