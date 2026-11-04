-- 0003: owner "Retry issuance" for a failed licence issuance. Additive only;
-- 0001/0002 are never edited.
--
-- Owner decision (2026-11-05): a paid order whose signing failed stays
-- "Paid · Licence pending". The owner retries the SAME issuance row (same
-- entitlement, generation, licence_id, machine code, issued_at); no new licence
-- and no new generation are created. Only a transfer creates a new issuance.
--
-- Why:
--  * vidya_portal gets UPDATE on (status, last_error) of licence_issuances so the
--    audited owner action can move a `failed` row back to `pending`. It still
--    cannot touch any identity or artefact column.
--  * licence_issuance_guard (a trigger, so it binds EVERY role, including
--    superusers / admin scripts) makes the invariants structural:
--      - identity columns (entitlement_id, generation, licence_id, machine_code,
--        issued_at) never change once inserted;
--      - payload, once written (the signer persists it BEFORE signing), never
--        changes — so any retry signs byte-identical bytes;
--      - signature / licence_key / vlic / signer_key_id, once written, never
--        change — one artefact per issuance, ever;
--      - status moves only along: pending → issued | failed | superseded,
--        failed → pending (retry; only while no signature exists),
--        issued → superseded. `issued` never goes back to pending/failed;
--      - `issued` requires a complete artefact.
SET search_path = billing, public;

CREATE OR REPLACE FUNCTION billing.licence_issuance_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.entitlement_id IS DISTINCT FROM OLD.entitlement_id
     OR NEW.generation IS DISTINCT FROM OLD.generation
     OR NEW.licence_id IS DISTINCT FROM OLD.licence_id
     OR NEW.machine_code IS DISTINCT FROM OLD.machine_code
     OR NEW.issued_at IS DISTINCT FROM OLD.issued_at THEN
    RAISE EXCEPTION 'licence_issuances identity columns are immutable';
  END IF;
  IF OLD.payload IS NOT NULL AND NEW.payload IS DISTINCT FROM OLD.payload THEN
    RAISE EXCEPTION 'licence_issuances.payload is immutable once persisted';
  END IF;
  IF OLD.signature IS NOT NULL AND (
       NEW.signature IS DISTINCT FROM OLD.signature
       OR NEW.licence_key IS DISTINCT FROM OLD.licence_key
       OR NEW.vlic IS DISTINCT FROM OLD.vlic
       OR NEW.signer_key_id IS DISTINCT FROM OLD.signer_key_id) THEN
    RAISE EXCEPTION 'licence_issuances artefact is immutable once signed';
  END IF;
  IF NEW.status IS DISTINCT FROM OLD.status THEN
    IF NOT (
         (OLD.status = 'pending' AND NEW.status IN ('issued', 'failed', 'superseded'))
      OR (OLD.status = 'failed' AND NEW.status = 'pending' AND OLD.signature IS NULL)
      OR (OLD.status = 'issued' AND NEW.status = 'superseded')
    ) THEN
      RAISE EXCEPTION 'licence_issuances status % -> % is not allowed', OLD.status, NEW.status;
    END IF;
  END IF;
  IF NEW.status = 'issued' AND (NEW.payload IS NULL OR NEW.signature IS NULL
                                OR NEW.licence_key IS NULL OR NEW.vlic IS NULL) THEN
    RAISE EXCEPTION 'an issued licence needs payload, signature, licence_key and vlic';
  END IF;
  RETURN NEW;
END $$;

CREATE TRIGGER licence_issuance_guard BEFORE UPDATE ON licence_issuances
  FOR EACH ROW EXECUTE FUNCTION billing.licence_issuance_guard();

-- Owner retry: failed → pending (and clear last_error; the history stays in
-- owner_audit and in the dead job rows, which are never reused).
GRANT UPDATE (status, last_error) ON licence_issuances TO vidya_portal;

-- Attempt history lookups by issuance (owner licence page, retry action).
CREATE INDEX jobs_issuance_idx ON jobs ((payload->>'issuance_id')) WHERE kind IN ('issue_licence', 'deliver_licence');
