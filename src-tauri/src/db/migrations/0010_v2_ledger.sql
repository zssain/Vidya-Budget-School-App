-- 0010_v2_ledger.sql — Phase 13 (v2): general ledger (foundation §8.3).
--
-- Additive only. Double-entry vouchers: every money movement writes ONE balanced
-- voucher (Σ debits = Σ credits, checked in vidya-core::ledger) made of ledger
-- entries against system accounts. voucher + ledger_entry are APPEND-ONLY (like
-- payment/reversal/audit): never edited, mistakes are fixed by a reversal voucher.
-- Backfill for existing payments/reversals is a Rust one-time pass
-- (ledger::backfill_vouchers) — derived rows only; originals untouched; day/mode
-- money totals are computed from payment/reversal and so are unchanged.

CREATE TABLE IF NOT EXISTS ledger_account (
  id        TEXT PRIMARY KEY,
  code      TEXT NOT NULL,
  name      TEXT NOT NULL,
  name_hi   TEXT,
  name_te   TEXT,               -- filled in the Telugu step (Step 11)
  kind      TEXT NOT NULL CHECK (kind IN ('asset','liability','income','expense')),
  system    INTEGER NOT NULL DEFAULT 0 CHECK (system IN (0,1)),
  active    INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  school_id TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

CREATE TABLE IF NOT EXISTS voucher (
  id           TEXT PRIMARY KEY,
  voucher_no   TEXT NOT NULL,
  kind         TEXT NOT NULL CHECK (kind IN
    ('receipt','reversal','expense','salary','advance','store_sale','opening','transfer')),
  date         TEXT NOT NULL,       -- YYYY-MM-DD
  narration    TEXT,
  source_table TEXT,                -- e.g. 'payment' | 'reversal'
  source_id    TEXT,
  created_by   TEXT,
  device_id    TEXT,
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (voucher_no)
);

CREATE TABLE IF NOT EXISTS ledger_entry (
  id           TEXT PRIMARY KEY,
  voucher_id   TEXT NOT NULL REFERENCES voucher(id),
  account_id   TEXT NOT NULL REFERENCES ledger_account(id),
  debit_paise  INTEGER NOT NULL DEFAULT 0 CHECK (debit_paise  >= 0),
  credit_paise INTEGER NOT NULL DEFAULT 0 CHECK (credit_paise >= 0)
);

CREATE INDEX IF NOT EXISTS idx_ledger_entry_voucher ON ledger_entry(voucher_id);
CREATE INDEX IF NOT EXISTS idx_ledger_entry_account ON ledger_entry(account_id);
CREATE INDEX IF NOT EXISTS idx_voucher_source ON voucher(source_table, source_id);
CREATE INDEX IF NOT EXISTS idx_voucher_date   ON voucher(date);

-- Append-only guards (like payment/reversal/audit): mistakes are new reversal
-- vouchers, never edits or deletes.
CREATE TRIGGER IF NOT EXISTS voucher_no_update BEFORE UPDATE ON voucher
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS voucher_no_delete BEFORE DELETE ON voucher
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS ledger_entry_no_update BEFORE UPDATE ON ledger_entry
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS ledger_entry_no_delete BEFORE DELETE ON ledger_entry
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;

-- System chart of accounts (mirrors vidya_core::ledger::system_accounts — ids,
-- codes and kinds must match). name_te is filled in Step 11 (Telugu).
INSERT OR IGNORE INTO ledger_account(id, code, name, name_hi, kind, system, active) VALUES
  ('cash',           '1001', 'Cash in hand',          'नकद',                  'asset',     1, 1),
  ('bank',           '1002', 'UPI / Bank',            'यूपीआई / बैंक',        'asset',     1, 1),
  ('cheques',        '1003', 'Cheques to deposit',    'जमा करने योग्य चेक',   'asset',     1, 1),
  ('staff_advances', '1004', 'Staff advances',        'स्टाफ अग्रिम',          'asset',     1, 1),
  ('fee_income',     '4001', 'Fee income',            'फीस आय',               'income',    1, 1),
  ('store_income',   '4002', 'Store income',          'स्टोर आय',              'income',    1, 1),
  ('salary_expense', '5001', 'Salary expense',        'वेतन व्यय',             'expense',   1, 1),
  ('electricity',    '5002', 'Electricity',           'बिजली',                 'expense',   1, 1),
  ('rent',           '5003', 'Rent',                  'किराया',                'expense',   1, 1),
  ('repairs',        '5004', 'Repairs & maintenance', 'मरम्मत और रखरखाव',      'expense',   1, 1),
  ('stationery',     '5005', 'Stationery',            'स्टेशनरी',              'expense',   1, 1),
  ('other_expense',  '5099', 'Other expense',         'अन्य व्यय',             'expense',   1, 1),
  ('opening_equity', '3001', 'Opening balance equity','प्रारंभिक शेष पूँजी',  'liability', 1, 1);
