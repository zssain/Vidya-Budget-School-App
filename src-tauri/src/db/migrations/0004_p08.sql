-- Phase 8 additions (prompts/P08 Part D — new session rollover).
--
-- Carry-forward link on fee_due: at session rollover, each student's unpaid
-- balance becomes ONE "Previous balance" due in the new Term 1. That due records
-- the old dues it represents here; the old dues are KEPT, never deleted (Part D).
-- New column only; no earlier migration is edited (Standing Rule 9).
ALTER TABLE fee_due ADD COLUMN carried_from_due_ids TEXT;
