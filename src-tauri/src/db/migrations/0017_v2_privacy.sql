-- 0017_v2_privacy.sql — Phase 13 (v2): privacy / DPDP (foundation §9).
--
-- Additive only. The school is the data fiduciary; Vidya never sends school data
-- to the developer. Consent per student+purpose; an incident log with the 72-hour
-- reporting duty; and a record of export/erase actions. Erase tombstones personal
-- fields but KEEPS financial, academic totals and audit (done in application code).

CREATE TABLE IF NOT EXISTS consent (
  id           TEXT PRIMARY KEY,
  student_id   TEXT NOT NULL REFERENCES student(id),
  guardian_id  TEXT REFERENCES guardian(id),
  purpose      TEXT NOT NULL CHECK (purpose IN ('school_records','messages')),
  method       TEXT NOT NULL CHECK (method IN ('signed_form','in_person')),
  recorded_by  TEXT,
  recorded_at  TEXT NOT NULL,
  withdrawn_at TEXT,
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_consent_student ON consent(student_id);

CREATE TABLE IF NOT EXISTS incident_log (
  id                TEXT PRIMARY KEY,
  occurred_on       TEXT,
  description       TEXT NOT NULL,
  action_taken      TEXT,
  reported_to_board INTEGER NOT NULL DEFAULT 0 CHECK (reported_to_board IN (0,1)),
  reported_on       TEXT,
  recorded_by       TEXT,
  school_id         TEXT REFERENCES school(id),
  created_at        TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_incident_created ON incident_log(created_at);

-- A record of every student export / erase (shown in Settings → Privacy).
CREATE TABLE IF NOT EXISTS privacy_action (
  id           TEXT PRIMARY KEY,
  student_id   TEXT,
  kind         TEXT NOT NULL CHECK (kind IN ('export','erase')),
  performed_by TEXT,
  performed_at TEXT NOT NULL,
  note         TEXT,
  school_id    TEXT REFERENCES school(id)
);
CREATE INDEX IF NOT EXISTS idx_privacy_action_at ON privacy_action(performed_at);
