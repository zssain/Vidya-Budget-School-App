-- Vidya initial schema (docs/00-SYSTEM-CONTEXT.md §7). Applied in one transaction.
-- Money = integer paise (i64). ids = UUIDv7 text. times = UTC ISO-8601 text.
-- dates = 'YYYY-MM-DD'. Every synced table carries the sync columns (version, hlc,
-- created_at, updated_at, updated_by_staff, updated_by_device, sync_state).

-- ---- schema version ------------------------------------------------------
CREATE TABLE schema_version (
  version   INTEGER NOT NULL,
  applied_at TEXT   NOT NULL
);

-- ---- school / licence / session ------------------------------------------
CREATE TABLE school (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  address       TEXT,
  board         TEXT,
  udise         TEXT,
  logo_blob     BLOB,
  backup_salt   BLOB NOT NULL,
  server_epoch  INTEGER NOT NULL DEFAULT 1,
  settings_json TEXT NOT NULL DEFAULT '{}',
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
    CHECK (sync_state IN ('draft','on_device','shared_drive','confirmed','rejected','conflict'))
);

CREATE TABLE licence (
  licence_id   TEXT PRIMARY KEY,
  school_id    TEXT NOT NULL REFERENCES school(id),
  plan         TEXT NOT NULL,
  issued_at    TEXT NOT NULL,
  max_students INTEGER,
  max_devices  INTEGER,
  signature    TEXT NOT NULL,
  raw_json     TEXT NOT NULL,
  last_check_at TEXT,
  status       TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','revoked','moved'))
);

CREATE TABLE academic_session (
  id         TEXT PRIMARY KEY,
  label      TEXT NOT NULL,             -- "2026–27"
  starts_on  TEXT NOT NULL,
  ends_on    TEXT NOT NULL,
  is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0,1)),
  read_only  INTEGER NOT NULL DEFAULT 0 CHECK (read_only IN (0,1))
);

CREATE TABLE term (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES academic_session(id),
  name       TEXT NOT NULL,
  starts_on  TEXT NOT NULL,
  ends_on    TEXT NOT NULL
);

