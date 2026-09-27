// The encrypted local store for the iPhone PWA (Phase 19, Step 3). Records are read/
// written through here so their values are always AES-GCM encrypted at rest
// (crypto.ts). Every write is ONE IndexedDB transaction — the row + its outbox op +
// an audit entry — using the SAME op format and HLC as the Android client (§8.1/8.2),
// so a change made on the iPhone syncs and re-validates on the school server exactly
// like one from the app.

import { APPLIED_OPS, CURSORS, KV, OUTBOX, openDb, RECORD_STORES, type RecordStore, reqDone, tx } from './db'
import { decryptJson, encryptJson } from './crypto'

interface EncRow {
  id: string
  enc: Uint8Array
}

/** An op — mirrors the Android op (§8.1). Payload/HLC/audience match byte-for-byte. */
export interface Op {
  op_id: string
  hlc: string
  device_id: string
  staff_id: string
  audience: string
  table: string
  record_id: string
  kind: 'insert' | 'update' | 'delete' | 'action'
  payload: unknown
  base_version: number | null
  server_epoch: number
}

/** An audit entry (§9). Appended in the same transaction as the row + op. */
export interface AuditEntry {
  at: string
  staff_id?: string
  device_id?: string
  action: string
  table?: string
  record_id?: string
  before_json?: string
  after_json?: string
  reason?: string
  op_id?: string
}

// ---- reads -----------------------------------------------------------------

export async function getRecord<T>(store: RecordStore, id: string): Promise<T | null> {
  const db = await openDb()
  const row = await reqDone<EncRow | undefined>(db.transaction(store, 'readonly').objectStore(store).get(id))
  return row ? decryptJson<T>(row.enc) : null
}

/** All records in `store` (decrypted), optionally filtered in memory. The phone's
 *  data volume is small (a class ≈ 40 students, a month ≈ 30 days), so decrypt-scan
 *  is fine and keeps index fields off disk in the clear. */
export async function listRecords<T>(store: RecordStore, filter?: (r: T) => boolean): Promise<T[]> {
  const db = await openDb()
  const rows = await reqDone<EncRow[]>(db.transaction(store, 'readonly').objectStore(store).getAll())
  const out: T[] = []
  for (const row of rows) {
    const v = await decryptJson<T>(row.enc)
    if (!filter || filter(v)) out.push(v)
  }
  return out
}

// ---- writes (one transaction: row + op + audit) ----------------------------

export async function writeRecord(params: {
  store: RecordStore
  id: string
  value: unknown
  op: Op
  audit: AuditEntry
}): Promise<void> {
  const db = await openDb()
  // Encrypt BEFORE opening the transaction (crypto.subtle is async; an IDB tx must
  // not span an await), then commit all three puts atomically.
  const [rowEnc, opEnc, auditEnc] = await Promise.all([
    encryptJson(params.value),
    encryptJson(params.op),
    encryptJson(params.audit),
  ])
  const auditId = params.audit.op_id ?? params.op.op_id
  await tx(db, [params.store, OUTBOX, 'audit_log'], 'readwrite', (t) => {
    t.objectStore(params.store).put({ id: params.id, enc: rowEnc })
    // Outbox keeps hlc + audience in clear so the Drive route can group by audience
    // and order by hlc without decrypting; the op payload itself is encrypted.
    t.objectStore(OUTBOX).put({ op_id: params.op.op_id, hlc: params.op.hlc, audience: params.op.audience, enc: opEnc })
    t.objectStore('audit_log').put({ id: auditId, enc: auditEnc })
  })
}

// ---- outbox + cursors (for the Drive route, Step 4) ------------------------

export interface OutboxRow {
  op_id: string
  hlc: string
  audience: string
  enc: Uint8Array
}

