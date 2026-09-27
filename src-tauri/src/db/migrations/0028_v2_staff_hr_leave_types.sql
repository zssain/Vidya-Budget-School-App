-- 0028 — Staff HR (P17), Step 1: leave types.
-- Additive & numbered (rule §9). `leave_type` belongs to the `hr` module (default
-- ON, §14); `scope::module_of_table` maps it so it stops syncing when HR is off.
-- Standard sync columns + school_id, like every synced table.

CREATE TABLE IF NOT EXISTS leave_type (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  name_hi       TEXT,
  name_te       TEXT,
  -- Yearly quota in working days; NULL = unlimited (an Unpaid type has no quota).
  yearly_quota  INTEGER,
  paid          INTEGER NOT NULL DEFAULT 1 CHECK (paid IN (0,1)),
  active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  sort_order    INTEGER NOT NULL DEFAULT 0,
  school_id     TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

-- **[OWNER defaults]** (§10.5 / OWNER-DECISIONS #8): Casual 12 paid, Sick 6 paid,
-- Unpaid (no quota, unpaid). Hindi/Telugu names are DRAFTS (native review pending,
-- OWNER #12). Editable in Settings → Staff HR.
INSERT INTO leave_type(id, name, name_hi, name_te, yearly_quota, paid, active, sort_order) VALUES
  ('lt-casual', 'Casual leave', 'आकस्मिक अवकाश', 'సాధారణ సెలవు', 12, 1, 1, 0),
  ('lt-sick',   'Sick leave',   'बीमारी अवकाश',  'అనారోగ్య సెలవు', 6, 1, 1, 1),
  ('lt-unpaid', 'Unpaid leave', 'अवैतनिक अवकाश', 'జీతం లేని సెలవు', NULL, 0, 1, 2);
