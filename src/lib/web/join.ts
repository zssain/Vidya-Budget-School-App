// Join-from-an-iPhone (Phase 19, Step 5, §18). The invitation QR/link carries the
// same payload as the Android app link, in the URL FRAGMENT (`#d=…`) so the static
// host never receives it. Because a browser can't reach the school PC's LAN server,
// the PWA joins through Drive: it writes a join request to `exchange/joins/`, the
// school PC answers with a sealed join response (drive/join.rs), and the PWA opens it
// with the key derived from the invite code (WASM `join_key`) and stores the device
// identity + audience keys the sync engine (Step 4) needs.

import type { DriveApi } from './drive/api'
import { JOINS } from './drive/api'
import { resolveExchange } from './drive/sync'
import { initWasm, joinAad, joinKey, sealOpen } from './wasm'
import { kvGet, kvPut } from './store'

/** The invitation payload (same fields the Android app link carries). */
export interface InvitePayload {
  invite_code: string
  school_name: string
  school_id: string
}

const FRAGMENT_KV = 'pending_join_fragment'
const encoder = new TextEncoder()
const decoder = new TextDecoder()

function b64urlToString(s: string): string {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/')
  const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
  return decoder.decode(bytes)
}

/** Parse the invitation payload from a URL fragment `#d=<base64url json>` (or null). */
export function parseJoinFragment(hash: string): InvitePayload | null {
  const m = /[#&]d=([^&]+)/.exec(hash)
  if (!m) return null
  try {
    const p = JSON.parse(b64urlToString(m[1])) as Partial<InvitePayload>
    if (p.invite_code && p.school_name && p.school_id) return p as InvitePayload
    return null
  } catch {
    return null
  }
}

/** Stash the payload before the "Add to Home Screen" step so the join resumes after
 *  the app is opened from the Home Screen (which starts with no fragment). */
export async function stashJoinFragment(payload: InvitePayload): Promise<void> {
  await kvPut(FRAGMENT_KV, payload)
}

/** The stashed invitation, if a join is in progress. */
export async function takeStashedJoin(): Promise<InvitePayload | null> {
  return (await kvGet<InvitePayload>(FRAGMENT_KV)) ?? null
}

function randomId(): string {
  const b = new Uint8Array(16)
  crypto.getRandomValues(b)
  return [...b].map((x) => x.toString(16).padStart(2, '0')).join('')
}

/** Write the join request to `exchange/joins/<request_id>.vjoin`; returns the id.
 *  (Plaintext JSON in the school's own sync account so the PC can authenticate the
 *  single-use invite code and derive the response seal key.) */
export async function requestJoin(drive: DriveApi, payload: InvitePayload): Promise<string> {
  const exchangeId = await resolveExchange(drive, payload.school_name, payload.school_id)
  const joinsId = await drive.ensureFolder(exchangeId, JOINS)
  const requestId = randomId()
  const req = {
    request_id: requestId,
    invite_code: payload.invite_code,
    device_name: 'iPhone',
    platform: 'web',
  }
  await drive.upload(joinsId, `${requestId}.vjoin`, encoder.encode(JSON.stringify(req)))
  return requestId
}

interface AudienceKey {
  audience: string
  key_b64: string
  version: number
}
interface JoinResp {
  device_id: string
  device_token: string
  staff: { id: string; name: string; role: string }
  receipt_series: string
  admission_series: string
  audience_keys: AudienceKey[]
  session_key: string
  school: { id: string; name: string }
  lease_expires_at: string
  server_epoch: number
}

/** Poll once for the school PC's sealed join response. Returns true once joined (the
 *  identity + audience keys are stored for the sync engine); false while waiting. */
export async function pollJoinResponse(
  drive: DriveApi,
  payload: InvitePayload,
  requestId: string,
): Promise<boolean> {
  await initWasm()
  const exchangeId = await resolveExchange(drive, payload.school_name, payload.school_id)
  const joinsId = await drive.ensureFolder(exchangeId, JOINS)
  const file = await drive.findChild(joinsId, `${requestId}.resp.vjoin`)
  if (!file) return false // "Waiting for the school computer (usually under a minute)"
  const sealed = await drive.download(file.id)
  const plain = sealOpen(joinKey(payload.invite_code), joinAad(requestId), sealed)
  const resp = JSON.parse(decoder.decode(plain)) as JoinResp

  // Persist the joined identity + the audience keys the Drive sync engine reads.
  await kvPut('device_id', resp.device_id)
  await kvPut('device_token', resp.device_token)
  await kvPut('staff', resp.staff)
  await kvPut('school_id', resp.school.id)
  await kvPut('school_name', resp.school.name)
  await kvPut('receipt_series', resp.receipt_series)
  await kvPut('admission_series', resp.admission_series)
  await kvPut('session_key', resp.session_key)
  await kvPut('lease_expires_at', resp.lease_expires_at)
  await kvPut('server_epoch', resp.server_epoch)
  const keys: Record<string, string> = {}
  let keyver = 1
  for (const k of resp.audience_keys) {
    keys[k.audience] = k.key_b64
    keyver = k.version
  }
  await kvPut('audience_keys', keys)
  await kvPut('audience_keyver', keyver)
  await kvPut(FRAGMENT_KV, null) // join complete — clear the stash
  return true
}

/** Whether this device has completed a join (has an identity + audience keys). */
export async function hasJoined(): Promise<boolean> {
  return (await kvGet<string>('device_id')) != null && (await kvGet<unknown>('audience_keys')) != null
}
