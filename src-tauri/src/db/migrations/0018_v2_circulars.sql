-- 0018_v2_circulars.sql — Phase 14 (v2): circulars & notices (§10.1, Step 6).
--
-- Additive only. A `circular` is composed by the Principal (or drafted by a Teacher
-- as a class notice → an approval request, P13's `class_notice` request type). The
-- `number` (CIR/<session>/NNN) is assigned on the SERVER when it is sent (like a
-- receipt number). `circular_read` records staff read events (one per staff member).
-- Both tables belong to the `circulars` module (scope::module_of_table) and never
-- leave a device when that module is off.

CREATE TABLE IF NOT EXISTS circular (
  id                TEXT PRIMARY KEY,
  number            TEXT,                          -- CIR/2026-27/014 — server-assigned on send
  title             TEXT NOT NULL,
  body              TEXT NOT NULL,                 -- the primary-language body
  languages_json    TEXT,                          -- optional per-language bodies {"hi":"…","te":"…"}
  audience_json     TEXT,                          -- {"kind":"whole_school|classes|staff","class_ids":[…]}
  channels_json     TEXT,                          -- ["staff_app","email","wa_groups","print","wa_auto"]
  attachments_json  TEXT,
  status            TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft','pending_approval','sent')),
  created_by        TEXT,
  approved_by       TEXT,
  sent_at           TEXT,
  calendar_event_id TEXT REFERENCES calendar_event(id),
  school_id         TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE INDEX IF NOT EXISTS idx_circular_status ON circular(status);

CREATE TABLE IF NOT EXISTS circular_read (
  id          TEXT PRIMARY KEY,
  circular_id TEXT NOT NULL REFERENCES circular(id),
  staff_id    TEXT NOT NULL REFERENCES staff(id),
  read_at     TEXT,
  school_id   TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device',
  UNIQUE (circular_id, staff_id)
);

CREATE INDEX IF NOT EXISTS idx_circular_read_circular ON circular_read(circular_id);
