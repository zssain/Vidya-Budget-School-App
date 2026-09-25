-- 0014_v2_custom_fields.sql — Phase 13 (v2): custom fields (foundation §8.9).
--
-- Additive only. Schools add their own fields to students/staff (text/number/date/
-- choice) with a label in en/hi/te; vidya-core::custom_fields validates the type.
-- `custom_field.key` is the stable CSV header. Values live in `custom_value`.

CREATE TABLE IF NOT EXISTS custom_field (
  id           TEXT PRIMARY KEY,
  entity       TEXT NOT NULL CHECK (entity IN ('student','staff')),
  key          TEXT NOT NULL,
  label        TEXT NOT NULL,
  label_hi     TEXT,
  label_te     TEXT,
  type         TEXT NOT NULL CHECK (type IN ('text','number','date','choice')),
  options_json TEXT,                 -- JSON array of strings, for `choice`
  required     INTEGER NOT NULL DEFAULT 0 CHECK (required IN (0,1)),
  active       INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  sort_order   INTEGER NOT NULL DEFAULT 0,
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (entity, key)
);

CREATE TABLE IF NOT EXISTS custom_value (
  id        TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL,           -- student.id or staff.id
  field_id  TEXT NOT NULL REFERENCES custom_field(id),
  value     TEXT,
  school_id TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (entity_id, field_id)
);

CREATE INDEX IF NOT EXISTS idx_custom_value_entity ON custom_value(entity_id);
CREATE INDEX IF NOT EXISTS idx_custom_field_entity ON custom_field(entity, active);
