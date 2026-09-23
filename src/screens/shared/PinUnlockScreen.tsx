// PIN unlock screen (prompts/P03 Step 7).
//
// Derived from the Welcome left-panel look: a paper panel (surface-welcome,
// radius 22) centered on var(--bg). Shows the wordmark + schoolName, a picker
// of the active staff on this device, a 4–6 digit PIN field, and error copy
// resolved from the thrown CmdError (remaining tries / wait-until time).
//
// No visible string is hard-coded: everything goes through t(). Colours come
// only from the app's CSS variable tokens.
import { useState } from 'react'
import * as api from '@/lib/api'
import type { SessionStaff, CmdError } from '@/lib/api'
import { refreshAppState } from '@/lib/store'
import { t } from '@/lib/i18n'
import { Icon } from '@/components/Icon'
import wordmark from '@/assets/vidya-horizontal-on-light.svg'

function capitalize(s: string): string {
  return s.length === 0 ? s : s[0].toUpperCase() + s.slice(1)
}

export default function PinUnlockScreen({
  staff,
  schoolName,
}: {
  staff: SessionStaff[]
  schoolName: string
}) {
  const [staffId, setStaffId] = useState<string>(staff[0]?.id ?? '')
  const [pin, setPin] = useState<string>('')
  const [error, setError] = useState<CmdError | null>(null)

  const canSubmit = staffId !== '' && pin.length >= 4 && pin.length <= 6

  async function submit(): Promise<void> {
    if (!canSubmit) return
    try {
      await api.unlock(staffId, pin)
      await refreshAppState()
    } catch (e) {
      setError(e as CmdError)
      setPin('')
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
          padding: '36px 56px',
          display: 'flex',
          flexDirection: 'column',
          gap: '24px',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
          <img
            src={wordmark}
            alt={t('app.name')}
            style={{ width: '190px', height: '62px', display: 'block' }}
          />
          <span style={{ fontSize: '12px', color: 'var(--muted)' }}>{schoolName}</span>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
          <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--ink)' }}>
            {t('pin.choose_user')}
          </span>
          <div
            role="radiogroup"
            aria-label={t('pin.choose_user')}
            style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}
          >
            {staff.map((s) => {
              const on = s.id === staffId
              return (
                <button
                  key={s.id}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  onClick={() => setStaffId(s.id)}
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '2px',
                    textAlign: 'left',
                    padding: '12px 14px',
                    borderRadius: '6px',
                    fontFamily: 'inherit',
                    cursor: 'pointer',
                    background: 'var(--surface)',
                    border: on ? '1.5px solid var(--accent)' : '1.5px solid var(--line)',
                    boxShadow: on ? '0 0 0 4px var(--accent-12)' : undefined,
                  }}
                >
                  <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--ink)' }}>
                    {s.name}
                  </span>
                  <span style={{ fontSize: '13px', color: 'var(--muted)' }}>
                    {capitalize(s.role)}
                  </span>
                </button>
              )
            })}
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <label htmlFor="pin" style={{ fontSize: '13px', fontWeight: 500, color: 'var(--ink)' }}>
            {t('pin.enter')}
          </label>
          <input
            id="pin"
            type="password"
            inputMode="numeric"
            autoComplete="off"
            maxLength={6}
            value={pin}
            onChange={(e) => {
              setPin(e.target.value.replace(/\D/g, '').slice(0, 6))
              if (error) setError(null)
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submit()
            }}
            style={{
              height: '48px',
              boxSizing: 'border-box',
              borderRadius: '6px',
              border: '1.5px solid var(--line-strong)',
              background: 'var(--surface)',
              padding: '0 14px',
              fontFamily: "'Geist', monospace",
              fontSize: '18px',
              letterSpacing: '0.3em',
              color: 'var(--ink)',
            }}
          />
          {error ? (
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
          {t('pin.unlock')}
          <Icon name="arrowUpRight" size={18} strokeWidth={1.75} />
        </button>
      </section>
    </div>
  )
}
