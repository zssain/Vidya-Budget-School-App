-- 0011_v2_numbering.sql — Phase 13 (v2): the numbering engine (foundation §8.4).
--
-- Additive only. One counter per (kind, series): receipts R-, vouchers V-, store
-- sales S-, hall tickets HT- (series = a device issue series like A2), and
-- circulars CIR/<session>/NNN (series = the session label, server-only). last_seq
-- is the highest sequence issued. Counters are device-local bookkeeping (a device
-- owns its series), so this table is NOT synced. The format lives in
-- vidya-core::numbering; the repo (src-tauri::numbering) manages last_seq and
-- seeds it from any pre-engine rows (payments/vouchers) for continuity.
CREATE TABLE IF NOT EXISTS number_series (
  kind     TEXT NOT NULL,
  series   TEXT NOT NULL,
  prefix   TEXT NOT NULL,
  last_seq INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (kind, series)
);
