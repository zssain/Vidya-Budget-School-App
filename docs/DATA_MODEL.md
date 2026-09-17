# DATA_MODEL.md — database schema

This is the exact schema for migration `0001_init.sql`. Later changes are new numbered migrations; never edit an applied migration.

## Conventions
- SQLite with SQLCipher. `PRAGMA foreign_keys = ON`, `journal_mode = WAL`, `busy_timeout = 5000`.
- Schema version: `PRAGMA user_version`.
- IDs: `TEXT` UUID v7 (created on any device).
- Money: `INTEGER` whole rupees, never negative unless stated.
- Dates: `TEXT` `YYYY-MM-DD`. Timestamps: `TEXT` UTC ISO 8601 with `Z`.
- Booleans: `INTEGER` with `CHECK (x IN (0,1))`.
- `hlc`: `TEXT` hybrid logical clock value from `vidya-core::hlc` (sortable as text). Every syncable row has `updated_hlc`.
- Soft delete is not used for accounting rows. Other rows use `active = 0`.
- The same schema is used on the office computer and on phones. Phones use the client tables and never receive rows their role may not see.

## Append-only tables (UPDATE and DELETE blocked by triggers)
`receipts`, `receipt_cancellations`, `transfer_certificates`, `change_log`.

## 0001_init.sql
```sql
-- ---------- device and app ----------
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
-- keys: device_id, device_code (receipt prefix), device_role ('server'|'client'),
--       hlc_last, last_backup_at, last_backup_seq, wizard_state_json, school_hours_json

CREATE TABLE app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
-- keys: session_timeout_minutes, receipt_paper ('a4'|'a5'|'thermal80'), print_language ('en'|'hi'|'both'),
--       keep_running_in_tray, start_at_login, keep_awake

-- ---------- license (office computer only) ----------
CREATE TABLE license (
  id              INTEGER PRIMARY KEY CHECK (id = 1),
  school_code     TEXT NOT NULL CHECK (length(school_code) BETWEEN 3 AND 12),
  activation_code TEXT NOT NULL,
  payload_b64     TEXT NOT NULL,
  signature_b64   TEXT NOT NULL,
  device_id       TEXT NOT NULL,          -- VD-XXXX-XXXX-XXXX
  max_users       INTEGER NOT NULL CHECK (max_users > 0),
  max_devices     INTEGER NOT NULL CHECK (max_devices >= 0),
  license_type    INTEGER NOT NULL,
  issued_on       TEXT NOT NULL,
  activated_at    TEXT NOT NULL
);

CREATE TABLE used_reset_nonces (
  nonce_hex TEXT PRIMARY KEY,
  used_at   TEXT NOT NULL
);

-- ---------- school ----------
CREATE TABLE school (
  id               INTEGER PRIMARY KEY CHECK (id = 1),
  name             TEXT NOT NULL CHECK (length(trim(name)) > 0),
  address          TEXT NOT NULL DEFAULT '',
  udise            TEXT NOT NULL DEFAULT '' CHECK (udise = '' OR (length(udise) = 11 AND udise NOT GLOB '*[^0-9]*')),
  board            TEXT NOT NULL,
  phone            TEXT NOT NULL DEFAULT '',
  logo_png         BLOB,
  updated_at       TEXT NOT NULL,
  updated_hlc      TEXT NOT NULL
);

CREATE TABLE academic_sessions (
  id                      TEXT PRIMARY KEY,
  name                    TEXT NOT NULL UNIQUE,            -- 2026-27
  starts_on               TEXT NOT NULL,
  ends_on                 TEXT NOT NULL,
  is_current              INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0,1)),
  terms                   INTEGER NOT NULL CHECK (terms IN (1,2,3,4,12)),
  transport_fee_per_term  INTEGER NOT NULL DEFAULT 0 CHECK (transport_fee_per_term >= 0),
  created_at              TEXT NOT NULL,
  updated_hlc             TEXT NOT NULL
);
CREATE UNIQUE INDEX ux_one_current_session ON academic_sessions(is_current) WHERE is_current = 1;

CREATE TABLE classes (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  sort_order  INTEGER NOT NULL,
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  updated_hlc TEXT NOT NULL
);

CREATE TABLE sections (
  id          TEXT PRIMARY KEY,
  class_id    TEXT NOT NULL REFERENCES classes(id),
  name        TEXT NOT NULL,                              -- A, B, C
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  updated_hlc TEXT NOT NULL,
  UNIQUE (class_id, name)
);
CREATE INDEX ix_sections_class ON sections(class_id);

CREATE TABLE fee_plans (
  session_id  TEXT NOT NULL REFERENCES academic_sessions(id),
  class_id    TEXT NOT NULL REFERENCES classes(id),
  tuition     INTEGER NOT NULL CHECK (tuition >= 0),
  exam        INTEGER NOT NULL CHECK (exam >= 0),
  other       INTEGER NOT NULL CHECK (other >= 0),
  updated_hlc TEXT NOT NULL,
  PRIMARY KEY (session_id, class_id)
);

CREATE TABLE subjects (
  id          TEXT PRIMARY KEY,
  class_id    TEXT NOT NULL REFERENCES classes(id),
  name        TEXT NOT NULL,
  sort_order  INTEGER NOT NULL,
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  updated_hlc TEXT NOT NULL,
  UNIQUE (class_id, name)
);
CREATE INDEX ix_subjects_class ON subjects(class_id);

CREATE TABLE exams (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL REFERENCES academic_sessions(id),
  name        TEXT NOT NULL,
  max_marks   INTEGER NOT NULL CHECK (max_marks BETWEEN 1 AND 500),
  sort_order  INTEGER NOT NULL,
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  updated_hlc TEXT NOT NULL,
  UNIQUE (session_id, name)
);

CREATE TABLE grade_scale (
  grade       TEXT PRIMARY KEY,                -- A, B, C, D, E
  min_percent INTEGER NOT NULL CHECK (min_percent BETWEEN 0 AND 100),
  updated_hlc TEXT NOT NULL
);

-- ---------- people ----------
CREATE TABLE users (
  id                  TEXT PRIMARY KEY,
  username            TEXT NOT NULL UNIQUE CHECK (username NOT GLOB '*[^a-z0-9]*' AND length(username) BETWEEN 2 AND 30),
  name                TEXT NOT NULL,
  role                TEXT NOT NULL CHECK (role IN ('principal','accountant','teacher')),
  mobile              TEXT NOT NULL DEFAULT '',
  password_hash       TEXT NOT NULL,           -- Argon2id PHC string
  must_change         INTEGER NOT NULL DEFAULT 1 CHECK (must_change IN (0,1)),
  failed_count        INTEGER NOT NULL DEFAULT 0,
  locked              INTEGER NOT NULL DEFAULT 0 CHECK (locked IN (0,1)),
  locked_until        TEXT,
  active              INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  language            TEXT NOT NULL DEFAULT 'en' CHECK (language IN ('en','hi')),
  created_at          TEXT NOT NULL,
  last_login_at       TEXT,
  password_changed_at TEXT,
  updated_hlc         TEXT NOT NULL
);
CREATE UNIQUE INDEX ux_one_principal ON users(role) WHERE role = 'principal';

CREATE TABLE user_sections (
  user_id    TEXT NOT NULL REFERENCES users(id),
  section_id TEXT NOT NULL REFERENCES sections(id),
  PRIMARY KEY (user_id, section_id)
);

CREATE TABLE students (
  id                TEXT PRIMARY KEY,
  adm_no            TEXT NOT NULL UNIQUE,              -- ADM/0001
  name              TEXT NOT NULL,
  gender            TEXT NOT NULL CHECK (gender IN ('Male','Female','Other')),
  dob               TEXT,
  father            TEXT NOT NULL,
  mother            TEXT NOT NULL DEFAULT '',
  mobile            TEXT NOT NULL CHECK (length(mobile) = 10 AND mobile NOT GLOB '*[^0-9]*'),
  category          TEXT NOT NULL CHECK (category IN ('General','OBC','SC','ST')),
  locality          TEXT NOT NULL DEFAULT '',
  aadhaar_collected INTEGER NOT NULL DEFAULT 0 CHECK (aadhaar_collected IN (0,1)),
  apaar_created     INTEGER NOT NULL DEFAULT 0 CHECK (apaar_created IN (0,1)),
  admitted_on       TEXT NOT NULL,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  updated_hlc       TEXT NOT NULL
);
CREATE INDEX ix_students_name ON students(name);

CREATE TABLE enrollments (
  id          TEXT PRIMARY KEY,
  student_id  TEXT NOT NULL REFERENCES students(id),
  session_id  TEXT NOT NULL REFERENCES academic_sessions(id),
  class_id    TEXT NOT NULL REFERENCES classes(id),
  section_id  TEXT NOT NULL REFERENCES sections(id),
  roll        INTEGER NOT NULL CHECK (roll > 0),
  rte         INTEGER NOT NULL DEFAULT 0 CHECK (rte IN (0,1)),
  transport   INTEGER NOT NULL DEFAULT 0 CHECK (transport IN (0,1)),
  concession  INTEGER NOT NULL DEFAULT 0 CHECK (concession >= 0),
  status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','left','promoted','passed_out')),
  left_on     TEXT,
  left_reason TEXT NOT NULL DEFAULT '',
  updated_hlc TEXT NOT NULL,
  UNIQUE (student_id, session_id)
);
CREATE INDEX ix_enrollments_section ON enrollments(session_id, section_id, status);
CREATE UNIQUE INDEX ux_enrollments_roll ON enrollments(session_id, section_id, roll) WHERE status = 'active';

-- ---------- attendance ----------
CREATE TABLE attendance_days (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL REFERENCES academic_sessions(id),
  section_id  TEXT NOT NULL REFERENCES sections(id),
  date        TEXT NOT NULL,
  saved_by    TEXT NOT NULL REFERENCES users(id),
  saved_at    TEXT NOT NULL,
  updated_hlc TEXT NOT NULL,
  UNIQUE (section_id, date)
);

CREATE TABLE attendance_marks (
  attendance_day_id TEXT NOT NULL REFERENCES attendance_days(id),
  student_id        TEXT NOT NULL REFERENCES students(id),
  status            TEXT NOT NULL CHECK (status IN ('P','A','L')),
  PRIMARY KEY (attendance_day_id, student_id)
);
CREATE INDEX ix_attendance_marks_student ON attendance_marks(student_id);

-- ---------- marks ----------
CREATE TABLE marks (
  id          TEXT PRIMARY KEY,
  exam_id     TEXT NOT NULL REFERENCES exams(id),
  student_id  TEXT NOT NULL REFERENCES students(id),
  subject_id  TEXT NOT NULL REFERENCES subjects(id),
  value       INTEGER,
  absent      INTEGER NOT NULL DEFAULT 0 CHECK (absent IN (0,1)),
  entered_by  TEXT NOT NULL REFERENCES users(id),
  entered_at  TEXT NOT NULL,
  updated_hlc TEXT NOT NULL,
  UNIQUE (exam_id, student_id, subject_id),
  CHECK ((absent = 1 AND value IS NULL) OR (absent = 0 AND value IS NOT NULL AND value >= 0))
);
-- A blank mark is "no row". value <= exams.max_marks is enforced in vidya-core validation.

-- ---------- fees ----------
CREATE TABLE receipts (
  id            TEXT PRIMARY KEY,
  receipt_no    TEXT NOT NULL UNIQUE,                  -- PC-0001, T1-0001
  session_id    TEXT NOT NULL REFERENCES academic_sessions(id),
  student_id    TEXT NOT NULL REFERENCES students(id),
  amount        INTEGER NOT NULL CHECK (amount > 0),
  mode          TEXT NOT NULL CHECK (mode IN ('Cash','UPI','Cheque')),
  reference     TEXT NOT NULL DEFAULT '',
  note          TEXT NOT NULL DEFAULT '',
  paid_on       TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  created_by    TEXT NOT NULL REFERENCES users(id),
  device_code   TEXT NOT NULL,
  balance_after INTEGER NOT NULL,
  hlc           TEXT NOT NULL
);
CREATE INDEX ix_receipts_student ON receipts(session_id, student_id);
CREATE INDEX ix_receipts_paid_on ON receipts(paid_on);
CREATE INDEX ix_receipts_reference ON receipts(mode, reference);

CREATE TABLE receipt_cancellations (
  receipt_id   TEXT PRIMARY KEY REFERENCES receipts(id),
  cancelled_by TEXT NOT NULL REFERENCES users(id),
  cancelled_at TEXT NOT NULL,
  reason       TEXT NOT NULL CHECK (length(trim(reason)) >= 4),
  hlc          TEXT NOT NULL
);

CREATE TABLE transfer_certificates (
  id         TEXT PRIMARY KEY,
  tc_no      TEXT NOT NULL UNIQUE,
  student_id TEXT NOT NULL REFERENCES students(id),
  issued_on  TEXT NOT NULL,
  issued_by  TEXT NOT NULL REFERENCES users(id),
  data_json  TEXT NOT NULL,
  hlc        TEXT NOT NULL
);

CREATE TABLE counters (
  name  TEXT PRIMARY KEY,      -- 'adm_no', 'tc_no', 'receipt:PC', 'receipt:T1'
  value INTEGER NOT NULL
);

-- ---------- audit and sync ----------
CREATE TABLE change_log (
  seq          INTEGER PRIMARY KEY AUTOINCREMENT,   -- server_seq on the office computer
  change_id    TEXT NOT NULL UNIQUE,                -- <device_id>:<device_counter>
  hlc          TEXT NOT NULL,
  device_id    TEXT NOT NULL,
  user_id      TEXT,
  kind         TEXT NOT NULL,                       -- auth, stu, fee, att, mark, user, settings, backup, device, setup, session
  entity       TEXT NOT NULL,
  entity_id    TEXT NOT NULL,
  op           TEXT NOT NULL CHECK (op IN ('insert','update_fields','append','replace_set','event')),
  summary_key  TEXT NOT NULL,                       -- translation key for the activity screen
  params_json  TEXT NOT NULL DEFAULT '{}',
  payload_json TEXT NOT NULL DEFAULT '{}',          -- what sync replays; never contains password hashes
  at           TEXT NOT NULL
);
CREATE INDEX ix_change_log_entity ON change_log(entity, entity_id);
CREATE INDEX ix_change_log_user ON change_log(user_id, seq);

CREATE TABLE field_clocks (
  entity    TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  field     TEXT NOT NULL,
  hlc       TEXT NOT NULL,
  device_id TEXT NOT NULL,
  PRIMARY KEY (entity, entity_id, field)
);

CREATE TABLE field_history (
  id                 TEXT PRIMARY KEY,
  entity             TEXT NOT NULL,
  entity_id          TEXT NOT NULL,
  field              TEXT NOT NULL,
  losing_value_json  TEXT NOT NULL,
  winning_value_json TEXT NOT NULL,
  losing_hlc         TEXT NOT NULL,
  winning_hlc        TEXT NOT NULL,
  at                 TEXT NOT NULL,
  undone_at          TEXT,
  undone_by          TEXT
);

CREATE TABLE alerts (
  id          TEXT PRIMARY KEY,
  kind        TEXT NOT NULL,          -- overpayment, attendance_replaced, backup_overdue, clock_skew
  entity      TEXT NOT NULL,
  entity_id   TEXT NOT NULL,
  message_key TEXT NOT NULL,
  params_json TEXT NOT NULL DEFAULT '{}',
  created_at  TEXT NOT NULL,
  resolved_at TEXT,
  resolved_by TEXT
);

-- ---------- devices (office computer) ----------
CREATE TABLE server_identity (
  id              INTEGER PRIMARY KEY CHECK (id = 1),
  cert_der        BLOB NOT NULL,
  key_der         BLOB NOT NULL,       -- protected by database encryption
  fingerprint_hex TEXT NOT NULL,
  created_at      TEXT NOT NULL
);

CREATE TABLE devices (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  platform       TEXT NOT NULL,
  public_key     BLOB NOT NULL UNIQUE,
  user_id        TEXT NOT NULL REFERENCES users(id),
  receipt_prefix TEXT NOT NULL UNIQUE CHECK (length(receipt_prefix) BETWEEN 2 AND 4),
  approved_by    TEXT NOT NULL REFERENCES users(id),
  approved_at    TEXT NOT NULL,
  last_seen_at   TEXT,
  last_pull_seq  INTEGER NOT NULL DEFAULT 0,
  needs_resync   INTEGER NOT NULL DEFAULT 0 CHECK (needs_resync IN (0,1)),
  revoked_at     TEXT,
  revoked_by     TEXT
);

CREATE TABLE pending_approvals (
  id                TEXT PRIMARY KEY,
  user_id           TEXT NOT NULL REFERENCES users(id),
  device_public_key BLOB NOT NULL,
  device_name       TEXT NOT NULL,
  platform          TEXT NOT NULL,
  match_code        TEXT NOT NULL,     -- "482913"
  created_at        TEXT NOT NULL,
  expires_at        TEXT NOT NULL,
  decided_at        TEXT,
  decision          TEXT CHECK (decision IN ('allowed','refused','expired'))
);

CREATE TABLE used_request_nonces (
  device_id TEXT NOT NULL,
  nonce     TEXT NOT NULL,
  seen_at   TEXT NOT NULL,
  PRIMARY KEY (device_id, nonce)
);

-- ---------- phone only ----------
CREATE TABLE outbox (
  change_id       TEXT PRIMARY KEY,
  local_seq       INTEGER NOT NULL UNIQUE,
  envelope_json   TEXT NOT NULL,
  created_at      TEXT NOT NULL,
  sent_at         TEXT,
  acked_at        TEXT,
  rejected_reason TEXT
);

CREATE TABLE sync_state (
  id                 INTEGER PRIMARY KEY CHECK (id = 1),
  school_code        TEXT NOT NULL,
  server_fingerprint TEXT NOT NULL,
  server_address     TEXT,
  device_id          TEXT NOT NULL,
  last_pull_seq      INTEGER NOT NULL DEFAULT 0,
  last_sync_at       TEXT,
  clock_offset_ms    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE auth_cache (
  user_id       TEXT PRIMARY KEY,
  username      TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

-- ---------- backups (office computer) ----------
CREATE TABLE backups_log (
  id          TEXT PRIMARY KEY,
  kind        TEXT NOT NULL CHECK (kind IN ('daily','monthly','manual','pendrive','safety','verify')),
  location    TEXT NOT NULL,          -- local file path
  created_at  TEXT NOT NULL,
  size_bytes  INTEGER,
  verified_at TEXT,
  error       TEXT
);

-- ---------- append-only protection ----------
CREATE TRIGGER trg_receipts_no_update BEFORE UPDATE ON receipts BEGIN SELECT RAISE(ABORT, 'receipts are append-only'); END;
CREATE TRIGGER trg_receipts_no_delete BEFORE DELETE ON receipts BEGIN SELECT RAISE(ABORT, 'receipts are append-only'); END;
CREATE TRIGGER trg_cancel_no_update BEFORE UPDATE ON receipt_cancellations BEGIN SELECT RAISE(ABORT, 'receipt cancellations are append-only'); END;
CREATE TRIGGER trg_cancel_no_delete BEFORE DELETE ON receipt_cancellations BEGIN SELECT RAISE(ABORT, 'receipt cancellations are append-only'); END;
CREATE TRIGGER trg_tc_no_update BEFORE UPDATE ON transfer_certificates BEGIN SELECT RAISE(ABORT, 'transfer certificates are append-only'); END;
CREATE TRIGGER trg_tc_no_delete BEFORE DELETE ON transfer_certificates BEGIN SELECT RAISE(ABORT, 'transfer certificates are append-only'); END;
CREATE TRIGGER trg_log_no_update BEFORE UPDATE ON change_log BEGIN SELECT RAISE(ABORT, 'change log is append-only'); END;
CREATE TRIGGER trg_log_no_delete BEFORE DELETE ON change_log BEGIN SELECT RAISE(ABORT, 'change log is append-only'); END;
```

