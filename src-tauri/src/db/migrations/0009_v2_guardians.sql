-- 0009_v2_guardians.sql — Phase 13 (v2): guardians as a separate table (foundation §8.2).
--
-- Additive only; no committed migration is edited; no financial/audit rows touched.
-- v2 moves guardian data out of the inline student.guardian_* columns into a
-- `guardian` table linked by `student_guardian` (one primary). The old columns
-- STAY (read-only, kept in sync by create/import for compatibility until P18
-- drops them). Guardians belong to the student-roster domain → audience `finance`.

CREATE TABLE IF NOT EXISTS guardian (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  relation    TEXT,
  mobile      TEXT,
  email       TEXT,
  language    TEXT NOT NULL DEFAULT 'en' CHECK (language IN ('en','hi','te')),
  whatsapp_ok INTEGER NOT NULL DEFAULT 1 CHECK (whatsapp_ok IN (0,1)),
  school_id   TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

CREATE TABLE IF NOT EXISTS student_guardian (
  id          TEXT PRIMARY KEY,
  student_id  TEXT NOT NULL REFERENCES student(id),
  guardian_id TEXT NOT NULL REFERENCES guardian(id),
  is_primary  INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
  school_id   TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (student_id, guardian_id)
);

CREATE INDEX IF NOT EXISTS idx_student_guardian_student  ON student_guardian(student_id);
CREATE INDEX IF NOT EXISTS idx_student_guardian_guardian ON student_guardian(guardian_id);
CREATE INDEX IF NOT EXISTS idx_guardian_mobile           ON guardian(mobile);

-- Backfill existing (v1) students: one guardian per distinct (mobile, name), so
-- siblings that share a guardian's mobile+name map to the SAME guardian row.
-- The guardian id is deterministic (`grd-` || MIN(student.id) in the group) so
-- the student_guardian link below can reference it without a temp table. On a
-- fresh install there are no students yet, so this is a no-op; new admissions
-- create their guardian rows in application code (create_student / import / seed)
-- with the same find-or-create-by-(mobile,name) rule.
INSERT INTO guardian(id, name, relation, mobile, email, language, whatsapp_ok, school_id, created_at, updated_at, sync_state)
SELECT
  'grd-' || MIN(s.id),
  MAX(COALESCE(s.guardian_name, '')),
  NULL,
  NULLIF(MAX(COALESCE(s.guardian_mobile, '')), ''),
  NULL,
  'en',
  1,
  (SELECT id FROM school LIMIT 1),
  '', '', 'confirmed'
FROM student s
WHERE COALESCE(s.guardian_name, '') <> '' OR COALESCE(s.guardian_mobile, '') <> ''
GROUP BY COALESCE(s.guardian_mobile, ''), COALESCE(s.guardian_name, '');

INSERT INTO student_guardian(id, student_id, guardian_id, is_primary, school_id, created_at, updated_at, sync_state)
SELECT
  'sg-' || s.id,
  s.id,
  'grd-' || (
    SELECT MIN(s2.id) FROM student s2
    WHERE COALESCE(s2.guardian_mobile, '') = COALESCE(s.guardian_mobile, '')
      AND COALESCE(s2.guardian_name, '')   = COALESCE(s.guardian_name, '')
  ),
  1,
  (SELECT id FROM school LIMIT 1),
  '', '', 'confirmed'
FROM student s
WHERE COALESCE(s.guardian_name, '') <> '' OR COALESCE(s.guardian_mobile, '') <> '';
