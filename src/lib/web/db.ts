// IndexedDB schema for the iPhone PWA (Phase 19, Step 3). One object store per
// synced client table the phone needs (scoped exactly like the Android client — see
// src-tauri/src/sync/scope.rs), plus the sync bookkeeping stores. Every record's
// VALUE is encrypted at rest (crypto.ts); the store name + primary key are the only
// things in clear, so IndexedDB never holds a plaintext name, mark or amount.
//
// This module only opens the database and declares the stores; reads/writes go
// through store.ts (encrypt/decrypt + one-transaction row+op+audit writes).

export const DB_NAME = 'vidya'
export const DB_VERSION = 1

/**
 * Synced record tables the staff phone client reads/writes (keyed by `id`). This is
 * the PWA-visible subset of the §7 data model — never the school-server-only tables
 * (op_log, drive_state, backup_run, licence, invite, device registry, …).
 */
export const RECORD_STORES = [
  // students / classes / staff / guardians
  'school', 'academic_session', 'term', 'class', 'subject', 'class_subject', 'staff',
  'student', 'enrollment', 'guardian', 'student_guardian',
  // attendance + marks (teacher)
  'attendance_sheet', 'attendance_mark', 'exam', 'exam_subject', 'marks_sheet', 'mark_entry',
  'grade_scale', 'grade_band',
  // fees (accountant collect-fee) — payments are append-only
  'fee_head', 'fee_due', 'payment', 'payment_allocation',
  // classroom
  'period', 'timetable_slot', 'substitution', 'homework_note', 'report_remark',
  'report_template', 'exam_room', 'exam_seat', 'exam_schedule',
  // communication + calendar
  'circular', 'circular_read', 'message', 'message_template', 'calendar_event', 'school_week',
  // staff HR
  'leave_type', 'leave_record', 'staff_attendance',
  // approvals / notifications / audit (kept locally, append-only)
  'request', 'notification', 'audit_log',
] as const

export type RecordStore = (typeof RECORD_STORES)[number]

/** Bookkeeping stores (not encrypted-record-shaped; see store.ts for their layout). */
export const OUTBOX = 'outbox' // pending ops to push (keyed by op_id)
export const APPLIED_OPS = 'applied_ops' // op_ids already applied (idempotency)
export const CURSORS = 'sync_cursor' // per-peer last hlc / server_seq
export const ATTACHMENTS = 'attachments' // compressed blobs, keyed by sha-256 hash
export const KV = 'kv' // device token, session key, epoch, lease, Drive tokens, meta

let dbPromise: Promise<IDBDatabase> | null = null

/** Open (creating/upgrading) the encrypted local database. Cached per page. */
export function openDb(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      for (const name of RECORD_STORES) {
        if (!db.objectStoreNames.contains(name)) db.createObjectStore(name, { keyPath: 'id' })
      }
      if (!db.objectStoreNames.contains(OUTBOX)) db.createObjectStore(OUTBOX, { keyPath: 'op_id' })
      if (!db.objectStoreNames.contains(APPLIED_OPS)) db.createObjectStore(APPLIED_OPS, { keyPath: 'op_id' })
      if (!db.objectStoreNames.contains(CURSORS)) db.createObjectStore(CURSORS, { keyPath: 'peer' })
      if (!db.objectStoreNames.contains(ATTACHMENTS)) db.createObjectStore(ATTACHMENTS, { keyPath: 'hash' })
      if (!db.objectStoreNames.contains(KV)) db.createObjectStore(KV, { keyPath: 'key' })
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
  return dbPromise
}

/** Promisify a single IDBRequest. */
export function reqDone<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

/** Run `fn` in one readwrite transaction over `stores`, resolving when it commits. */
export function tx(
  db: IDBDatabase,
  stores: string[],
  mode: IDBTransactionMode,
  fn: (t: IDBTransaction) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(stores, mode)
    t.oncomplete = () => resolve()
    t.onerror = () => reject(t.error)
    t.onabort = () => reject(t.error ?? new Error('transaction aborted'))
    fn(t)
  })
}
