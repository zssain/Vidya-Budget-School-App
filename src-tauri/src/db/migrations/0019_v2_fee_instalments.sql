-- 0019_v2_fee_instalments.sql — Phase 15 (v2): fee instalments (§10.2, Step 1).
--
-- Additive only. A fee head may carry an instalment plan in `instalments_json`
-- (either an explicit list [{no, amount_paise, due_date}] or a monthly rule for
-- `month` frequency — see vidya_core::fees::InstalmentPlan). Dues gain an
-- `instalment_no` (1-based) and a `due_date` (YYYY-MM-DD). Existing dues get
-- instalment_no = 1 (column default) and due_date = the term start (best effort:
-- a matching term's start, else the month's first, else the current session start).
-- Allocation stays oldest-due-first, now ordered by due_date (unchanged principle).

ALTER TABLE fee_head ADD COLUMN instalments_json TEXT;

ALTER TABLE fee_due ADD COLUMN instalment_no INTEGER NOT NULL DEFAULT 1;
ALTER TABLE fee_due ADD COLUMN due_date TEXT;

-- Backfill due_date = term start for existing dues (best effort, additive).
-- 1) periods that name a term → that term's start date.
UPDATE fee_due SET due_date = (
  SELECT t.starts_on FROM term t WHERE t.name = fee_due.period LIMIT 1
) WHERE due_date IS NULL
  AND EXISTS (SELECT 1 FROM term t WHERE t.name = fee_due.period);

-- 2) YYYY-MM month periods → the first of that month.
UPDATE fee_due SET due_date = fee_due.period || '-01'
  WHERE due_date IS NULL
  AND fee_due.period GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]';

-- 3) everything else (once / year periods) → the current session start.
UPDATE fee_due SET due_date = (
  SELECT s.starts_on FROM academic_session s WHERE s.is_current = 1 LIMIT 1
) WHERE due_date IS NULL;

CREATE INDEX IF NOT EXISTS idx_fee_due_due_date ON fee_due(due_date);
