// iPhone PWA onboarding (Phase 20, C2). Wires the Step-5 join primitives end-to-end:
//   invite (URL fragment) → Connect Google Drive → write join request → poll for the
//   school PC's sealed response → set a local PIN → home.
// The school PC answers the request in its live Drive loop (src-tauri sync/drive/live
// server_drive_tick). Every string via t(); colours via CSS variable tokens. Verified
// on a real iPhone against a connected school PC (the owner's end-to-end check) —
// the join/seal logic itself is covered by the Rust + web unit suites.
import { useEffect, useRef, useState } from 'react'
import * as api from '@/lib/api'
import { refreshAppState } from '@/lib/store'
import { t } from '@/lib/i18n'
import { Icon } from '@/components/Icon'
import wordmark from '@/assets/vidya-horizontal-on-light.svg'
import {
  type InvitePayload,
  hasJoined,
  parseJoinFragment,
  pollJoinResponse,
  requestJoin,
  stashJoinFragment,
  takeStashedJoin,
} from '@/lib/web/join'
import { connectDrive } from '@/lib/web/drive/auth'
import { GoogleDrive } from '@/lib/web/drive/google'

type Phase = 'invite' | 'connecting' | 'waiting' | 'setpin'

// A thrown Error.message → a user message key (only DRIVE_NOT_CONFIGURED is specific;
// everything else is the generic "couldn't join" so we never show a raw code).
function errKey(e: unknown): string {
  const m = e instanceof Error ? e.message : String(e)
  return m === 'DRIVE_NOT_CONFIGURED' ? 'error.DRIVE_NOT_CONFIGURED' : 'error.JOIN_FAILED'
}

const POLL_MS = 5000

export default function JoinScreen() {
  const [payload, setPayload] = useState<InvitePayload | null>(null)
  const [phase, setPhase] = useState<Phase>('invite')
  const [errorKey, setErrorKey] = useState<string | null>(null)
  const driveRef = useRef<GoogleDrive | null>(null)
  const reqIdRef = useRef<string | null>(null)

  // On mount: resume onboarding. A device that already joined (reload before the PIN
  // was set) jumps straight to the PIN step; otherwise read the invite from the URL
  // fragment (and stash it so it survives Add-to-Home-Screen) or a prior stash.
  useEffect(() => {
    let alive = true
    void (async () => {
      if (await hasJoined()) {
        if (alive) setPhase('setpin')
        return
      }
      const fromHash = parseJoinFragment(typeof location === 'undefined' ? '' : location.hash)
      if (fromHash) await stashJoinFragment(fromHash)
      const p = fromHash ?? (await takeStashedJoin())
      if (alive && p) setPayload(p)
    })()
    return () => {
      alive = false
    }
  }, [])

  async function connectAndJoin(): Promise<void> {
    if (!payload) return
    setErrorKey(null)
    setPhase('connecting')
    try {
      await connectDrive() // GIS consent (a tap) — no secret, drive.file scope only
      const drive = new GoogleDrive()
      driveRef.current = drive
      reqIdRef.current = await requestJoin(drive, payload)
      setPhase('waiting')
    } catch (e) {
      setErrorKey(errKey(e))
      setPhase('invite')
    }
  }

  // Poll for the school PC's sealed response while waiting.
  useEffect(() => {
    if (phase !== 'waiting' || !payload || !driveRef.current || !reqIdRef.current) return
    let alive = true
    let handle: ReturnType<typeof setTimeout> | null = null
    const poll = async (): Promise<void> => {
      try {
        const done = await pollJoinResponse(driveRef.current!, payload, reqIdRef.current!)
        if (!alive) return
        if (done) setPhase('setpin')
        else handle = setTimeout(() => void poll(), POLL_MS)
      } catch (e) {
        if (alive) {
          setErrorKey(errKey(e))
          setPhase('invite')
        }
      }
    }
    void poll()
    return () => {
      alive = false
      if (handle) clearTimeout(handle)
    }
  }, [phase, payload])

  async function finishPin(pin: string): Promise<void> {
    await api.create_pin(pin) // web create_pin → hashes + stores the PIN, leaves unlocked
    await refreshAppState() // → unlocked → the teacher home; App starts the sync loop
  }

  if (phase === 'setpin') return <SetPin onDone={finishPin} />

  const busy = phase === 'connecting' || phase === 'waiting'
  const statusKey = phase === 'connecting' ? 'join.web.connecting' : phase === 'waiting' ? 'join.web.waiting' : null

  return (
    <Panel>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, textAlign: 'center' }}>
        <span style={{ fontSize: 17, fontWeight: 600, color: 'var(--ink)' }}>{t('join.web.title')}</span>
        {payload ? (
          <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('join.web.invited', { school: payload.school_name })}</span>
        ) : (
          <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('join.web.no_invite')}</span>
        )}
      </div>

      {statusKey ? (
        <p role="status" style={{ fontSize: 13, color: 'var(--muted)', textAlign: 'center', lineHeight: 1.5, margin: 0 }}>
          {t(statusKey)}
        </p>
      ) : null}

      {errorKey ? (
        <span role="alert" style={{ fontSize: 13, color: 'var(--danger)', textAlign: 'center' }}>
          {t(errorKey)}
        </span>
      ) : null}

      {payload ? (
        <PrimaryButton disabled={busy} onClick={() => void connectAndJoin()} label={t(errorKey ? 'join.web.retry' : 'join.web.connect')} />
      ) : null}
    </Panel>
  )
}

