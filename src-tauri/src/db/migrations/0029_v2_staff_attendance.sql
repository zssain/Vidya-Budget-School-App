-- 0029 — Staff HR (P17), Step 2: staff attendance.
-- Additive & numbered. `staff_attendance` is in the `hr` module. One row per
-- (staff, day). Times: `check_in_at` / `check_out_at` are the server's UTC ISO
-- (audit-aligned, sortable); `check_in_min` / `check_out_min` are the device's
-- LOCAL minutes-since-midnight (display + the late rule). `clock_warning` is the
-- Principal-facing "phone clock differs > 10 min" flag (§10.5). Not append-only:
-- a day's row is updated (check-out, accept away, correction) — the audit log
-- records every change.

CREATE TABLE IF NOT EXISTS staff_attendance (
  id            TEXT PRIMARY KEY,
  staff_id      TEXT NOT NULL REFERENCES staff(id),
  date          TEXT NOT NULL,                       -- YYYY-MM-DD (device-local)
  check_in_at   TEXT,                                -- server UTC ISO
  check_out_at  TEXT,                                -- server UTC ISO
  check_in_min  INTEGER,                             -- local minutes since midnight
  check_out_min INTEGER,
  route         TEXT CHECK (route IN ('lan','drive','manual')),
  status        TEXT NOT NULL CHECK (status IN ('present','late','away_pending','absent','leave','half_day')),
  accepted_by   TEXT REFERENCES staff(id),           -- who accepted an away check-in / made a manual entry
  clock_warning INTEGER NOT NULL DEFAULT 0 CHECK (clock_warning IN (0,1)),
  note          TEXT,
  school_id     TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (staff_id, date)
);

CREATE INDEX IF NOT EXISTS idx_staff_attendance_date ON staff_attendance(date);
CREATE INDEX IF NOT EXISTS idx_staff_attendance_staff ON staff_attendance(staff_id, date);