/** Pending ops to push, in HLC order (§8.3). */
export async function pendingOps(): Promise<Op[]> {
  const db = await openDb()
  const rows = await reqDone<OutboxRow[]>(db.transaction(OUTBOX, 'readonly').objectStore(OUTBOX).getAll())
  rows.sort((a, b) => (a.hlc < b.hlc ? -1 : a.hlc > b.hlc ? 1 : 0))
  return Promise.all(rows.map((r) => decryptJson<Op>(r.enc)))
}

/** Drop ops the server has acknowledged. */
export async function ackOps(opIds: string[]): Promise<void> {
  const db = await openDb()
  await tx(db, [OUTBOX], 'readwrite', (t) => {
    for (const id of opIds) t.objectStore(OUTBOX).delete(id)
  })
}

/** Record that a peer op was applied (idempotency by op_id, §8.4). */
export async function markApplied(opId: string, result: string): Promise<void> {
  const db = await openDb()
  await tx(db, [APPLIED_OPS], 'readwrite', (t) => {
    t.objectStore(APPLIED_OPS).put({ op_id: opId, applied_at: new Date().toISOString(), result })
  })
}

export async function wasApplied(opId: string): Promise<boolean> {
  const db = await openDb()
  const row = await reqDone(db.transaction(APPLIED_OPS, 'readonly').objectStore(APPLIED_OPS).get(opId))
  return row != null
}

/** Apply a peer's op received via Drive (provisional, §8.3): upsert/delete the row
 *  and mark the op applied — one transaction, idempotent by op_id. Does NOT create a
 *  new outbox op (this op did not originate here). An unknown table is skipped. */
export async function applyRemoteOp(op: Op): Promise<'applied' | 'duplicate' | 'skipped'> {
  if (await wasApplied(op.op_id)) return 'duplicate'
  if (!(RECORD_STORES as readonly string[]).includes(op.table)) {
    await markApplied(op.op_id, 'unknown_table')
    return 'skipped'
  }
  const store = op.table as RecordStore
  const db = await openDb()
  const appliedRow = { op_id: op.op_id, applied_at: new Date().toISOString(), result: 'applied' }
  if (op.kind === 'delete') {
    await tx(db, [store, APPLIED_OPS], 'readwrite', (t) => {
      t.objectStore(store).delete(op.record_id)
      t.objectStore(APPLIED_OPS).put(appliedRow)
    })
  } else {
    const enc = await encryptJson(op.payload)
    await tx(db, [store, APPLIED_OPS], 'readwrite', (t) => {
      t.objectStore(store).put({ id: op.record_id, enc })
      t.objectStore(APPLIED_OPS).put(appliedRow)
    })
  }
  return 'applied'
}

export async function getCursor(peer: string): Promise<{ last_hlc?: string; last_server_seq?: number } | null> {
  const db = await openDb()
  const row = await reqDone<{ peer: string; last_hlc?: string; last_server_seq?: number } | undefined>(
    db.transaction(CURSORS, 'readonly').objectStore(CURSORS).get(peer),
  )
  return row ?? null
}

export async function setCursor(peer: string, last_hlc?: string, last_server_seq?: number): Promise<void> {
  const db = await openDb()
  await tx(db, [CURSORS], 'readwrite', (t) => {
    t.objectStore(CURSORS).put({ peer, last_hlc, last_server_seq })
  })
}

// ---- kv (device token, session key, epoch, lease, Drive tokens, meta) ------
// Small values are stored in the clear (ids/timestamps) except secrets, which are
// encrypted with the same db key. Handlers choose per key.

export async function kvGet<T>(key: string): Promise<T | undefined> {
  const db = await openDb()
  const row = await reqDone<{ key: string; value: T } | undefined>(
    db.transaction(KV, 'readonly').objectStore(KV).get(key),
  )
  return row?.value
}

export async function kvPut(key: string, value: unknown): Promise<void> {
  const db = await openDb()
  await tx(db, [KV], 'readwrite', (t) => {
    t.objectStore(KV).put({ key, value })
  })
}
