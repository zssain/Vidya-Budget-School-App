// Device PIN setup for a joined phone / second PC (state `pin_setup`).
//
// Joining enrols the device; it never signs a person in. Before any school data is
// shown, the staff member creates this device's own 4–6 digit PIN (entered twice).
// Same paper-panel look as PinUnlockScreen (tokens only, every string through t()).
import { useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, SessionStaff } from '@/lib/api'
import { refreshAppState } from '@/lib/store'
import { t } from '@/lib/i18n'
import { Icon } from '@/components/Icon'
import wordmark from '@/assets/vidya-horizontal-on-light.svg'

const fieldStyle = {
  height: '48px',
  boxSizing: 'border-box' as const,
  borderRadius: '6px',
  border: '1.5px solid var(--line-strong)',
  background: 'var(--surface)',
  padding: '0 14px',
  fontFamily: "'Geist', monospace",
  fontSize: '18px',
  letterSpacing: '0.3em',
  color: 'var(--ink)',
}

const digits = (v: string) => v.replace(/\D/g, '').slice(0, 6)

export default function DevicePinSetupScreen({ staff }: { staff: SessionStaff }) {
  const [pin, setPin] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState<CmdError | null>(null)
  const [mismatch, setMismatch] = useState(false)
  const [busy, setBusy] = useState(false)

  const canSubmit = !busy && pin.length >= 4 && confirm.length >= 4

  async function submit(): Promise<void> {
    if (!canSubmit) return
    if (pin !== confirm) {
      setMismatch(true)
      setConfirm('')
      return
    }
    setBusy(true)
    try {
      await api.create_device_pin(pin)
      await refreshAppState()
    } catch (e) {
      setError(e as CmdError)
      setPin('')
      setConfirm('')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '20px',
        boxSizing: 'border-box',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: '14px',
      }}
    >
      <section
        style={{
          width: '440px',
          maxWidth: '100%',
          boxSizing: 'border-box',
          background: 'var(--surface-welcome)',
          borderRadius: '22px',
          boxShadow: '0 1px 2px rgba(11,26,51,0.06)',
          padding: '36px 40px',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
          <img src={wordmark} alt={t('app.name')} style={{ width: '190px', height: '62px', display: 'block' }} />
          <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--ink)' }}>{staff.name}</span>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
          <span style={{ fontSize: '16px', fontWeight: 600, color: 'var(--ink)' }}>{t('pin.setup_title')}</span>
          <span style={{ fontSize: '13px', color: 'var(--muted)', lineHeight: 1.5 }}>{t('pin.setup_hint')}</span>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <label htmlFor="new-pin" style={{ fontSize: '13px', fontWeight: 500, color: 'var(--ink)' }}>
            {t('pin.new')}
          </label>
          <input
            id="new-pin"
            type="password"
            inputMode="numeric"
            autoComplete="off"
            maxLength={6}
            value={pin}
            onChange={(e) => {
              setPin(digits(e.target.value))
              setError(null)
              setMismatch(false)
            }}
            style={fieldStyle}
          />
          <label htmlFor="confirm-pin" style={{ fontSize: '13px', fontWeight: 500, color: 'var(--ink)' }}>
            {t('pin.confirm')}
          </label>
          <input
            id="confirm-pin"
            type="password"
            inputMode="numeric"
            autoComplete="off"
            maxLength={6}
            value={confirm}
            onChange={(e) => {
              setConfirm(digits(e.target.value))
              setError(null)
              setMismatch(false)
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submit()
            }}
            style={fieldStyle}
          />
          {mismatch ? (
            <span role="alert" style={{ fontSize: '13px', color: 'var(--danger)' }}>
              {t('pin.mismatch')}
            </span>
          ) : error ? (
            <span role="alert" style={{ fontSize: '13px', color: 'var(--danger)' }}>
              {t(error.message_key, error.vars as Record<string, string | number>)}
            </span>
          ) : null}
        </div>

        <button
          type="button"
          disabled={!canSubmit}
          onClick={() => void submit()}
          style={{
            height: '50px',
            borderRadius: '6px',
            border: 0,
            background: 'var(--accent)',
            color: 'var(--surface)',
            fontSize: '15px',
            fontWeight: 500,
            fontFamily: 'inherit',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '0 18px',
            cursor: canSubmit ? 'pointer' : 'default',
            opacity: canSubmit ? 1 : 0.55,
          }}
        >
          {t('pin.create')}
          <Icon name="arrowUpRight" size={18} strokeWidth={1.75} />
        </button>
      </section>
    </div>
  )
}
