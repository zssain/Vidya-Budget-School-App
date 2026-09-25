-- 0015_v2_roles.sql — Phase 13 (v2): roles as data (foundation §8.10).
--
-- Additive only. The three built-in roles are stored as rows; their permission
-- matrix (role × action → allow|request) lives in `role_permission`, seeded in
-- application code from vidya_core::permissions::default_permissions() (which is
-- DERIVED from `can`, so the data can never drift from the code). `can` remains
-- the authoritative decision point. No custom-role UI yet.

CREATE TABLE IF NOT EXISTS role (
  id        TEXT PRIMARY KEY,
  key       TEXT NOT NULL UNIQUE,
  name      TEXT NOT NULL,
  built_in  INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0,1)),
  school_id TEXT REFERENCES school(id)
);

CREATE TABLE IF NOT EXISTS role_permission (
  role_id TEXT NOT NULL REFERENCES role(id),
  action  TEXT NOT NULL,
  effect  TEXT NOT NULL CHECK (effect IN ('allow','request')),
  PRIMARY KEY (role_id, action)
);

INSERT OR IGNORE INTO role(id, key, name, built_in) VALUES
  ('role-principal',  'principal',  'Principal',  1),
  ('role-accountant', 'accountant', 'Accountant', 1),
  ('role-teacher',    'teacher',    'Teacher',    1);
