-- 0027_v2_exam_seating.sql — Phase 16 (v2): exam seating & hall tickets (§10.4, Step 5).
--
-- Additive only. Belongs to the `classroom` module (default ON, §14). Principal-
-- managed on the school PC (hall tickets + seating charts print there), so these
-- sync via the snapshot, not device ops. exam_room = a room (rows × cols +
-- invigilator); exam_seat = one student's seat (regenerated deterministically);
-- exam_schedule = the date/subject/time for a class in the exam (shown on hall
-- tickets). Seating is regenerated wholesale, so exam_seat rows are replaced, not
-- append-only.

CREATE TABLE IF NOT EXISTS exam_room (
  id             TEXT PRIMARY KEY,
  exam_id        TEXT NOT NULL REFERENCES exam(id),
  name           TEXT NOT NULL,
  rows           INTEGER NOT NULL CHECK (rows >= 1),
  cols           INTEGER NOT NULL CHECK (cols >= 1),
  invigilator_id TEXT REFERENCES staff(id),
  sort_order     INTEGER NOT NULL DEFAULT 0,
  school_id      TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_exam_room_exam ON exam_room(exam_id, sort_order);

CREATE TABLE IF NOT EXISTS exam_seat (
  id         TEXT PRIMARY KEY,
  exam_id    TEXT NOT NULL REFERENCES exam(id),
  room_id    TEXT NOT NULL REFERENCES exam_room(id),
  seat_no    INTEGER NOT NULL,
  student_id TEXT NOT NULL REFERENCES student(id),
  class_slot TEXT,
  school_id  TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_exam_seat_exam ON exam_seat(exam_id);
CREATE UNIQUE INDEX IF NOT EXISTS uix_exam_seat_student ON exam_seat(exam_id, student_id);

CREATE TABLE IF NOT EXISTS exam_schedule (
  id               TEXT PRIMARY KEY,
  exam_id          TEXT NOT NULL REFERENCES exam(id),
  date             TEXT NOT NULL,          -- YYYY-MM-DD
  class_id         TEXT NOT NULL REFERENCES class(id),
  class_subject_id TEXT REFERENCES class_subject(id),
  starts_at        TEXT,                   -- HH:MM
  school_id        TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_exam_schedule_exam ON exam_schedule(exam_id, date);
