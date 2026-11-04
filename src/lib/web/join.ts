// Join-from-an-iPhone (Phase 19, Step 5, §18) — join protocol v2. The web invite
// link carries `{v:2, invite_id, secret, school_name, school_id}` in the URL FRAGMENT
// (`#d=…`), so the static host never receives it. A browser can't reach the school
// PC's LAN server, so the PWA joins through Drive: it writes a request holding only
// the invite id, a random request id and an HMAC PROOF of the 256-bit secret (WASM
// `join_v2_proof`) — never the secret or a code. The school PC verifies the proof and
// seals the response under a key derived from the secret + request id, which only
// this device can derive. A Drive reader can neither forge a join nor open one.

import type { DriveApi } from './drive/api'
import { JOINS } from './drive/api'
import { resolveExchange } from './drive/sync'
import { initWasm, joinV2Aad, joinV2Proof, joinV2ResponseKey, sealOpen } from './wasm'
import { kvGet, kvPut, secretGet, secretPut } from './store'

/** The web invitation payload (join v2). `secret` is base64url(32 bytes). */
export interface InvitePayload {
  v: 2
  invite_id: string
  secret: string
  school_name: string
  school_id: string
}

const FRAGMENT_KV = 'pending_join_fragment'
const encoder = new TextEncoder()
const decoder = new TextDecoder()

function b64urlToBytes(s: string): Uint8Array {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/')
  return Uint8Array.from(atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4)), (c) => c.charCodeAt(0))
}

function bytesToB64url(b: Uint8Array): string {
  return btoa(String.fromCharCode(...b)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function b64urlToString(s: string): string {
  return decoder.decode(b64urlToBytes(s))
}

/** Parse the invitation payload from a URL fragment `#d=<base64url json>` (or null). */
export function parseJoinFragment(hash: string): InvitePayload | null {
  const m = /[#&]d=([^&]+)/.exec(hash)
  if (!m) return null
  try {
    const p = JSON.parse(b64urlToString(m[1])) as Partial<InvitePayload>
    // Only v2 invites: an older link (plaintext short code) is refused.
    if (p.v !== 2 || !p.invite_id || !p.secret || !p.school_name || !p.school_id) return null
    if (b64urlToBytes(p.secret).length !== 32) return null
    return p as InvitePayload
  } catch {
    return null
  }
}

/** Stash the payload before the "Add to Home Screen" step so the join resumes after
 *  the app is opened from the Home Screen (which starts with no fragment). */
export async function stashJoinFragment(payload: InvitePayload): Promise<void> {
  await secretPut(FRAGMENT_KV, payload) // carries the invite secret → encrypted
}

/** The stashed invitation, if a join is in progress. */
export async function takeStashedJoin(): Promise<InvitePayload | null> {
  try {
    return (await secretGet<InvitePayload>(FRAGMENT_KV)) ?? null
  } catch {
    return null // PIN-wrapped (K2) and the app is locked — no resumable join now
  }
}

function randomId(): string {
  const b = new Uint8Array(16)
  crypto.getRandomValues(b)
  return [...b].map((x) => x.toString(16).padStart(2, '0')).join('')
}

const DEVICE_NAME = 'iPhone'
const PLATFORM = 'web'

/** The join request body (pure → unit-tested): non-secret fields + the HMAC proof.
 *  Never contains the invite secret. */
export function buildJoinRequest(payload: InvitePayload, requestId: string): Record<string, unknown> {
  const proof = joinV2Proof(b64urlToBytes(payload.secret), payload.invite_id, requestId, DEVICE_NAME, PLATFORM)
  return {
    v: 2,
    invite_id: payload.invite_id,
    request_id: requestId,
    device_name: DEVICE_NAME,
    platform: PLATFORM,
    proof: bytesToB64url(proof),
  }
}

/** Write the join request to `exchange/joins/<request_id>.vjoin`; returns the id. */
export async function requestJoin(drive: DriveApi, payload: InvitePayload): Promise<string> {
  await initWasm()
  const exchangeId = await resolveExchange(drive, payload.school_name, payload.school_id)
  const joinsId = await drive.ensureFolder(exchangeId, JOINS)
  const requestId = randomId()
  const req = buildJoinRequest(payload, requestId)
  await drive.upload(joinsId, `${requestId}.vjoin`, encoder.encode(JSON.stringify(req)))
  return requestId
}

interface JoinResp {
  device_id: string
  device_token: string
  staff: { id: string; name: string; role: string }
  receipt_series: string
  admission_series: string
  session_key: string
  school: { id: string; name: string }
  lease_expires_at: string
  server_epoch: number
}

/** Poll once for the school PC's sealed join response. Returns true once joined (the
 *  identity + session key are stored for the sync engine); false while waiting. */
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
  const plain = sealOpen(
    joinV2ResponseKey(b64urlToBytes(payload.secret), requestId),
    joinV2Aad(payload.invite_id, requestId, payload.school_id),
    sealed,
  )
  const resp = JSON.parse(decoder.decode(plain)) as JoinResp

  // Persist the joined identity + the session key the Drive sync engine reads (no
  // audience keys: every school-PC -> device payload is sealed per device).
  await kvPut('device_id', resp.device_id)
  await secretPut('device_token', resp.device_token)
  await kvPut('staff', resp.staff)
  await kvPut('school_id', resp.school.id)
  await kvPut('school_name', resp.school.name)
  await kvPut('receipt_series', resp.receipt_series)
  await kvPut('admission_series', resp.admission_series)
  await secretPut('session_key', resp.session_key)
  await kvPut('lease_expires_at', resp.lease_expires_at)
  await kvPut('server_epoch', resp.server_epoch)
  await secretPut(FRAGMENT_KV, null) // join complete — clear the stash
  return true
}

/** Whether this device has completed a join (has an identity + session key). Checks
 *  that the session key is STORED without opening it: once a PIN exists it is
 *  PIN-wrapped (K2), and `app_state` asks this while locked — opening it there would
 *  reject LOCKED and strand a joined phone on "Please sign in" instead of the PIN gate. */
export async function hasJoined(): Promise<boolean> {
  return (await kvGet<string>('device_id')) != null && (await kvGet<unknown>('session_key')) != null
}
