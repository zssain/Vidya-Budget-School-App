-- 0006_v2_modules.sql — Phase 11 (v2): module switches (foundation item 8).
--
-- Additive only. A disabled module hides its screens, rejects its commands/ops
-- (MODULE_OFF), and does not sync its tables to devices; turning a module off
-- never deletes data (00-SYSTEM-CONTEXT §14). Core is always on and has no row.

CREATE TABLE IF NOT EXISTS module_setting (
  key        TEXT PRIMARY KEY,
  enabled    INTEGER NOT NULL DEFAULT 1,
  changed_by TEXT,
  changed_at TEXT
);

-- §11 defaults: School accounts / Classroom / Staff HR / Circulars ON;
-- Automatic WhatsApp / School store / Instant sync (relay) OFF.
INSERT OR IGNORE INTO module_setting(key, enabled) VALUES
  ('accounts', 1),
  ('classroom', 1),
  ('hr', 1),
  ('circulars', 1),
  ('wa_auto', 0),
  ('store', 0),
  ('instant_sync', 0);
