-- 0020_v2_expenses.sql — Phase 15 (v2): expenses (§10.3, Step 3).
--
-- Additive only. An expense is APPEND-ONLY (like payment/reversal): it is never
-- edited; a mistake is fixed by an `expense_reversal` (a reversal voucher),
-- Principal only. Each expense posts one balanced voucher (Dr category account,
-- Cr the money account for paid_via) in the same transaction (src-tauri::ledger).
-- `bill_attachment` is the sha256 hash into the local attachment store (Step 2).
-- Both tables belong to the `accounts` module (scope::module_of_table).

CREATE TABLE IF NOT EXISTS expense (
  id                  TEXT PRIMARY KEY,
  voucher_id          TEXT REFERENCES voucher(id),
  category_account_id TEXT NOT NULL REFERENCES ledger_account(id),
  amount_paise        INTEGER NOT NULL CHECK (amount_paise > 0),
  paid_via            TEXT NOT NULL CHECK (paid_via IN ('cash','upi','bank')),
  details             TEXT,
  vendor              TEXT,
  bill_attachment     TEXT,                 -- sha256 of the compressed bill photo
  spent_on            TEXT NOT NULL,        -- YYYY-MM-DD
  created_by          TEXT REFERENCES staff(id),
  device_id           TEXT REFERENCES device(id),
  school_id           TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE INDEX IF NOT EXISTS idx_expense_spent_on ON expense(spent_on);
CREATE INDEX IF NOT EXISTS idx_expense_category ON expense(category_account_id);

CREATE TABLE IF NOT EXISTS expense_reversal (
  id          TEXT PRIMARY KEY,
  expense_id  TEXT NOT NULL REFERENCES expense(id),
  voucher_id  TEXT REFERENCES voucher(id),
  reason      TEXT NOT NULL,
  approved_by TEXT REFERENCES staff(id),
  applied_at  TEXT NOT NULL,
  school_id   TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE INDEX IF NOT EXISTS idx_expense_reversal_expense ON expense_reversal(expense_id);

-- Append-only guards: expenses/reversals are corrected by a new reversal, never
-- edited or deleted (identical to payment/reversal/voucher/audit in 0001/0010).
CREATE TRIGGER IF NOT EXISTS expense_no_update BEFORE UPDATE ON expense
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS expense_no_delete BEFORE DELETE ON expense
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS expense_reversal_no_update BEFORE UPDATE ON expense_reversal
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS expense_reversal_no_delete BEFORE DELETE ON expense_reversal
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
