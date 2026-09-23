// Setup wizard (prompts/P03 Step 6). Same shell as Welcome (left paper panel +
// right navy panel; the right panel shows the step list instead of the headline).
// Six steps: School · Session & terms · Classes · You · Recovery key · Ready.
// Derived from the Welcome mock (docs §6.2) using existing tokens only.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError } from '@/lib/api'
import { refreshAppState } from '@/lib/store'
import { t } from '@/lib/i18n'
import wordmark from '@/assets/vidya-horizontal-on-light.svg'

const STEP_KEYS = [
  'setup.step.school',
  'setup.step.session',
  'setup.step.classes',
  'setup.step.you',
  'setup.step.recovery',
  'setup.step.ready',
]

const DEFAULT_CLASSES = ['Nursery', 'LKG', 'UKG', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X', 'XI', 'XII']

function errMsg(e: CmdError): string {
  return t(e.message_key, e.vars as Record<string, string | number>)
}

export default function SetupWizard({ startStep }: { startStep: number }) {
  const [step, setStep] = useState(Math.min(Math.max(startStep, 1), 6))
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  // Step 1 — school
  const [name, setName] = useState('')
  const [address, setAddress] = useState('')
  const [board, setBoard] = useState('CBSE')
  const [udise, setUdise] = useState('')
  const [phone, setPhone] = useState('')
  // Step 2 — session
  const [label, setLabel] = useState('2026–27')
  // Step 4 — you
  const [pName, setPName] = useState('')
  const [mobile, setMobile] = useState('')
  const [pin, setPin] = useState('')
  const [pin2, setPin2] = useState('')
  // Step 5 — recovery
  const [recovery, setRecovery] = useState<string | null>(null)
  const [g3, setG3] = useState('')
  const [g5, setG5] = useState('')

  // Fetch/refresh the recovery key when reaching step 5.
  useEffect(() => {
    if (step === 5 && !recovery) {
      api
        .create_recovery_key()
        .then((r) => setRecovery(r.key))
        .catch((e) => setError(errMsg(e as CmdError)))
    }
  }, [step, recovery])

  async function guard(fn: () => Promise<void>): Promise<void> {
    setBusy(true)
    setError(null)
    try {
      await fn()
    } catch (e) {
      setError(errMsg(e as CmdError))
    } finally {
      setBusy(false)
    }
  }

  const next = () => setStep((s) => Math.min(s + 1, 6))

  async function submitSchool() {
    await guard(async () => {
      await api.setup_school({ name, address, board, udise: udise || null, phone })
      next()
    })
  }
  async function submitSession() {
    await guard(async () => {
      await api.setup_session({
        label,
        starts_on: '2026-04-01',
        ends_on: '2027-03-31',
        term1_starts: '2026-04-01',
        term1_ends: '2026-09-30',
        term2_starts: '2026-10-01',
        term2_ends: '2027-03-31',
      })
      next()
    })
  }
  async function submitClasses() {
    await guard(async () => {
      const sections = DEFAULT_CLASSES.map((n) => ({
        name: n,
        section: 'A',
        display: /^(Nursery|LKG|UKG)$/.test(n) ? n : `${n}-A`,
      }))
      await api.setup_classes(sections)
      next()
    })
  }
  async function submitPrincipal() {
    await guard(async () => {
      if (pin.length < 4 || pin.length > 6 || pin !== pin2) {
        throw { code: 'VALIDATION', message_key: 'error.VALIDATION', vars: {} } as CmdError
      }
      await api.setup_principal(pName, mobile)
      await api.create_pin(pin) // stores the principal PIN (advances to step 6 on the backend)
      next()
    })
  }
  async function confirmRecovery() {
    await guard(async () => {
      await api.confirm_recovery_key(g3, g5)
      next()
    })
  }
  async function finish() {
    await guard(async () => {
      await refreshAppState() // setup complete → app routes to PIN unlock
    })
  }

  const inputStyle: React.CSSProperties = {
    height: 46,
    borderRadius: 6,
    border: '1px solid var(--line-strong)',
    padding: '0 14px',
    background: 'var(--white)',
    color: 'var(--ink)',
    fontSize: 15,
    width: '100%',
    boxSizing: 'border-box',
  }
  const field = (labelKey: string, node: React.ReactNode) => (
    <label style={{ display: 'block', marginBottom: 14 }}>
      <span style={{ display: 'block', fontSize: 12, color: 'var(--muted)', marginBottom: 6 }}>{t(labelKey)}</span>
      {node}
    </label>
  )

  const primaryBtn = (labelText: string, onClick: () => void) => (
    <button
      onClick={onClick}
      disabled={busy}
      style={{
        height: 46,
        borderRadius: 6,
        border: 'none',
        background: 'var(--accent)',
        color: '#fff',
        fontSize: 15,
        fontWeight: 500,
        padding: '0 22px',
        cursor: busy ? 'default' : 'pointer',
        opacity: busy ? 0.6 : 1,
      }}
    >
      {labelText}
    </button>
  )

  const stepBody = () => {
    switch (step) {
      case 1:
        return (
          <>
            {field('setup.school.name', <input style={inputStyle} value={name} onChange={(e) => setName(e.target.value)} />)}
            {field('setup.school.address', <input style={inputStyle} value={address} onChange={(e) => setAddress(e.target.value)} />)}
            {field('setup.school.board', (
              <select style={inputStyle} value={board} onChange={(e) => setBoard(e.target.value)}>
                <option>CBSE</option>
                <option>ICSE</option>
                <option>State Board</option>
              </select>
            ))}
            {field('setup.school.udise', <input style={inputStyle} value={udise} onChange={(e) => setUdise(e.target.value)} />)}
            {field('setup.school.phone', <input style={inputStyle} value={phone} onChange={(e) => setPhone(e.target.value)} />)}
            {primaryBtn(t('setup.continue'), submitSchool)}
          </>
        )
      case 2:
        return (
          <>
            {field('setup.session.label', <input style={inputStyle} value={label} onChange={(e) => setLabel(e.target.value)} />)}
            <p style={{ fontSize: 13, color: 'var(--muted)' }}>
              {t('setup.session.term1')}: Apr–Sep · {t('setup.session.term2')}: Oct–Mar
            </p>
            {primaryBtn(t('setup.continue'), submitSession)}
          </>
        )
      case 3:
        return (
          <>
            <p style={{ fontSize: 14, color: 'var(--ink)', marginBottom: 12 }}>{DEFAULT_CLASSES.join(' · ')}</p>
            {primaryBtn(t('setup.continue'), submitClasses)}
          </>
        )
      case 4:
        return (
          <>
            {field('setup.you.name', <input style={inputStyle} value={pName} onChange={(e) => setPName(e.target.value)} />)}
            {field('setup.you.mobile', <input style={inputStyle} inputMode="numeric" value={mobile} onChange={(e) => setMobile(e.target.value)} />)}
            {field('setup.you.pin', <input style={inputStyle} type="password" inputMode="numeric" maxLength={6} value={pin} onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))} />)}
            {field('setup.you.pin_again', <input style={inputStyle} type="password" inputMode="numeric" maxLength={6} value={pin2} onChange={(e) => setPin2(e.target.value.replace(/\D/g, ''))} />)}
            {primaryBtn(t('setup.continue'), submitPrincipal)}
          </>
        )
      case 5:
        return (
          <>
            <p style={{ fontSize: 13, color: 'var(--muted)', marginBottom: 12 }}>{t('setup.recovery.intro')}</p>
            <div
              style={{
                fontFamily: 'monospace',
                fontSize: 20,
                letterSpacing: '0.08em',
                background: 'var(--white)',
                border: '1px solid var(--line-strong)',
                borderRadius: 8,
                padding: '16px 18px',
                marginBottom: 14,
                wordSpacing: '0.3em',
              }}
            >
              {recovery ?? '…'}
            </div>
            <p style={{ fontSize: 12, color: 'var(--muted)', marginBottom: 8 }}>{t('setup.recovery.retype')}</p>
            <div style={{ display: 'flex', gap: 10, marginBottom: 14 }}>
              {field('setup.recovery.group3', <input style={inputStyle} value={g3} onChange={(e) => setG3(e.target.value.toUpperCase())} />)}
              {field('setup.recovery.group5', <input style={inputStyle} value={g5} onChange={(e) => setG5(e.target.value.toUpperCase())} />)}
            </div>
            {primaryBtn(t('setup.continue'), confirmRecovery)}
          </>
        )
      default:
        return (
          <>
            <ul style={{ listStyle: 'none', padding: 0, margin: '0 0 16px', fontSize: 15, color: 'var(--ink)' }}>
              <li>✓ {t('setup.ready.licence')}</li>
              <li>✓ {t('setup.ready.school')}</li>
              <li>✓ {t('setup.ready.recovery')}</li>
              <li style={{ color: 'var(--muted)' }}>· {t('setup.ready.staff')}</li>
              <li style={{ color: 'var(--muted)' }}>· {t('setup.ready.drive')}</li>
            </ul>
            {primaryBtn(t('setup.ready.finish'), finish)}
          </>
        )
    }
  }

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg-welcome-outer)', display: 'flex', padding: 24, boxSizing: 'border-box', gap: 24 }}>
      {/* Left paper panel — the current step's form. */}
      <section
        style={{
          width: 600,
          flexShrink: 0,
          background: 'var(--surface-welcome)',
          borderRadius: 22,
          boxShadow: '0 1px 2px rgba(11,26,51,0.06)',
          padding: '36px 56px',
          boxSizing: 'border-box',
        }}
      >
        <img src={wordmark} alt={t('app.name')} width={190} height={62} />
        <h1 style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 40, letterSpacing: '-0.025em', margin: '24px 0 8px' }}>
          {t(STEP_KEYS[step - 1])}
        </h1>
        <div style={{ marginTop: 20 }}>{stepBody()}</div>
        {error && <p style={{ color: 'var(--danger)', fontSize: 13, marginTop: 12 }}>{error}</p>}
      </section>

      {/* Right navy panel — the step list. */}
      <aside
        style={{
          flexGrow: 1,
          background: 'radial-gradient(120% 80% at 80% 10%, #1A3560 0%, #0C1B38 55%, #08152B 100%)',
          borderRadius: 22,
          padding: '48px 56px',
          color: 'var(--on-navy)',
          boxSizing: 'border-box',
        }}
      >
        <p style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--on-navy-label)', marginBottom: 24 }}>
          {t('setup.title')}
        </p>
        <ol style={{ listStyle: 'none', padding: 0, margin: 0 }}>
          {STEP_KEYS.map((k, i) => {
            const n = i + 1
            const done = n < step
            const current = n === step
            return (
              <li
                key={k}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 12,
                  padding: '12px 0',
                  color: current ? '#fff' : done ? 'var(--on-navy)' : 'var(--on-navy-muted)',
                  fontWeight: current ? 600 : 400,
                }}
              >
                <span
                  style={{
                    width: 26,
                    height: 26,
                    borderRadius: 13,
                    display: 'inline-flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontSize: 12,
                    background: current ? 'var(--gold)' : 'rgba(255,255,255,0.07)',
                    color: current ? 'var(--navy)' : 'var(--on-navy)',
                  }}
                >
                  {done ? '✓' : n}
                </span>
                {t(k)}
              </li>
            )
          })}
        </ol>
      </aside>
    </div>
  )
}
