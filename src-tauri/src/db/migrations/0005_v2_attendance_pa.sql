-- 0005_v2_attendance_pa.sql — Phase 11 (v2): Attendance Present/Absent cutover.
--
-- Additive only. Records the v2 cutover time; it makes NO change to any existing
-- attendance_mark row (legacy L marks stay exactly as stored, 00-SYSTEM-CONTEXT
-- §7a). From the cutover on, vidya-core rejects a new L on the write path, and
-- the sync engine rejects a synced op carrying L whose HLC is at/after the
-- cutover with reason code LEAVE_MARK_REMOVED (an out-of-date phone).

-- A small key/value store for schema-level facts that are not per-row data.
CREATE TABLE IF NOT EXISTS schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- The cutover constant. Stored as a readable ISO string AND as Unix milliseconds
-- so the sync engine can compare it against an op's HLC (whose leading 13 digits
-- are wall_ms). Computed from a fixed literal, so it is deterministic (no clock
-- read at migration time). SQLite reads bare date-time literals as UTC.
INSERT OR IGNORE INTO schema_meta(key, value) VALUES
  ('attendance_pa_cutover_iso', '2026-09-25T00:00:00Z'),
  ('attendance_pa_cutover_ms',  CAST(strftime('%s', '2026-09-25 00:00:00') AS INTEGER) * 1000);