-- ---- classes / subjects --------------------------------------------------
CREATE TABLE class (
  id               TEXT PRIMARY KEY,
  name             TEXT NOT NULL,
  section          TEXT,
  display          TEXT NOT NULL,       -- "VII-B"
  class_teacher_id TEXT REFERENCES staff(id),
  sort_order       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE subject (
  id      TEXT PRIMARY KEY,
  name    TEXT NOT NULL,
  name_hi TEXT
);

CREATE TABLE class_subject (
  id         TEXT PRIMARY KEY,
  class_id   TEXT NOT NULL REFERENCES class(id),
  subject_id TEXT NOT NULL REFERENCES subject(id),
  teacher_id TEXT REFERENCES staff(id)
);

-- ---- staff / devices / invites -------------------------------------------
CREATE TABLE staff (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  role         TEXT NOT NULL CHECK (role IN ('principal','accountant','teacher')),
  mobile       TEXT,
  google_email TEXT,
  pin_hash     TEXT,
  pin_fail_count   INTEGER NOT NULL DEFAULT 0,
  pin_locked_until TEXT,
  state        TEXT NOT NULL DEFAULT 'invited'
    CHECK (state IN ('invited','active','suspended','removed')),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE TABLE device (
  id               TEXT PRIMARY KEY,
  staff_id         TEXT NOT NULL REFERENCES staff(id),
  platform         TEXT NOT NULL CHECK (platform IN ('windows','macos','android')),
  name             TEXT NOT NULL,
  token_hash       TEXT NOT NULL,
  receipt_series   TEXT,
  admission_series TEXT,
  last_seen_at     TEXT,
  revoked_at       TEXT,
  lease_expires_at TEXT,
  needs_rejoin     INTEGER NOT NULL DEFAULT 0 CHECK (needs_rejoin IN (0,1))
);

CREATE TABLE invite (
  id         TEXT PRIMARY KEY,
  code_hash  TEXT NOT NULL,
  staff_id   TEXT NOT NULL REFERENCES staff(id),
  expires_at TEXT NOT NULL,
  used_at    TEXT,
  revoked_at TEXT
);

-- ---- students / enrollment -----------------------------------------------
CREATE TABLE student (
  id             TEXT PRIMARY KEY,
  admission_no   TEXT,                  -- NULL until confirmed
  provisional_no TEXT,
  name           TEXT NOT NULL,
  dob            TEXT,
  gender         TEXT,
  guardian_name  TEXT,
  guardian_mobile TEXT,
  address        TEXT,
  transport      INTEGER NOT NULL DEFAULT 0 CHECK (transport IN (0,1)),
  category       TEXT,
  rte            INTEGER NOT NULL DEFAULT 0 CHECK (rte IN (0,1)),
  aadhaar_status TEXT NOT NULL DEFAULT 'none'
    CHECK (aadhaar_status IN ('none','submitted','verified')),
  status         TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','left')),
  left_on        TEXT,
  left_reason    TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE TABLE enrollment (
  id         TEXT PRIMARY KEY,
  student_id TEXT NOT NULL REFERENCES student(id),
  class_id   TEXT NOT NULL REFERENCES class(id),
  session_id TEXT NOT NULL REFERENCES academic_session(id),
  roll_no    INTEGER,
  from_date  TEXT NOT NULL,
  to_date    TEXT,                      -- a section transfer closes one row, opens another
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

-- ---- attendance ----------------------------------------------------------
CREATE TABLE attendance_sheet (
  id           TEXT PRIMARY KEY,
  class_id     TEXT NOT NULL REFERENCES class(id),
  date         TEXT NOT NULL,
  status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','submitted')),
  submitted_by TEXT REFERENCES staff(id),
  submitted_at TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device',
  UNIQUE (class_id, date)
);

CREATE TABLE attendance_mark (
  id         TEXT PRIMARY KEY,
  sheet_id   TEXT NOT NULL REFERENCES attendance_sheet(id),
  student_id TEXT NOT NULL REFERENCES student(id),
  mark       TEXT NOT NULL CHECK (mark IN ('P','A','L')),
  UNIQUE (sheet_id, student_id)
);

-- ---- exams / marks -------------------------------------------------------
CREATE TABLE exam (
  id         TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES academic_session(id),
  term_id    TEXT REFERENCES term(id),
  name       TEXT NOT NULL,
  starts_on  TEXT,
  ends_on    TEXT
);

CREATE TABLE exam_subject (
  id               TEXT PRIMARY KEY,
  exam_id          TEXT NOT NULL REFERENCES exam(id),
  class_subject_id TEXT NOT NULL REFERENCES class_subject(id),
  max_marks        INTEGER NOT NULL CHECK (max_marks > 0)
);

CREATE TABLE marks_sheet (
  id              TEXT PRIMARY KEY,
  exam_subject_id TEXT NOT NULL REFERENCES exam_subject(id),
  status          TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','submitted')),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device',
  UNIQUE (exam_subject_id)
);

CREATE TABLE mark_entry (
  id         TEXT PRIMARY KEY,
  sheet_id   TEXT NOT NULL REFERENCES marks_sheet(id),
  student_id TEXT NOT NULL REFERENCES student(id),
  marks      INTEGER,                   -- NULL = not entered, never zero
  absent     INTEGER NOT NULL DEFAULT 0 CHECK (absent IN (0,1)),
  UNIQUE (sheet_id, student_id)
);

-- ---- grade scale ---------------------------------------------------------
CREATE TABLE grade_scale (
  id       TEXT PRIMARY KEY,
  name     TEXT NOT NULL,
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0,1))
);

CREATE TABLE grade_band (
  id          TEXT PRIMARY KEY,
  scale_id    TEXT NOT NULL REFERENCES grade_scale(id),
  min_pct     INTEGER NOT NULL,         -- tenths of a percent
  max_pct     INTEGER NOT NULL,
  grade       TEXT NOT NULL,
  grade_point INTEGER
);

-- ---- fees ----------------------------------------------------------------
CREATE TABLE fee_head (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  name_hi       TEXT,
  amount_paise  INTEGER NOT NULL CHECK (amount_paise >= 0),
  frequency     TEXT NOT NULL CHECK (frequency IN ('term','month','once')),
  applies_to    TEXT NOT NULL DEFAULT 'all',   -- 'all' | 'transport' | JSON class_ids
  active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))
);

