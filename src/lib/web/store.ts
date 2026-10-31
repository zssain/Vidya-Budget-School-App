// The encrypted local store for the iPhone PWA (Phase 19, Step 3). Records are read/
// written through here so their values are always AES-GCM encrypted at rest
// (crypto.ts). Every write is ONE IndexedDB transaction — the row + its outbox op +
// an audit entry — using the SAME op format and HLC as the Android client (§8.1/8.2),
// so a change made on the iPhone syncs and re-validates on the school server exactly
// like one from the app.

import { APPLIED_OPS, CURSORS, KV, OUTBOX, openDb, RECORD_STORES, type RecordStore, reqDone, tx } from './db'
import { dbKey, decryptBytesWith, decryptJson, encryptBytesWith, encryptJson } from './crypto'
import { currentPinKey } from './pinkey'

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
  /** Wall ms of the last upload to Drive (absent = never uploaded). */
  uploaded_at?: number
  /** Set when the school PC refused the op: kept (not dropped) for the user to see. */
  status?: 'rejected' | 'conflict' | 'flagged'
  reason?: string | null
}

/** Re-upload an op whose upload was never acknowledged after this long. */
export const REUPLOAD_AFTER_MS = 30 * 60 * 1000

/** Which outbox rows to push now (pure → unit-tested): not refused, and either never
 *  uploaded or uploaded long enough ago with no ack (the bundle may have been lost). */
export function rowsToPush(rows: OutboxRow[], nowMs: number): OutboxRow[] {
  return rows
    .filter((r) => !r.status && (r.uploaded_at == null || nowMs - r.uploaded_at > REUPLOAD_AFTER_MS))
    .sort((a, b) => (a.hlc < b.hlc ? -1 : a.hlc > b.hlc ? 1 : 0))
}

async function allOutbox(): Promise<OutboxRow[]> {
  const db = await openDb()
  return reqDone<OutboxRow[]>(db.transaction(OUTBOX, 'readonly').objectStore(OUTBOX).getAll())
}

/** Ops still waiting for the school PC (not refused), in HLC order (§8.3). */
export async function pendingOps(): Promise<Op[]> {
  const rows = (await allOutbox()).filter((r) => !r.status)
  rows.sort((a, b) => (a.hlc < b.hlc ? -1 : a.hlc > b.hlc ? 1 : 0))
  return Promise.all(rows.map((r) => decryptJson<Op>(r.enc)))
}

/** Ops to upload on this pass (see rowsToPush). */
export async function opsToPush(nowMs: number): Promise<Op[]> {
  return Promise.all(rowsToPush(await allOutbox(), nowMs).map((r) => decryptJson<Op>(r.enc)))
}

/** Remember that these ops were uploaded at `nowMs` (so they aren't re-sent every pass). */
export async function markUploaded(opIds: string[], nowMs: number): Promise<void> {
  const ids = new Set(opIds)
  const rows = (await allOutbox()).filter((r) => ids.has(r.op_id))
  const db = await openDb()
  await tx(db, [OUTBOX], 'readwrite', (t) => {
    for (const r of rows) t.objectStore(OUTBOX).put({ ...r, uploaded_at: nowMs })
  })
}

/** One per-op result from the school PC's ack. */
export interface AckResult {
  op_id: string
  status: 'confirmed' | 'rejected' | 'conflict' | 'flagged'
  reason_code?: string | null
}

/** Apply an ack: confirmed ops leave the outbox; refused ones stay, marked with the
 *  status + reason (never silently dropped). Unknown op ids are ignored. */
export async function resolveOps(results: AckResult[]): Promise<{ confirmed: number; refused: number }> {
  const byId = new Map((await allOutbox()).map((r) => [r.op_id, r]))
  let confirmed = 0
  let refused = 0
  const db = await openDb()
  await tx(db, [OUTBOX], 'readwrite', (t) => {
    for (const res of results) {
      const row = byId.get(res.op_id)
      if (!row) continue
      if (res.status === 'confirmed') {
        t.objectStore(OUTBOX).delete(res.op_id)
        confirmed += 1
      } else {
        t.objectStore(OUTBOX).put({ ...row, status: res.status, reason: res.reason_code ?? null })
        refused += 1
      }
    }
  })
  return { confirmed, refused }
}

/** Ops the school PC refused, for the UI to surface. */
export async function refusedOps(): Promise<OutboxRow[]> {
  return (await allOutbox()).filter((r) => r.status != null)
}

