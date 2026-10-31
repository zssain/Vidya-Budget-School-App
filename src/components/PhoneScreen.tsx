// The frame shared by the teacher phone screens behind the Teacher Home tiles
// (repair T1): the navy gradient header with a back button + serif title (as on
// Apply leave / My timetable), a scrolling light body, an optional sticky footer
// and a toast. Colours are the mock's tokens only.
import type { CSSProperties, ReactNode } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'

export const PHONE_CARD: CSSProperties = {
  borderRadius: 14,
  background: 'var(--surface)',
  border: '1px solid var(--line)',
  overflow: 'hidden',
}

export const PHONE_ROW: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  padding: '14px 16px',
  borderTop: '1px solid var(--track)',
}

export const PHONE_PRIMARY: CSSProperties = {
  width: '100%',
  height: 52,
  borderRadius: 8,
  border: '1px solid var(--accent)',
  background: 'var(--accent)',
  color: 'var(--white)',
  fontSize: 15,
  fontWeight: 600,
}

export const PHONE_SECONDARY: CSSProperties = {
  width: '100%',
  height: 48,
  borderRadius: 6,
  border: '1px solid var(--line-strong)',
  background: 'var(--surface)',
  color: 'var(--ink)',
  fontSize: 15,
  fontWeight: 500,
}

export const PHONE_FIELD: CSSProperties = {
  borderRadius: 12,
  border: '1px solid var(--line)',
  background: 'var(--white)',
  color: 'var(--ink)',
  padding: '12px 14px',
  fontSize: 15,
  boxSizing: 'border-box',
  width: '100%',
}

/** A small status pill (Submitted / Draft / Pending …). */
export function PhonePill({ tone, children }: { tone: 'ok' | 'warn' | 'bad' | 'muted' | 'accent'; children: ReactNode }) {
  const colours: Record<string, { bg: string; fg: string }> = {
    ok: { bg: 'var(--accent-12)', fg: 'var(--accent)' },
    warn: { bg: 'var(--pill-partpaid-bg)', fg: 'var(--pill-partpaid-fg)' },
    bad: { bg: 'var(--pill-unpaid-bg)', fg: 'var(--pill-unpaid-fg)' },
    accent: { bg: 'var(--pill-marks-bg)', fg: 'var(--pill-marks-fg)' },
    muted: { bg: 'var(--panel)', fg: 'var(--muted)' },
  }
  const c = colours[tone]
  return (
    <span style={{ fontSize: 11, fontWeight: 600, padding: '3px 9px', borderRadius: 10, background: c.bg, color: c.fg, whiteSpace: 'nowrap', flexShrink: 0 }}>
      {children}
    </span>
  )
}

export function PhoneEmpty({ children }: { children: ReactNode }) {
  return <div style={{ padding: '28px 18px', borderRadius: 14, background: 'var(--panel)', color: 'var(--muted)', textAlign: 'center', lineHeight: 1.5 }}>{children}</div>
}

export default function PhoneScreen({
  title,
  sub,
  back = '/teacher/home',
  footer,
  toast,
  children,
}: {
  title: string
  sub?: string
  back?: string
  footer?: ReactNode
  toast?: string | null
  children: ReactNode
}) {
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', minHeight: '100dvh', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: 'calc(10px + env(safe-area-inset-top)) 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button type="button" aria-label={t('tp.back')} onClick={() => navigate(back)} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{title}</h1>
        {sub ? <span style={{ fontSize: 13, color: '#9FACBF' }}>{sub}</span> : null}
      </header>
      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>{children}</div>
      {footer ? (
        <div style={{ flexShrink: 0, background: '#FDFDFB', borderTop: '1px solid #D5DDE0', padding: '12px 16px calc(16px + env(safe-area-inset-bottom))', display: 'flex', flexDirection: 'column', gap: 8 }}>{footer}</div>
      ) : null}
      {toast ? (
        <div role="status" style={{ position: 'absolute', left: 16, right: 16, bottom: 90, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 14px', fontSize: 14, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Icon name="check" size={16} strokeWidth={2.2} /> {toast}
        </div>
      ) : null}
    </div>
  )
}
