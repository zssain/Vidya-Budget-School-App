// The Drive sync route in the browser (Phase 19, Step 4). Pushes sealed `.vop`
// bundles to exchange/ops-<device_id>/, reads the school PC's acks, and pulls peers'
// bundles for audiences this device holds a key for — applying them PROVISIONALLY
// (§8.3). Sealing is the WASM (byte-identical to the app); the audience keys and the
// device/school identity come from the join (Step 5, stored in kv). Runtime is
// verified by the Playwright/fake-Drive harness (Step 9); this module is pure logic
// over the DriveApi interface, so the harness can drive it with FakeDrive.

import type { DriveApi } from './api'
import { ACKS, EXCHANGE, opsFolder } from './api'
import { bundleAad, seal, sealOpen } from '../wasm'
import { ackOps, applyRemoteOp, kvGet, type Op, pendingOps } from '../store'

const MAX_OPS_PER_BUNDLE = 500 // matches vidya-core/Rust (§11)
const enc = new TextEncoder()
const dec = new TextDecoder()

/** audience → base64 32-byte key, populated at join (§8.8). */
type AudienceKeys = Record<string, string>

interface ExchangeCtx {
  drive: DriveApi
  exchangeId: string
  deviceId: string
  keys: AudienceKeys
  keyVersion: number
}

function b64ToBytes(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
}

/** Resolve (creating) Vidya/<school> (<school_id>)/exchange; return its folder id. */
export async function resolveExchange(drive: DriveApi, schoolName: string, schoolId: string): Promise<string> {
  const vidya = await drive.ensureFolder(drive.root(), 'Vidya')
  const school = await drive.ensureFolder(vidya, `${schoolName} (${schoolId})`)
  return drive.ensureFolder(school, EXCHANGE)
}

/** Push pending ops: group by audience (already HLC-ordered), seal each ≤500-op
 *  bundle, upload to a temp name then rename (§11 atomic publish). */
export async function pushOutbox(ctx: ExchangeCtx): Promise<number> {
  const ops = await pendingOps()
  if (ops.length === 0) return 0
  const myOps = await ctx.drive.ensureFolder(ctx.exchangeId, opsFolder(ctx.deviceId))
  const byAud = new Map<string, Op[]>()
  for (const op of ops) {
    if (!ctx.keys[op.audience]) continue // no key for this audience → skip
    const arr = byAud.get(op.audience) ?? []
    arr.push(op)
    byAud.set(op.audience, arr)
  }
  let pushed = 0
  for (const [audience, group] of byAud) {
    const key = b64ToBytes(ctx.keys[audience])
    for (let i = 0; i < group.length; i += MAX_OPS_PER_BUNDLE) {
      const chunk = group.slice(i, i + MAX_OPS_PER_BUNDLE)
      const sealed = seal(key, bundleAad(audience, ctx.keyVersion), enc.encode(JSON.stringify(chunk)))
      const hlc = chunk[chunk.length - 1].hlc
      const finalName = `${hlc}-${audience}-v${ctx.keyVersion}.vop`
      const id = await ctx.drive.upload(myOps, `.tmp-${finalName}`, sealed, {
        properties: { vidya_audience: audience, vidya_keyver: String(ctx.keyVersion) },
      })
      await ctx.drive.rename(id, finalName)
      pushed += chunk.length
    }
  }
  return pushed
}

/** Read the school PC's ack file for this device and drop acked ops from the outbox. */
export async function readAcks(ctx: ExchangeCtx): Promise<number> {
  const acksFolder = await ctx.drive.ensureFolder(ctx.exchangeId, ACKS)
  const file = await ctx.drive.findChild(acksFolder, `${ctx.deviceId}.json`)
  if (!file) return 0
  const bytes = await ctx.drive.download(file.id)
  const acked = (JSON.parse(dec.decode(bytes)) as { acked?: string[] }).acked ?? []
  await ackOps(acked)
  return acked.length
}

/** Pull peers' bundles for audiences we hold a key for; apply them provisionally.
 *  A bundle that fails to open (tamper / wrong key) is skipped, not applied. */
export async function pullPeers(ctx: ExchangeCtx): Promise<number> {
  const children = await ctx.drive.list(ctx.exchangeId)
  const mine = opsFolder(ctx.deviceId)
  let applied = 0
  for (const child of children) {
    if (!child.name.startsWith('ops-') || child.name === mine) continue
    for (const b of await ctx.drive.list(child.id)) {
      if (!b.name.endsWith('.vop') || b.name.startsWith('.tmp-')) continue
      const audience = b.properties?.vidya_audience
      if (!audience || !ctx.keys[audience]) continue
      const keyver = Number(b.properties?.vidya_keyver ?? ctx.keyVersion)
      const sealed = await ctx.drive.download(b.id)
      let ops: Op[]
      try {
        const plain = sealOpen(b64ToBytes(ctx.keys[audience]), bundleAad(audience, keyver), sealed)
        ops = JSON.parse(dec.decode(plain)) as Op[]
      } catch {
        continue // tampered / wrong key
      }
      for (const op of ops) {
        if ((await applyRemoteOp(op)) === 'applied') applied += 1
      }
    }
  }
  return applied
}

export interface SyncResult {
  pushed: number
  acked: number
  pulled: number
}

export async function syncOnce(ctx: ExchangeCtx): Promise<SyncResult> {
  return {
    pushed: await pushOutbox(ctx),
    acked: await readAcks(ctx),
    pulled: await pullPeers(ctx),
  }
}

/** Build the exchange context from the joined identity + keys in kv, or null if this
 *  device has not joined a school yet (Step 5 populates these). */
export async function buildCtx(drive: DriveApi): Promise<ExchangeCtx | null> {
  const deviceId = await kvGet<string>('device_id')
  const schoolName = await kvGet<string>('school_name')
  const schoolId = await kvGet<string>('school_id')
  const keys = await kvGet<AudienceKeys>('audience_keys')
  if (!deviceId || !schoolName || !schoolId || !keys) return null
  const keyVersion = (await kvGet<number>('audience_keyver')) ?? 1
  const exchangeId = await resolveExchange(drive, schoolName, schoolId)
  return { drive, exchangeId, deviceId, keys, keyVersion }
}

/** Run one sync pass immediately (best effort) — e.g. right after a local write. */
export async function syncNow(drive: DriveApi): Promise<SyncResult | null> {
  const ctx = await buildCtx(drive)
  return ctx ? syncOnce(ctx) : null
}

let timer: ReturnType<typeof setTimeout> | null = null

/** Start the foreground sync loop: 20 s ± 3 s while visible, and immediately on
 *  `online` / when the app becomes visible (§8.3). No background sync (§18). */
export function startSync(drive: DriveApi): void {
  if (timer) return
  const tick = async (): Promise<void> => {
    try {
      if (typeof document === 'undefined' || document.visibilityState === 'visible') {
        await syncNow(drive)
      }
    } catch {
      /* offline / quota — the Sync screen surfaces "Needs attention" */
    }
    const jitter = 20_000 + Math.floor((Math.random() - 0.5) * 6_000)
    timer = setTimeout(() => void tick(), jitter)
  }
  if (typeof window !== 'undefined') {
    window.addEventListener('online', () => void tick())
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') void tick()
    })
  }
  void tick()
}
