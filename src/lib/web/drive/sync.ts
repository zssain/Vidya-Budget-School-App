// The Drive sync route in the browser (Phase 19, Step 4). Pushes sealed `.vop`
// bundles to exchange/ops-<device_id>/, reads the school PC's acks, and pulls peers'
// bundles for audiences this device holds a key for — applying them PROVISIONALLY
// (§8.3). Sealing is the WASM (byte-identical to the app); the audience keys and the
// device/school identity come from the join (Step 5, stored in kv). Runtime is
// verified by the Playwright/fake-Drive harness (Step 9); this module is pure logic
// over the DriveApi interface, so the harness can drive it with FakeDrive.

import type { DriveApi } from './api'
import { ACKS, EXCHANGE, opsFolder } from './api'

/** The school PC's per-device export folder (exchange.rs DOWN). */
export const DOWN = 'down'
import { deriveDirectionKeys, deviceBundleAad, deviceDownAad, relayAad, seal, sealOpen } from '../wasm'
import { type AckResult, type CanonicalRow, kvGet, kvPut, markUploaded, type Op, opsToPush, replaceAllRecords, resolveOps, secretGet } from '../store'

const MAX_OPS_PER_BUNDLE = 500 // matches vidya-core/Rust (§11)
const enc = new TextEncoder()
const dec = new TextDecoder()

/** audience → base64 32-byte key, populated at join (§8.8). */
type AudienceKeys = Record<string, string>

interface ExchangeCtx {
  drive: DriveApi
  exchangeId: string
  deviceId: string
  /** base64 per-device session key (from join) — opens this device's acks. */
  sessionKey: string
  keys: AudienceKeys
  keyVersion: number
}

/** Resolve (creating) Vidya/<school> (<school_id>)/exchange; return its folder id. */
export async function resolveExchange(drive: DriveApi, schoolName: string, schoolId: string): Promise<string> {
  const vidya = await drive.ensureFolder(drive.root(), 'Vidya')
  const school = await drive.ensureFolder(vidya, `${schoolName} (${schoolId})`)
  return drive.ensureFolder(school, EXCHANGE)
}

/** Push pending ops as device-sealed bundles (format v2): sealed with THIS device's
 *  c2s key (from its session key), AAD bound to the device id — identical to the
 *  native `exchange::push_outbox`. No audience key is needed to upload, and the school
 *  PC can prove which device wrote each bundle. Upload to a temp name then rename. */
export async function pushOutbox(ctx: ExchangeCtx, nowMs: number = Date.now()): Promise<number> {
  // Only ops not yet uploaded (or whose upload went unacknowledged for a long time).
  const ops = (await opsToPush(nowMs)).filter((o) => o.device_id === ctx.deviceId)
  if (ops.length === 0) return 0
  const myOps = await ctx.drive.ensureFolder(ctx.exchangeId, opsFolder(ctx.deviceId))
  const { c2s } = deriveDirectionKeys(ctx.sessionKey)
  let pushed = 0
  for (let i = 0; i < ops.length; i += MAX_OPS_PER_BUNDLE) {
    const chunk: Op[] = ops.slice(i, i + MAX_OPS_PER_BUNDLE)
    const sealed = seal(c2s, deviceBundleAad(ctx.deviceId), enc.encode(JSON.stringify(chunk)))
    const finalName = `${chunk[0].hlc}-dev-v2.vop`
    const id = await ctx.drive.upload(myOps, `.tmp-${finalName}`, sealed, {
      properties: { vidya_format: 'device-v2' },
    })
    await ctx.drive.rename(id, finalName)
    await markUploaded(chunk.map((o) => o.op_id), nowMs)
    pushed += chunk.length
  }
  return pushed
}

/** The school PC's per-device ack (`exchange.rs write_ack`): sealed with this
 *  device's session-derived s2c key, AAD = relay AAD("ACK", device, device, 0). */
export interface DeviceAck {
  last_hlc: string
  results: AckResult[]
}

/** Open + parse a sealed ack exactly as the native client does (pure given WASM).
 *  Throws on a wrong key / device / tampered bytes (fail closed). */