/** One refused change for the UI (the desktop's SyncProblemDto + the op's table). */
export interface SyncProblem {
  id: string
  reason: string | null
  table?: string
}

/** Map refused outbox rows to SyncProblems, oldest first (pure given `openOp`). An op
 *  that can't be decrypted is still listed — just without its table. */
export async function toSyncProblems(rows: OutboxRow[], openOp: (r: OutboxRow) => Promise<Pick<Op, 'table'>>): Promise<SyncProblem[]> {
  const sorted = [...rows].filter((r) => r.status != null).sort((a, b) => (a.hlc < b.hlc ? -1 : a.hlc > b.hlc ? 1 : 0))
  return Promise.all(
    sorted.map(async (r) => {
      let table: string | undefined
      try {
        table = (await openOp(r)).table
      } catch {
        table = undefined
      }
      return { id: r.op_id, reason: r.reason ?? null, ...(table ? { table } : {}) }
    }),
  )
}

/** The refused changes for `list_sync_problems`. */
export async function syncProblems(): Promise<SyncProblem[]> {
  return toSyncProblems(await refusedOps(), (r) => decryptJson<Op>(r.enc))
}

/** One canonical row from the school PC's down export. */
export interface CanonicalRow {
  table: string
  record_id: string
  payload: Record<string, unknown>
}

/** Which rows a full replace writes (pure → unit-tested): canonical rows for the
 *  stores this app keeps, plus the local copies of rows that still have an
 *  unconfirmed outbox op (so a pending edit doesn't vanish until the school PC
 *  confirms it). */
export function planReplace(
  canonical: CanonicalRow[],
  pendingKeys: Set<string>,
  stores: readonly string[],
): CanonicalRow[] {
  const keep = canonical.filter((c) => stores.includes(c.table) && !pendingKeys.has(`${c.table}/${c.record_id}`))
  return keep
}

/** Replace ALL local school data with the school PC's export in ONE transaction
 *  (all or nothing): removed rows and lost access disappear; rows with a pending
 *  local change keep their local copy. */
export async function replaceAllRecords(canonical: CanonicalRow[]): Promise<number> {
  const db = await openDb()
  const pending = (await allOutbox()).filter((r) => !r.status)
  const pendingOps = await Promise.all(pending.map((r) => decryptJson<Op>(r.enc)))
  const pendingKeys = new Set(pendingOps.map((o) => `${o.table}/${o.record_id}`))
  // Keep the encrypted local rows behind pending ops.
  const kept: { store: string; row: unknown }[] = []
  for (const o of pendingOps) {
    if (!(RECORD_STORES as readonly string[]).includes(o.table)) continue
    const row = await reqDone(db.transaction(o.table, 'readonly').objectStore(o.table).get(o.record_id))
    if (row) kept.push({ store: o.table, row })
  }
  const rows = planReplace(canonical, pendingKeys, RECORD_STORES)
  // Encrypt BEFORE the transaction (an IDB tx must not span an await).
  const enc = await Promise.all(rows.map(async (r) => ({ store: r.table, id: r.record_id, enc: await encryptJson({ id: r.record_id, ...r.payload }) })))
  await tx(db, [...RECORD_STORES], 'readwrite', (t) => {
    for (const name of RECORD_STORES) t.objectStore(name).clear()
    for (const e of enc) t.objectStore(e.store).put({ id: e.id, enc: e.enc })
    for (const k of kept) t.objectStore(k.store).put(k.row)
  })
  return enc.length
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

/** Put several kv entries in ONE transaction (all or nothing). */
export async function kvPutMany(entries: [string, unknown][]): Promise<void> {
  if (entries.length === 0) return
  const db = await openDb()
  await tx(db, [KV], 'readwrite', (t) => {
    for (const [key, value] of entries) t.objectStore(KV).put({ key, value })
  })
}

// ---- secrets (device token, session key, audience keys, pending invite) -------
//
// Two layers (repair K2):
//   { sealed }    — AES-GCM under the non-extractable data key only. Written before a
//                   PIN exists (join) or while locked; re-wrapped at the next unlock.
//   { pinsealed } — data-key AES-GCM over an inner AES-GCM under the PIN-derived key
//                   (pinkey.ts), AAD-bound to the kv name. Needs the unlocked session.
// A legacy plain value (older build) is read once and re-stored sealed.

/** Every kv entry that holds a secret (all go through secretPut/secretGet). */
export const SECRET_KEYS = ['device_token', 'session_key', 'audience_keys', 'pending_join_fragment'] as const

interface SealedKv {
  sealed: Uint8Array
}
interface PinSealedKv {
  pinsealed: Uint8Array
}
export type StoredSecret = SealedKv | PinSealedKv

function isSealed(v: unknown): v is SealedKv {
  return typeof v === 'object' && v !== null && (v as SealedKv).sealed instanceof Uint8Array
}
export function isPinSealed(v: unknown): v is PinSealedKv {
  return typeof v === 'object' && v !== null && (v as PinSealedKv).pinsealed instanceof Uint8Array
}

const secretAad = (name: string): Uint8Array => new TextEncoder().encode('vidya-secret:' + name)
const LOCKED_ERR = { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} }

