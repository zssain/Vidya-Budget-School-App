// The write path for the PWA (Phase 20, C3) — build an op + commit it with the row and
// an audit entry in one transaction (store.writeRecord), the same op format the app
// pushes (§8.1). The server applies a client op via sync::apply::apply_op → apply_upsert,
// which INSERTs the payload's columns + the record id and AUTO-ADDS the sync bookkeeping
// (hlc, created_at, updated_at, updated_by_*, sync_state). So a write op's payload carries
// the FULL set of row columns MINUS those auto-added ones (never include created_at /
// sync_state / version, or apply_upsert would duplicate the column).

import type { AuditEntry, Op } from './store'
import { type RecordStore } from './db'
import { kvGet, kvPut, writeRecord } from './store'
import { hlcNext, initWasm } from './wasm'

const HLC_KEY = 'hlc_clock'

function randHex(n: number): string {
  const b = new Uint8Array(n)
  crypto.getRandomValues(b)
  return [...b].map((x) => x.toString(16).padStart(2, '0')).join('')
}

/** A new record/op id like the Rust `new_id(prefix)` (`<prefix>-<hex>`). */
export function newId(prefix: string): string {
  return `${prefix}-${randHex(16)}`
}

// vidya_core::audience::audience_for — a byte-for-byte TS copy (the WASM does not expose
// it). The pushing device must hold the returned audience's key to seal the bundle.
const CLASS_SCOPED = new Set([
  'attendance_sheet', 'attendance_mark', 'marks_sheet', 'mark_entry', 'exam', 'exam_subject',
  'timetable_slot', 'homework_note', 'report_remark',
])
const FINANCE = new Set([
  'fee_head', 'fee_due', 'payment', 'payment_allocation', 'reversal',
  'ledger_account', 'voucher', 'ledger_entry',
  'student', 'enrollment', 'guardian', 'student_guardian', 'consent',
])
const ADMIN = new Set([
  'school', 'academic_session', 'term', 'subject', 'class', 'class_subject', 'staff', 'device',
  'invite', 'conflict', 'review_flag', 'notification', 'licence', 'grade_scale', 'grade_band',
  'school_week', 'calendar_event', 'message', 'message_template', 'custom_field', 'custom_value',
  'period', 'substitution', 'exam_room', 'exam_seat', 'exam_schedule', 'report_lock', 'report_template',
])

/** The sealing audience of an op (string form). Class-scoped tables need the class id.
 *  `request` has no table-only audience (the write site decides it → pass `audience`
 *  explicitly to `writeOp` instead of calling this). Throws on an unknown/ambiguous
 *  table so a write can never be silently sealed to the wrong audience. */
export function audienceFor(table: string, classId?: string | null): string {
  if (CLASS_SCOPED.has(table)) {
    if (!classId) throw new Error('class_id required for class audience')
    return `class:${classId}`
  }
  if (FINANCE.has(table)) return 'finance'
  if (ADMIN.has(table)) return 'admin'
  throw new Error(`no table-only audience for '${table}'`)
}

async function nextHlc(deviceId: string): Promise<string> {
  await initWasm()
  const prev = (await kvGet<string>(HLC_KEY)) ?? null
  const hlc = hlcNext(prev, Date.now(), deviceId)
  await kvPut(HLC_KEY, hlc)
  return hlc
}

/** Build the op from this device's identity + HLC clock. Exposed for unit tests. */
export function buildOp(params: {
  deviceId: string
  staffId: string
  epoch: number
  hlc: string
  table: string
  recordId: string
  kind: Op['kind']
  audience: string
  payload: Record<string, unknown>
  baseVersion: number | null
}): Op {
  return {
    op_id: newId('op'),
    hlc: params.hlc,
    device_id: params.deviceId,
    staff_id: params.staffId,
    audience: params.audience,
    table: params.table,
    record_id: params.recordId,
    kind: params.kind,
    payload: params.payload,
    base_version: params.baseVersion,
    server_epoch: params.epoch,
  }
}

/** Commit a write: the local row (optimistic copy) + its outbox op + an audit entry, in
 *  one transaction. The sync loop seals + pushes the op with the audience key held at join. */
export async function writeOp(params: {
  table: string
  recordId: string
  kind: Op['kind']
  audience: string
  payload: Record<string, unknown>
  localRow: Record<string, unknown>
  baseVersion?: number | null
  action: string
}): Promise<void> {
  const deviceId = (await kvGet<string>('device_id')) ?? ''
  const staffId = (await kvGet<{ id: string }>('staff'))?.id ?? ''
  const epoch = (await kvGet<number>('server_epoch')) ?? 1
  const hlc = await nextHlc(deviceId)
  const op = buildOp({
    deviceId,
    staffId,
    epoch,
    hlc,
    table: params.table,
    recordId: params.recordId,
    kind: params.kind,
    audience: params.audience,
    payload: params.payload,
    baseVersion: params.baseVersion ?? null,
  })
  const audit: AuditEntry = {
    at: new Date().toISOString(),
    staff_id: staffId,
    device_id: deviceId,
    action: params.action,
    table: params.table,
    record_id: params.recordId,
    op_id: op.op_id,
  }
  await writeRecord({ store: params.table as RecordStore, id: params.recordId, value: params.localRow, op, audit })
}
