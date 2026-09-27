-- 0030 — Staff HR (P17), Step 3: leave records.
-- Additive & numbered. A `leave_record` is written when the Principal approves a
-- `leave` request (P13 registry): the approval also marks the staff-attendance
-- days as `leave` and notifies the requester. `days` = working days in the range;
-- `paid_days` + `unpaid_days` = the split against the type's remaining balance
-- (unpaid spill-over, §10.5). `hr` module; standard sync columns + school_id.

CREATE TABLE IF NOT EXISTS leave_record (
  id            TEXT PRIMARY KEY,
  staff_id      TEXT NOT NULL REFERENCES staff(id),
  leave_type_id TEXT NOT NULL REFERENCES leave_type(id),
  from_date     TEXT NOT NULL,                       -- YYYY-MM-DD, inclusive
  to_date       TEXT NOT NULL,
  days          INTEGER NOT NULL,
  paid_days     INTEGER NOT NULL DEFAULT 0,
  unpaid_days   INTEGER NOT NULL DEFAULT 0,
  request_id    TEXT REFERENCES request(id),
  approved_by   TEXT REFERENCES staff(id),
  school_id     TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

CREATE INDEX IF NOT EXISTS idx_leave_record_staff ON leave_record(staff_id);
CREATE INDEX IF NOT EXISTS idx_leave_record_type ON leave_record(leave_type_id);