CREATE TABLE fee_due (
  id           TEXT PRIMARY KEY,
  student_id   TEXT NOT NULL REFERENCES student(id),
  fee_head_id  TEXT NOT NULL REFERENCES fee_head(id),
  period       TEXT NOT NULL,
  amount_paise INTEGER NOT NULL CHECK (amount_paise >= 0),
  cancelled_at TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE TABLE payment (
  id           TEXT PRIMARY KEY,
  receipt_no   TEXT NOT NULL,
  student_id   TEXT NOT NULL REFERENCES student(id),
  amount_paise INTEGER NOT NULL CHECK (amount_paise > 0),
  mode         TEXT NOT NULL CHECK (mode IN ('cash','upi','cheque')),
  reference    TEXT,
  collected_by TEXT NOT NULL REFERENCES staff(id),
  collected_at TEXT NOT NULL,
  device_id    TEXT REFERENCES device(id),
  confirmed_at TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device',
  UNIQUE (receipt_no)
);

CREATE TABLE payment_allocation (
  id           TEXT PRIMARY KEY,
  payment_id   TEXT NOT NULL REFERENCES payment(id),
  fee_due_id   TEXT REFERENCES fee_due(id),
  amount_paise INTEGER NOT NULL CHECK (amount_paise > 0),
  kind         TEXT NOT NULL CHECK (kind IN ('due','advance_credit'))
);

CREATE TABLE reversal (
  id         TEXT PRIMARY KEY,
  payment_id TEXT NOT NULL REFERENCES payment(id),
  reason     TEXT NOT NULL,
  request_id TEXT REFERENCES request(id),
  approved_by TEXT REFERENCES staff(id),
  applied_at TEXT NOT NULL
);

-- ---- requests / conflicts / flags / notifications ------------------------
CREATE TABLE request (
  id                TEXT PRIMARY KEY,
  type              TEXT NOT NULL CHECK (type IN
    ('marks_correction','attendance_correction','student_details','payment_reversal','access_change','device_replacement')),
  target_table      TEXT NOT NULL,
  target_id         TEXT NOT NULL,
  base_version      INTEGER NOT NULL,
  before_json       TEXT,
  after_json        TEXT,
  reason            TEXT NOT NULL,
  requested_by      TEXT NOT NULL REFERENCES staff(id),
  revision          INTEGER NOT NULL DEFAULT 1,
  parent_request_id TEXT REFERENCES request(id),
  status            TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending','approved','rejected','returned','cancelled')),
  decided_by        TEXT REFERENCES staff(id),
  decided_at        TEXT,
  note              TEXT,
  apply_state       TEXT NOT NULL DEFAULT 'not_applied'
    CHECK (apply_state IN ('not_applied','applied','failed','stale')),
  applied_at        TEXT,
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL, updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE TABLE conflict (
  id             TEXT PRIMARY KEY,
  "table"        TEXT NOT NULL,
  record_id      TEXT NOT NULL,
  field          TEXT NOT NULL,
  value_a        TEXT, hlc_a TEXT, device_a TEXT, staff_a TEXT,
  value_b        TEXT, hlc_b TEXT, device_b TEXT, staff_b TEXT,
  status         TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
  resolved_by    TEXT REFERENCES staff(id),
  resolution_op_id TEXT
);

CREATE TABLE review_flag (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN
    ('excess_payment','possible_duplicate_payment','revoked_author','unknown_record')),
  ref_table    TEXT NOT NULL,
  ref_id       TEXT NOT NULL,
  details_json TEXT,
  status       TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
  resolved_by  TEXT REFERENCES staff(id)
);

CREATE TABLE notification (
  id        TEXT PRIMARY KEY,
  staff_id  TEXT NOT NULL REFERENCES staff(id),
  kind      TEXT NOT NULL,
  title_key TEXT NOT NULL,
  vars_json TEXT,
  link      TEXT,
  read_at   TEXT
);

-- ---- audit + sync bookkeeping --------------------------------------------
CREATE TABLE audit_log (
  seq        INTEGER PRIMARY KEY AUTOINCREMENT,
  at         TEXT NOT NULL,
  staff_id   TEXT,
  device_id  TEXT,
  action     TEXT NOT NULL,
  "table"    TEXT,
  record_id  TEXT,
  before_json TEXT,
  after_json  TEXT,
  reason     TEXT,
  op_id      TEXT,
  prev_hash  TEXT NOT NULL,
  hash       TEXT NOT NULL
);

CREATE TABLE outbox (
  op_id       TEXT PRIMARY KEY,
  hlc         TEXT NOT NULL,
  device_id   TEXT NOT NULL,
  staff_id    TEXT NOT NULL,
  audience    TEXT NOT NULL,
  "table"     TEXT NOT NULL,
  record_id   TEXT NOT NULL,
  kind        TEXT NOT NULL CHECK (kind IN ('insert','update','delete','action')),
  payload     TEXT NOT NULL,
  base_version INTEGER,
  server_epoch INTEGER NOT NULL
);

CREATE TABLE op_log (
  server_seq  INTEGER PRIMARY KEY AUTOINCREMENT,
  op_id       TEXT NOT NULL UNIQUE,
  hlc         TEXT NOT NULL,
  device_id   TEXT NOT NULL,
  staff_id    TEXT NOT NULL,
  "table"     TEXT NOT NULL,
  record_id   TEXT NOT NULL,
  kind        TEXT NOT NULL,
  payload     TEXT NOT NULL,
  applied_at  TEXT NOT NULL
);

CREATE TABLE applied_ops (
  op_id      TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  result     TEXT
);

CREATE TABLE sync_cursor (
  peer       TEXT PRIMARY KEY,
  last_hlc   TEXT,
  last_server_seq INTEGER,
  updated_at TEXT
);

CREATE TABLE drive_state (
  id         TEXT PRIMARY KEY,
  key        TEXT NOT NULL UNIQUE,
  value_json TEXT
);

CREATE TABLE backup_run (
  id          TEXT PRIMARY KEY,
  started_at  TEXT NOT NULL,
  finished_at TEXT,
  status      TEXT NOT NULL,
  destination TEXT,
  checksum    TEXT,
  chain_head  TEXT
);

-- ---- indexes (prompts/P02 Step 3.3) --------------------------------------
CREATE UNIQUE INDEX idx_student_admission_no ON student(admission_no) WHERE admission_no IS NOT NULL;
CREATE INDEX idx_enrollment_class   ON enrollment(class_id);
CREATE INDEX idx_enrollment_student ON enrollment(student_id);
CREATE INDEX idx_fee_due_student    ON fee_due(student_id);
CREATE INDEX idx_payment_student    ON payment(student_id);
CREATE INDEX idx_payment_collected  ON payment(collected_at);
CREATE INDEX idx_request_status     ON request(status);
CREATE INDEX idx_outbox_hlc         ON outbox(hlc);

-- ---- full-text search on students (Devanagari-friendly) ------------------
CREATE VIRTUAL TABLE student_fts USING fts5(
  name, admission_no, provisional_no, guardian_name, guardian_mobile,
  content='student', content_rowid='rowid'
);
CREATE TRIGGER student_ai AFTER INSERT ON student BEGIN
  INSERT INTO student_fts(rowid, name, admission_no, provisional_no, guardian_name, guardian_mobile)
  VALUES (new.rowid, new.name, new.admission_no, new.provisional_no, new.guardian_name, new.guardian_mobile);
END;
CREATE TRIGGER student_ad AFTER DELETE ON student BEGIN
  INSERT INTO student_fts(student_fts, rowid, name, admission_no, provisional_no, guardian_name, guardian_mobile)
  VALUES ('delete', old.rowid, old.name, old.admission_no, old.provisional_no, old.guardian_name, old.guardian_mobile);
END;
CREATE TRIGGER student_au AFTER UPDATE ON student BEGIN
  INSERT INTO student_fts(student_fts, rowid, name, admission_no, provisional_no, guardian_name, guardian_mobile)
  VALUES ('delete', old.rowid, old.name, old.admission_no, old.provisional_no, old.guardian_name, old.guardian_mobile);
  INSERT INTO student_fts(rowid, name, admission_no, provisional_no, guardian_name, guardian_mobile)
  VALUES (new.rowid, new.name, new.admission_no, new.provisional_no, new.guardian_name, new.guardian_mobile);
END;

-- ---- append-only guards (prompts/P02 Step 3.5) ---------------------------
CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER payment_no_update BEFORE UPDATE ON payment
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER payment_no_delete BEFORE DELETE ON payment
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER reversal_no_update BEFORE UPDATE ON reversal
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER reversal_no_delete BEFORE DELETE ON reversal
  BEGIN SELECT RAISE(ABORT, 'append-only'); END;
