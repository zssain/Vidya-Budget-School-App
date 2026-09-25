-- 0012_v2_request_types.sql — Phase 13 (v2): approval registry (foundation §8.5).
--
-- Extends `request.type` to accept the three new v2 types (leave, attendance_duty,
-- class_notice) served through the vidya-core approval registry. SQLite can't ALTER
-- a CHECK, so this is the standard table-recreate: same columns/order + the widened
-- CHECK. Additive in effect (no committed migration edited; no rows dropped). The
-- `reversal.request_id` FK resolves by name to the recreated table.
--
-- NOTE: the recreate's implicit row-copy needs no `reversal` row to reference a
-- `request` being deleted; on a fresh/demo DB `reversal` is empty so DROP is clean.
-- On a production DB that has applied payment reversals the reversal→request FK
-- would block the DROP; that path runs the recreate with foreign_keys OFF at the
-- app layer (flagged in the P13 handoff).

CREATE TABLE request_new (
  id                TEXT PRIMARY KEY,
  type              TEXT NOT NULL CHECK (type IN
    ('marks_correction','attendance_correction','student_details','payment_reversal',
     'access_change','device_replacement','leave','attendance_duty','class_notice')),
  target_table      TEXT NOT NULL,
  target_id         TEXT NOT NULL,
  base_version      INTEGER NOT NULL,
  before_json       TEXT,
  after_json        TEXT,
  reason            TEXT NOT NULL,
  requested_by      TEXT NOT NULL REFERENCES staff(id),
  revision          INTEGER NOT NULL DEFAULT 1,
  parent_request_id TEXT REFERENCES request(id),
  status            TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending','approved','rejected','returned','cancelled')),
  decided_by        TEXT REFERENCES staff(id),
  decided_at        TEXT,
  note              TEXT,
  apply_state       TEXT NOT NULL DEFAULT 'not_applied'
    CHECK (apply_state IN ('not_applied','applied','failed','stale')),
  applied_at        TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

INSERT INTO request_new SELECT * FROM request;
DROP TABLE request;
ALTER TABLE request_new RENAME TO request;

CREATE INDEX IF NOT EXISTS idx_request_status ON request(status);
