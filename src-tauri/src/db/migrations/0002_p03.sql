-- Phase 3 additions (prompts/P03). Local-only app/setup state, stored inside the
-- encrypted DB (so it is "stored encrypted" per Step 4). Not a synced table.

-- Generic key-value store for app + setup-wizard state:
--   'pending_licence'  the verified licence held between activation and school
--                      creation (the licence row needs a school row to exist).
--   'setup_step'       the last completed wizard step (resume after restart).
--   'setup_school' / 'setup_session' / 'setup_classes' / 'setup_principal'
--                      collected-but-not-committed wizard values (no PIN, no
--                      recovery key — those never touch storage in plaintext).
--   'device_mode'      'server' on a single-PC school (this phase).
CREATE TABLE app_kv (
  key        TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
