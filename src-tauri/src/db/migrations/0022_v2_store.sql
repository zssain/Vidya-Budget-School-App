-- 0022_v2_store.sql — Phase 15 (v2): optional School store (§10.3, Step 6).
--
-- Additive only. Belongs to the OPTIONAL `store` module (default OFF, §14):
-- scope::module_of_table maps these tables to "store", so they never sync to a
-- device when the module is off. A sale is APPEND-ONLY (like payment) and posts a
-- Store-income voucher (Dr money, Cr store income) with an S-… receipt number.
-- stock_move records every stock change (sale/purchase/adjust); store_item.stock
-- is the running level.

CREATE TABLE IF NOT EXISTS store_item (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  name_hi      TEXT,
  name_te      TEXT,
  price_paise  INTEGER NOT NULL CHECK (price_paise >= 0),
  stock        INTEGER NOT NULL DEFAULT 0,
  low_stock_at INTEGER NOT NULL DEFAULT 0,
  active       INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

CREATE TABLE IF NOT EXISTS store_sale (
  id          TEXT PRIMARY KEY,
  receipt_no  TEXT NOT NULL,                -- S-A2-0031 (per-device series)
  student_id  TEXT REFERENCES student(id),
  guardian_id TEXT REFERENCES guardian(id),
  items_json  TEXT NOT NULL,               -- [{item_id, name, qty, price_paise}]
  total_paise INTEGER NOT NULL CHECK (total_paise > 0),
  mode        TEXT NOT NULL CHECK (mode IN ('cash','upi','cheque')),
  voucher_id  TEXT REFERENCES voucher(id),
  sold_by     TEXT REFERENCES staff(id),
  sold_at     TEXT NOT NULL,
  device_id   TEXT REFERENCES device(id),
  school_id   TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device',
  UNIQUE (receipt_no)
);
CREATE INDEX IF NOT EXISTS idx_store_sale_sold_at ON store_sale(sold_at);

CREATE TABLE IF NOT EXISTS stock_move (
  id      TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES store_item(id),
  qty     INTEGER NOT NULL,                 -- signed: sale negative, purchase positive
  reason  TEXT NOT NULL CHECK (reason IN ('sale','purchase','adjust')),
  ref_id  TEXT,                             -- the store_sale id for a sale
  by      TEXT REFERENCES staff(id),
  at      TEXT NOT NULL,
  school_id TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);
CREATE INDEX IF NOT EXISTS idx_stock_move_item ON stock_move(item_id);

-- A sale is append-only (corrected by a reversal, like a payment).
CREATE TRIGGER IF NOT EXISTS store_sale_no_update BEFORE UPDATE ON store_sale
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER IF NOT EXISTS store_sale_no_delete BEFORE DELETE ON store_sale
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