export function openAck(sealed: Uint8Array, sessionKeyB64: string, deviceId: string): DeviceAck {
  const { s2c } = deriveDirectionKeys(sessionKeyB64)
  const plain = sealOpen(s2c, relayAad('ACK', deviceId, deviceId, 0), sealed)
  const ack = JSON.parse(dec.decode(plain)) as DeviceAck
  if (!Array.isArray(ack.results)) throw new Error('malformed ack')
  return ack
}

/** Read the school PC's ack for this device: confirmed ops leave the outbox, refused
 *  ones are kept with their reason. An unreadable ack is skipped (not fatal), so the
 *  rest of the pass (peer pull) still runs. */
export async function readAcks(ctx: ExchangeCtx): Promise<number> {
  const acksFolder = await ctx.drive.ensureFolder(ctx.exchangeId, ACKS)
  const file = await ctx.drive.findChild(acksFolder, `${ctx.deviceId}.json`)
  if (!file) return 0
  let ack: DeviceAck
  try {
    ack = openAck(await ctx.drive.download(file.id), ctx.sessionKey, ctx.deviceId)
  } catch {
    return 0
  }
  const { confirmed } = await resolveOps(ack.results)
  return confirmed
}

/** Peers' uploads are sealed to the school PC with each device's own key, so they
 *  are not readable here (no provisional peer apply — spec default). Canonical changes
 *  reach this device from the school PC. Kept as a no-op for the SyncResult shape. */
export async function pullPeers(_ctx: ExchangeCtx): Promise<number> {
  return 0
}

/** The school PC's sealed per-device export (exchange.rs DownExport). */
export interface DownExport {
  v: number
  device_id: string
  cursor: number
  server_epoch: number
  changes: CanonicalRow[]
}

/** Open + verify a down export exactly as the native read_down_export does. Throws
 *  on a wrong key / another device's export / tampered bytes (fail closed). */
export function openDown(sealed: Uint8Array, sessionKeyB64: string, deviceId: string): DownExport {
  const { s2c } = deriveDirectionKeys(sessionKeyB64)
  const body = JSON.parse(dec.decode(sealOpen(s2c, deviceDownAad(deviceId), sealed))) as DownExport
  if (body.device_id !== deviceId || !Array.isArray(body.changes)) throw new Error('down export for another device')
  return body
}

/** Apply the school PC's export if it is newer than what this device holds: the whole
 *  local data set is replaced in one transaction. Returns rows written (0 = no news). */
export async function readDown(ctx: ExchangeCtx): Promise<number> {
  const folder = await ctx.drive.ensureFolder(ctx.exchangeId, DOWN)
  const file = await ctx.drive.findChild(folder, `${ctx.deviceId}.vdown`)
  if (!file) return 0
  let body: DownExport
  try {
    body = openDown(await ctx.drive.download(file.id), ctx.sessionKey, ctx.deviceId)
  } catch {
    return 0
  }
  const held = (await kvGet<number>('down_cursor')) ?? -1
  if (body.cursor <= held) return 0
  const n = await replaceAllRecords(body.changes)
  await kvPut('down_cursor', body.cursor)
  await kvPut('server_epoch', body.server_epoch)
  return n
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
    pulled: (await pullPeers(ctx)) + (await readDown(ctx)),
  }
}

/** Build the exchange context from the joined identity + keys in kv, or null if this
 *  device has not joined a school yet (Step 5 populates these). */
export async function buildCtx(drive: DriveApi): Promise<ExchangeCtx | null> {
  const deviceId = await kvGet<string>('device_id')
  const schoolName = await kvGet<string>('school_name')
  const schoolId = await kvGet<string>('school_id')
  const keys = await secretGet<AudienceKeys>('audience_keys')
  if (!deviceId || !schoolName || !schoolId || !keys) return null
  const sessionKey = await secretGet<string>('session_key')
  if (!sessionKey) return null
  const keyVersion = (await kvGet<number>('audience_keyver')) ?? 1
  const exchangeId = await resolveExchange(drive, schoolName, schoolId)
  return { drive, exchangeId, deviceId, sessionKey, keys, keyVersion }
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
