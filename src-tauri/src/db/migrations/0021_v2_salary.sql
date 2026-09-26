-- 0021_v2_salary.sql — Phase 15 (v2): salary register (§10.3, Step 5).
--
-- Additive only. Belongs to the `accounts` module (scope::module_of_table).
-- salary_structure: monthly salary per staff (effective-dated). staff_advance:
-- advances given (with a recovery-per-month plan). salary_run + salary_line: a
-- month's pay run and its per-staff computed lines. Paying a line posts a balanced
-- salary voucher (Dr salary expense, Cr staff advances recovered, Cr money) in the
-- same transaction (src-tauri::ledger). Advances post an advance voucher.

CREATE TABLE IF NOT EXISTS salary_structure (
  id             TEXT PRIMARY KEY,
  staff_id       TEXT NOT NULL REFERENCES staff(id),
  monthly_paise  INTEGER NOT NULL CHECK (monthly_paise >= 0),
  effective_from TEXT NOT NULL,             -- YYYY-MM-DD
  school_id      TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_salary_structure_staff ON salary_structure(staff_id, effective_from);

CREATE TABLE IF NOT EXISTS staff_advance (
  id                       TEXT PRIMARY KEY,
  staff_id                 TEXT NOT NULL REFERENCES staff(id),
  amount_paise             INTEGER NOT NULL CHECK (amount_paise > 0),
  voucher_id               TEXT REFERENCES voucher(id),
  recover_per_month_paise  INTEGER NOT NULL DEFAULT 0 CHECK (recover_per_month_paise >= 0),
  recovered_paise          INTEGER NOT NULL DEFAULT 0 CHECK (recovered_paise >= 0),
  given_on                 TEXT NOT NULL,    -- YYYY-MM-DD
  created_by               TEXT REFERENCES staff(id),
  device_id                TEXT REFERENCES device(id),
  school_id                TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_staff_advance_staff ON staff_advance(staff_id);

CREATE TABLE IF NOT EXISTS salary_run (
  id           TEXT PRIMARY KEY,
  month        TEXT NOT NULL,               -- YYYY-MM
  status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','finalised')),
  finalised_by TEXT REFERENCES staff(id),
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (month)
);

CREATE TABLE IF NOT EXISTS salary_line (
  id                     TEXT PRIMARY KEY,
  run_id                 TEXT NOT NULL REFERENCES salary_run(id),
  staff_id               TEXT NOT NULL REFERENCES staff(id),
  monthly_paise          INTEGER NOT NULL,
  working_days           INTEGER NOT NULL,
  days_present           INTEGER NOT NULL,
  unpaid_leave_days      INTEGER NOT NULL,
  deduction_paise        INTEGER NOT NULL DEFAULT 0,
  advance_recovery_paise INTEGER NOT NULL DEFAULT 0,
  net_paise              INTEGER NOT NULL,
  paid_voucher_id        TEXT REFERENCES voucher(id),
  school_id              TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (run_id, staff_id)
);
CREATE INDEX IF NOT EXISTS idx_salary_line_run ON salary_line(run_id);