/** Seal a secret value (pure given the keys): PIN layer when a PIN key is held. */
export async function sealSecretWith(name: string, value: unknown, dataKey: CryptoKey, pinKey: CryptoKey | null): Promise<StoredSecret> {
  const json = new TextEncoder().encode(JSON.stringify(value))
  if (!pinKey) return { sealed: await encryptBytesWith(dataKey, json) }
  const inner = await encryptBytesWith(pinKey, json, secretAad(name))
  return { pinsealed: await encryptBytesWith(dataKey, inner) }
}

/** Open a stored secret (pure given the keys). A PIN-sealed value without the PIN key
 *  rejects with LOCKED; a legacy plain value is returned as is. */
export async function openSecretWith<T>(name: string, stored: unknown, dataKey: CryptoKey, pinKey: CryptoKey | null): Promise<T | undefined> {
  if (stored == null) return undefined
  if (isPinSealed(stored)) {
    if (!pinKey) throw LOCKED_ERR
    const inner = await decryptBytesWith(dataKey, stored.pinsealed)
    return JSON.parse(new TextDecoder().decode(await decryptBytesWith(pinKey, inner, secretAad(name)))) as T
  }
  if (isSealed(stored)) return JSON.parse(new TextDecoder().decode(await decryptBytesWith(dataKey, stored.sealed))) as T
  return stored as T
}

/** Re-wrap stored secrets from `fromPin` (null = no PIN layer yet) to `toPin` (pure
 *  given the keys). Values already PIN-sealed under the same key are left alone; a
 *  PIN-sealed value that `fromPin` can't open throws (never silently dropped). */
export async function rewrapValues(
  entries: [string, unknown][],
  dataKey: CryptoKey,
  fromPin: CryptoKey | null,
  toPin: CryptoKey,
): Promise<[string, StoredSecret][]> {
  const out: [string, StoredSecret][] = []
  for (const [name, stored] of entries) {
    if (stored == null) continue
    if (isPinSealed(stored) && fromPin === toPin) continue
    const value = await openSecretWith<unknown>(name, stored, dataKey, fromPin)
    out.push([name, await sealSecretWith(name, value, dataKey, toPin)])
  }
  return out
}

/** Re-wrap every stored secret under `toPin` (migration at unlock / PIN creation, or
 *  a PIN change from `fromPin`); `extra` kv entries commit in the same transaction. */
export async function rewrapSecrets(fromPin: CryptoKey | null, toPin: CryptoKey, extra: [string, unknown][] = []): Promise<number> {
  const raw: [string, unknown][] = []
  for (const name of SECRET_KEYS) raw.push([name, await kvGet<unknown>(name)])
  const rewrapped = await rewrapValues(raw, await dbKey(), fromPin, toPin)
  await kvPutMany([...rewrapped, ...extra])
  return rewrapped.length
}

/** Store a secret encrypted (null deletes it): under the PIN key too when unlocked. */
export async function secretPut(key: string, value: unknown): Promise<void> {
  if (value == null) return kvPut(key, null)
  await kvPut(key, await sealSecretWith(key, value, await dbKey(), currentPinKey()))
}

/** Read a secret. Rejects with LOCKED for a PIN-sealed value while locked (callers
 *  that run while locked — the Drive sync loop — treat that as "skip this pass"). A
 *  legacy plain or data-key-only value is re-stored under the strongest layer held. */
export async function secretGet<T>(key: string): Promise<T | undefined> {
  const v = await kvGet<unknown>(key)
  if (v == null) return undefined
  const pin = currentPinKey()
  const value = await openSecretWith<T>(key, v, await dbKey(), pin)
  if (!isPinSealed(v) && (pin || !isSealed(v))) await secretPut(key, value)
  return value
}