// PIN create: enter + confirm (4–6 digits), then Finish.
function SetPin({ onDone }: { onDone: (pin: string) => Promise<void> }) {
  const [pin, setPin] = useState('')
  const [confirm, setConfirm] = useState('')
  const [mismatch, setMismatch] = useState(false)
  const [busy, setBusy] = useState(false)
  const valid = pin.length >= 4 && pin.length <= 6

  async function submit(): Promise<void> {
    if (!valid || busy) return
    if (pin !== confirm) {
      setMismatch(true)
      return
    }
    setBusy(true)
    try {
      await onDone(pin)
    } finally {
      setBusy(false)
    }
  }

  const onlyDigits = (v: string): string => v.replace(/\D/g, '').slice(0, 6)

  return (
    <Panel>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, textAlign: 'center' }}>
        <span style={{ fontSize: 17, fontWeight: 600, color: 'var(--ink)' }}>{t('join.web.setpin_title')}</span>
        <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('join.web.setpin_hint')}</span>
      </div>

      <PinField id="newpin" label={t('join.web.pin')} value={pin} onChange={(v) => { setPin(onlyDigits(v)); setMismatch(false) }} />
      <PinField
        id="confirmpin"
        label={t('join.web.pin_confirm')}
        value={confirm}
        onChange={(v) => { setConfirm(onlyDigits(v)); setMismatch(false) }}
        onEnter={() => void submit()}
      />

      {mismatch ? (
        <span role="alert" style={{ fontSize: 13, color: 'var(--danger)' }}>{t('join.web.pin_mismatch')}</span>
      ) : null}

      <PrimaryButton disabled={!valid || busy} onClick={() => void submit()} label={t('join.web.finish')} />
    </Panel>
  )
}

// ---- small shared presentational bits (match PinUnlockScreen's look) ----

function Panel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 20,
        boxSizing: 'border-box',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: 14,
      }}
    >
      <section
        style={{
          width: 440,
          maxWidth: '100%',
          boxSizing: 'border-box',
          background: 'var(--surface-welcome)',
          borderRadius: 22,
          boxShadow: '0 1px 2px rgba(11,26,51,0.06)',
          padding: '36px 56px',
          display: 'flex',
          flexDirection: 'column',
          gap: 24,
        }}
      >
        <img src={wordmark} alt={t('app.name')} style={{ width: 190, height: 62, display: 'block', margin: '0 auto' }} />
        {children}
      </section>
    </div>
  )
}

function PinField({
  id,
  label,
  value,
  onChange,
  onEnter,
}: {
  id: string
  label: string
  value: string
  onChange: (v: string) => void
  onEnter?: () => void
}) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      <label htmlFor={id} style={{ fontSize: 13, fontWeight: 500, color: 'var(--ink)' }}>{label}</label>
      <input
        id={id}
        type="password"
        inputMode="numeric"
        autoComplete="off"
        maxLength={6}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && onEnter) onEnter()
        }}
        style={{
          height: 48,
          boxSizing: 'border-box',
          borderRadius: 6,
          border: '1.5px solid var(--line-strong)',
          background: 'var(--surface)',
          padding: '0 14px',
          fontFamily: "'Geist', monospace",
          fontSize: 18,
          letterSpacing: '0.3em',
          color: 'var(--ink)',
        }}
      />
    </div>
  )
}

function PrimaryButton({ disabled, onClick, label }: { disabled: boolean; onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      style={{
        height: 50,
        borderRadius: 6,
        border: 0,
        background: 'var(--accent)',
        color: 'var(--surface)',
        fontSize: 15,
        fontWeight: 500,
        fontFamily: 'inherit',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 18px',
        cursor: disabled ? 'default' : 'pointer',
        opacity: disabled ? 0.55 : 1,
      }}
    >
      {label}
      <Icon name="arrowUpRight" size={18} strokeWidth={1.75} />
    </button>
  )
}
