-- 0025_v2_homework_notes.sql — Phase 16 (v2): homework & class notes (§10.4, Step 3).
--
-- Additive only. Belongs to the `classroom` module (default ON, §14). A teacher of
-- the class writes homework or class notes (study material only — no student
-- photos or marks). attachments_json is [{name, size, mime, drive_file_id, local_hash}];
-- files are uploaded to the sync account's notes/<class>/<yyyy-mm>/ folder
-- (NOT encrypted — study material). shared_json records how it was shared
-- (WhatsApp tap / email queued). Class-scoped: every teacher of the class + the
-- Principal see the history; a teacher may delete only their own within 24 h.

CREATE TABLE IF NOT EXISTS homework_note (
  id               TEXT PRIMARY KEY,
  class_id         TEXT NOT NULL REFERENCES class(id),
  class_subject_id TEXT REFERENCES class_subject(id),
  kind             TEXT NOT NULL CHECK (kind IN ('homework','notes')),
  text             TEXT NOT NULL DEFAULT '',
  attachments_json TEXT NOT NULL DEFAULT '[]',
  shared_json      TEXT NOT NULL DEFAULT '[]',
  created_by       TEXT REFERENCES staff(id),
  school_id        TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);
CREATE INDEX IF NOT EXISTS idx_homework_note_class ON homework_note(class_id, created_at);