## Derived values (never stored, always computed in vidya-core)
| Value | Formula |
|---|---|
| Term fee | `tuition + exam + other` for the class in the session |
| Session fee due | RTE → 0; otherwise `(term fee + (transport ? transport_fee_per_term : 0)) × terms − concession`, not below 0 |
| Paid | Sum of `receipts.amount` for the enrollment's session and student, excluding cancelled receipts |
| Balance | `max(0, due − paid)` |
| Fee state | `rte` if RTE; `paid` if paid ≥ due; `part` if paid > 0; otherwise `due` |
| Exam result | For each active subject with a mark row: max += exam.max_marks; got += value (AB counts 0). Percent = got / max × 100 rounded to 1 decimal (use integer maths: `round(got × 1000 / max) / 10`) |
| Grade | Highest grade whose `min_percent` ≤ percent |
| Attendance % | Present marks ÷ all marks for the student in the date range, rounded to whole number |
| Previous session dues | Balance of the student's enrollment in the previous session |

## Default data created by setup
- `grade_scale`: A 80, B 65, C 50, D 33, E 0.
- `exams` for the first session: Unit Test 1 (25), Half Yearly (100), Unit Test 2 (25), Annual (100).
- Subjects by class: Nursery, LKG, UKG → Hindi, English, Numbers, Drawing. I to V → Hindi, English, Mathematics, EVS, Drawing. VI to VIII → Hindi, English, Mathematics, Science, Social Science, Computer. IX to XII → Hindi, English, Mathematics, Science, Social Science.
- `counters`: `adm_no` 0, `tc_no` 0, `receipt:PC` 0.
- `app_settings`: session_timeout_minutes 30, receipt_paper a4, print_language en, keep_running_in_tray 1, start_at_login 1, keep_awake 1, backup_staff_can_run "1", backup_destinations "[]" (JSON array).
