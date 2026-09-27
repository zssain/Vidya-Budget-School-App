-- 0031_v2_device_web_platform.sql — Phase 19: allow 'web' as a device platform so an
-- iPhone PWA can join (§18). SQLite can't ALTER a CHECK, so this is the standard
-- table-recreate with the widened CHECK — SAME columns and order as the current
-- `device` table (0001 + 0003 `session_key`/`last_counter` + 0016 `school_id`), so
-- every device row + token is preserved. `payment.device_id` references `device(id)`;
-- the migration runner applies this with foreign_keys OFF and re-checks integrity
-- afterwards (P18 runner), and the recreated rows keep their ids so the FK resolves.

CREATE TABLE device_new (
  id               TEXT PRIMARY KEY,
  staff_id         TEXT NOT NULL REFERENCES staff(id),
  platform         TEXT NOT NULL CHECK (platform IN ('windows','macos','android','web')),
  name             TEXT NOT NULL,
  token_hash       TEXT NOT NULL,
  receipt_series   TEXT,
  admission_series TEXT,
  last_seen_at     TEXT,
  revoked_at       TEXT,
  lease_expires_at TEXT,
  needs_rejoin     INTEGER NOT NULL DEFAULT 0 CHECK (needs_rejoin IN (0,1)),
  session_key      TEXT,
  last_counter     INTEGER NOT NULL DEFAULT 0,
  school_id        TEXT
);

INSERT INTO device_new SELECT * FROM device;
DROP TABLE device;
ALTER TABLE device_new RENAME TO device;
