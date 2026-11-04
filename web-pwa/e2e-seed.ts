// TEST-ONLY seeding harness for the WebKit PWA e2e suite (tests/e2e/pwa). It is
// loaded ONLY by the `--mode e2e` build (web-pwa/dist-e2e): main.tsx imports it
// behind the build-time constant `__VIDYA_E2E__`, which vite.config.ts defines as
// `false` for every other mode, so the import is dead code and this module is not
// emitted into the production bundle (web-pwa/dist). tests/e2e/pwa/pwa.spec.ts
// asserts that by scanning the production build for the `__vidyaE2E` marker.
//
// It seeds a joined, PIN-protected teacher through the app's OWN code paths — the
// same kv/secret writers the join uses, the real createPin, the real phone write
// commands (webInvoke → outbox op), and the real ack application (resolveOps) — so
// the stored bytes are exactly what the app itself writes (no hand-made ciphertext).

import { webInvoke } from '@/lib/web/backend'
import { createPin, lock } from '@/lib/web/lock'
import { kvPutMany, markUploaded, pendingOps, refusedOps, resolveOps, secretPut } from '@/lib/web/store'

export interface E2ESeed {
  deviceId: string
  /** base64 session key — the test uses it to open the bundles the app uploads. */
  sessionKey: string
  /** The homework-note op the school PC refused (FORBIDDEN). */
  refusedOpId: string
  /** The unrelated check-in op still waiting to be sent. */
  pendingOpId: string
}

function b64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
}

function localDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

async function seedJoinedTeacher(opts: { pin: string }): Promise<E2ESeed> {
  const deviceId = 'dev-e2e-iphone'
  const sessionKey = b64(crypto.getRandomValues(new Uint8Array(32)))
  // The joined identity, exactly as join.ts pollJoinResponse stores it.
  await kvPutMany([
    ['device_id', deviceId],
    ['staff', { id: 'stf-e2e-meena', name: 'Meena Iyer', role: 'teacher' }],
    ['school_id', 'sch-e2e'],
    ['school_name', 'E2E Public School'],
    ['receipt_series', 'R'],
    ['admission_series', 'A'],
    ['lease_expires_at', new Date(Date.now() + 30 * 864e5).toISOString()],
    ['server_epoch', 1],
  ])
  await secretPut('device_token', 'e2e-device-token')
  await secretPut('session_key', sessionKey)
  // Real PIN creation: Argon2id verifier + PIN-wrap of the secrets above; unlocks.
  await createPin(opts.pin)

  // Two real phone writes through the web command dispatcher.
  const note = (await webInvoke<{ id: string }>('save_homework_note', {
    input: { class_id: 'cls-e2e-7a', class_subject_id: null, kind: 'homework', text: 'Refused e2e note', attachments: [] },
  }))
  const now = new Date()
  await webInvoke('staff_check_in', {
    input: { route: 'drive', date: localDate(now), minute: now.getHours() * 60 + now.getMinutes(), device_ms: now.getTime() },
  })
  const ops = await pendingOps()
  const refused = ops.find((o) => o.table === 'homework_note' && o.record_id === note.id)
  const pending = ops.find((o) => o.table === 'staff_attendance')
  if (!refused || !pending || ops.length !== 2) throw new Error(`e2e seed: unexpected outbox ${JSON.stringify(ops.map((o) => o.table))}`)

  // The note was uploaded long ago (well past the re-upload window) and the school
  // PC's ack refused it — applied with the real ack path. A buggy push filter would
  // therefore re-send it; the correct one never does.
  await markUploaded([refused.op_id], Date.now() - 2 * 60 * 60 * 1000)
  await resolveOps([{ op_id: refused.op_id, status: 'rejected', reason_code: 'FORBIDDEN' }])

  lock() // the test reloads and unlocks through the real PIN screen
  return { deviceId, sessionKey, refusedOpId: refused.op_id, pendingOpId: pending.op_id }
}

/** The outbox as the app's store reads it (op ids only), for assertions. */
async function outbox(): Promise<{ pending: string[]; refused: { id: string; status: string | undefined; reason: string | null | undefined }[] }> {
  const [pending, refused] = await Promise.all([pendingOps(), refusedOps()])
  return {
    pending: pending.map((o) => o.op_id),
    refused: refused.map((r) => ({ id: r.op_id, status: r.status, reason: r.reason })),
  }
}

declare global {
  interface Window {
    __vidyaE2E?: { seedJoinedTeacher: typeof seedJoinedTeacher; outbox: typeof outbox }
  }
}

window.__vidyaE2E = { seedJoinedTeacher, outbox }
