-- 0035_v2_review_flag_store.sql — P21 W1 (store sales from a second PC).
--
-- A store sale made on another device is ALWAYS recorded when the school PC replays
-- it (real money already taken, §8.6) — stock may go negative. Instead of refusing,
-- the school PC raises a review flag for the Principal:
--   * store_oversold     — the sale took an item's stock below zero;
--   * store_sale_review  — the device's price differed from the school PC's, or its
--                          receipt number was already used (renumbered).
-- SQLite cannot widen a CHECK in place, so review_flag is rebuilt (same columns;
-- nothing references it). Triggers on it are re-installed after migrations.

CREATE TABLE review_flag_new (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN
    ('excess_payment','possible_duplicate_payment','revoked_author','unknown_record',
     'store_oversold','store_sale_review')),
  ref_table    TEXT NOT NULL,
  ref_id       TEXT NOT NULL,
  details_json TEXT,
  status       TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
  resolved_by  TEXT REFERENCES staff(id),
  school_id    TEXT
);
INSERT INTO review_flag_new(id, kind, ref_table, ref_id, details_json, status, resolved_by, school_id)
  SELECT id, kind, ref_table, ref_id, details_json, status, resolved_by, school_id FROM review_flag;
DROP TABLE review_flag;
ALTER TABLE review_flag_new RENAME TO review_flag;
