// Collect fee (Accountant) — pixel-exact React translation of
// design/screens/FeeCollection.dc.html. Every inline style is copied verbatim
// from the mock; the only substitutions are the accent CSS vars, the app's
// keyframe names (vSheet / vFade / vPopFee / vIn8), the shared <Icon> and
// bundled logo, and i18n via t(...). Interaction logic mirrors renderVals()
// (docs/01-MOCK-SPEC.md §8). Numbers are grouped with toLocaleString('en-IN')
// exactly as the mock does, so the amount / balance / pay strings match 1:1.

import { useState, type CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { parseDigits } from '@/lib/format'
import { navigate } from '@/lib/router'
import logo from '@/assets/vidya-horizontal-on-dark.svg'
import type { CollectFeeData } from '@/dev/fixtures/collectFee'

type Mode = 'cash' | 'upi' | 'cheque'

interface CollectFeeState {
  amount: string
  mode: Mode
  done: boolean
  reference: string
  receiptNo: string | null
}

const SERIF = "'Newsreader', Georgia, serif"

/** Pill styles by variant — from renderVals()'s `pill(bg, fg)` helper. */
const pillStyle = (bg: string, fg: string): CSSProperties => ({
  justifySelf: 'start',
  display: 'inline-flex',
  alignItems: 'center',
  height: 24,
  padding: '0 10px',
  borderRadius: 4,
  fontSize: 12,
  fontWeight: 500,
  background: bg,
  color: fg,
})

const PILL: Record<CollectFeeData['rows'][number]['pill'], CSSProperties> = {
  // Part paid: #F4ECDC / #6B5220
  partpaid: pillStyle('#F4ECDC', '#6B5220'),
  // Paid: accent at 12% / accent  → var(--accent-12) / var(--accent)
  paid: pillStyle('var(--accent-12)', 'var(--accent)'),
  // Unpaid: #F6E4E2 / #8E2F2A
  unpaid: pillStyle('#F6E4E2', '#8E2F2A'),
}

export default function CollectFeeScreen({
  data,
  initial,
  onRecord,
  onSelectStudent,
  onCommsNotice,
}: {
  data: CollectFeeData
  initial?: { amount?: string; mode?: Mode; done?: boolean }
  /** Real flow: record the payment → returns the real receipt number. */
  onRecord?: (amountPaise: number, mode: Mode, reference: string) => Promise<string>
  /** Real flow: a search row was clicked (id is carried on the row when wired). */
  onSelectStudent?: (id: string) => void
  /** Real flow: Print / Share are Phase-7 — show a "coming later" notice. */
  onCommsNotice?: () => void
}) {
  const [state, setState] = useState<CollectFeeState>({
    amount: initial?.amount ?? '1000',
    mode: initial?.mode ?? 'upi',
    done: initial?.done ?? false,
    reference: '',
    receiptNo: null,
  })

  const due = data.due
  const amt = parseInt(String(state.amount).replace(/[^0-9]/g, ''), 10) || 0
  const over = amt > due
  const fmt = (v: number): string => '₹' + v.toLocaleString('en-IN')
  const cantSave = amt <= 0 || over

  const modeDefs: [Mode, string][] = [
    ['cash', t('fee.mode.cash')],
    ['upi', t('fee.mode.upi')],
    ['cheque', t('fee.mode.cheque')],
  ]
  const refs: Record<'upi' | 'cheque', [string, string]> = {
    upi: [t('fee.ref.upi.label'), t('fee.ref.upi.hint')],
    cheque: [t('fee.ref.cheque.label'), t('fee.ref.cheque.hint')],
  }

  const showRef = state.mode !== 'cash'
  const ref = state.mode === 'cheque' ? refs.cheque : refs.upi
  const refLabel = ref[0]
  const refHint = ref[1]
  const modeLabel = modeDefs.find((m) => m[0] === state.mode)![1]
  const balFmt = fmt(Math.max(due - amt, 0))
  const saveLabel = amt > 0 && !over ? t('fee.record', { amount: amt.toLocaleString('en-IN') }) : t('fee.enterValid')

  const onAmount = (v: string) => setState((s) => ({ ...s, amount: parseDigits(v, 7) }))
  const save = () => {
    if (cantSave) return
    if (onRecord) {
      // Real flow: record via the backend, then show the REAL receipt number.
      void onRecord(amt * 100, state.mode, state.reference).then((receiptNo) =>
        setState((s) => ({ ...s, done: true, receiptNo })),
      )
    } else {
      setState((s) => ({ ...s, done: true }))
    }
  }
  const reset = () => setState({ done: false, amount: '1000', mode: 'upi', reference: '', receiptNo: null })
  const close = () => navigate('/placeholder')

  // Amount chips — [label, value]; selected chip goes navy when amt === value.
  const chips: [string, number][] = [
    [t('fee.chip.fullDue'), 3100],
    [t('fee.chip.thousand'), 1000],
    [t('fee.chip.fiveHundred'), 500],
  ]

  // amountBox style — accent border + accent-12 ring normally; danger when over.
  const amountBox: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    height: 62,
    borderRadius: 6,
    background: '#FFFFFF',
    transition: 'box-shadow .18s ease, border-color .18s ease',
    border: `1.5px solid ${over ? '#C0392B' : 'var(--accent)'}`,
    boxShadow: `0 0 0 4px ${over ? 'rgba(192,57,43,0.14)' : 'var(--accent-12)'}`,
  }

  return (
    <div
      style={{
        position: 'relative',
        width: 1440,
        height: 1080,
        display: 'flex',
        background: '#F5F7F6',
        color: '#13233F',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: 14,
        overflow: 'hidden',
      }}
    >
      {/* --- Sidebar (256px navy) --- */}
      <aside
        style={{
          width: 256,
          flexShrink: 0,
          background: '#0C1B38',
          color: '#FFFFFF',
          display: 'flex',
          flexDirection: 'column',
          padding: '26px 16px 20px',
          boxSizing: 'border-box',
          gap: 28,
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4, padding: '0 10px' }}>
          <img src={logo} alt={t('app.name')} width={160} height={52} style={{ width: 160, height: 52, display: 'block' }} />
          <span style={{ fontSize: 12, color: '#9FACBF' }}>{t('fee.school')}</span>
        </div>
        <nav aria-label="Main" style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, color: '#C9D2DE', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="home" strokeWidth={1.6} />
            <span>{t('fee.nav.home')}</span>
          </a>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, background: 'rgba(255,255,255,0.07)', color: '#FFFFFF', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="fees" strokeWidth={1.6} color="#C5AB7A" />
            <span>{t('fee.nav.collect')}</span>
            <span style={{ marginLeft: 'auto', width: 5, height: 5, borderRadius: 3, background: '#C5AB7A' }} />
          </a>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, color: '#C9D2DE', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="students" strokeWidth={1.6} />
            <span>{t('fee.nav.students')}</span>
          </a>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, color: '#C9D2DE', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="receipts" strokeWidth={1.6} />
            <span>{t('fee.nav.receipts')}</span>
          </a>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, color: '#C9D2DE', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="daybook" strokeWidth={1.6} />
            <span>{t('fee.nav.daybook')}</span>
          </a>
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); close() }}
            style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, color: '#C9D2DE', textDecoration: 'none', fontWeight: 500 }}
          >
            <Icon name="requests" strokeWidth={1.6} />
            <span>{t('fee.nav.requests')}</span>
          </a>
        </nav>
        <div style={{ marginTop: 'auto', display: 'flex', alignItems: 'center', gap: 10, padding: '0 6px' }}>
          <div style={{ width: 34, height: 34, borderRadius: 17, background: '#1C3358', color: '#E6D3A8', display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 600, fontSize: 13 }}>SP</div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            <span style={{ fontWeight: 500, fontSize: 13 }}>{t('fee.user.name')}</span>
            <span style={{ fontSize: 12, color: '#9FACBF' }}>{t('fee.user.role')}</span>
          </div>
        </div>
      </aside>

      {/* --- Main --- */}
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <main style={{ padding: '48px 48px', display: 'flex', flexDirection: 'column', gap: 28 }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 11, fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>
              <span style={{ width: 5, height: 5, borderRadius: 3, background: 'var(--accent)' }} />
              {t('fee.eyebrow')}
            </span>
            <div style={{ display: 'flex', flexDirection: 'column' }}>
              <h1 className="v-serif" style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 46, lineHeight: 1.05, letterSpacing: '-0.025em' }}>{t('fee.h1')}</h1>
              <span className="v-serif" style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 46, lineHeight: 1.1, letterSpacing: '-0.025em', color: 'var(--accent)' }}>{t('fee.h1sub')}</span>
            </div>
          </div>
          <div style={{ position: 'relative', width: 560 }}>
            <span style={{ position: 'absolute', left: 14, top: 14, display: 'inline-flex' }}>
              <Icon name="search" strokeWidth={1.75} color="#56657A" />
            </span>
            <input
              type="search"
              aria-label={t('fee.search.aria')}
              value="kavya"
              readOnly
              style={{ width: '100%', height: 46, boxSizing: 'border-box', borderRadius: 6, border: '1px solid #C9D3D2', background: '#FDFDFB', padding: '0 16px 0 42px', fontFamily: 'inherit', fontSize: 15, color: '#13233F' }}
            />
          </div>
          <div style={{ background: '#FDFDFB', border: '1px solid #D5DDE0', borderRadius: 16, overflow: 'hidden' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '2.2fr 1fr 1fr 1fr', padding: '12px 24px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: '#56657A', borderBottom: '1px solid #E8EDEC' }}>
              <span>{t('fee.table.student')}</span>
              <span>{t('fee.table.class')}</span>
              <span>{t('fee.table.due')}</span>
              <span>{t('fee.table.status')}</span>
            </div>
            {data.rows.map((s) => (
              <div key={s.adm} onClick={() => { if (s.id && onSelectStudent) onSelectStudent(s.id) }} style={{ display: 'grid', gridTemplateColumns: '2.2fr 1fr 1fr 1fr', alignItems: 'center', padding: '14px 24px', borderBottom: '1px solid #E8EDEC' }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <span style={{ fontWeight: 500 }}>{s.name}</span>
                  <span style={{ fontSize: 12, color: '#56657A' }}>{t('fee.table.adm', { adm: s.adm })}</span>
                </div>
                <span>{s.cls}</span>
                <span style={{ fontVariantNumeric: 'tabular-nums' }}>{s.due}</span>
                <span style={PILL[s.pill]}>{s.status}</span>
              </div>
            ))}
          </div>
        </main>
      </div>

      {/* --- Overlay --- */}
      <div style={{ position: 'absolute', inset: 0, background: 'rgba(11,26,51,0.45)', animation: 'vFade 240ms ease both' }} />

      {/* --- Sheet (520px) --- */}
      <section
        aria-label={t('fee.sheet.aria')}
        style={{
          position: 'absolute',
          top: 16,
          right: 16,
          bottom: 16,
          width: 520,
          background: '#FDFDFB',
          borderRadius: 20,
          boxShadow: '0 30px 60px rgba(11,26,51,0.3)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          animation: 'vSheet 380ms cubic-bezier(.2,.8,.2,1) both',
        }}
      >
        {!state.done ? (
          <>
            {/* Sheet header */}
            <div style={{ padding: '24px 28px 18px', display: 'flex', flexDirection: 'column', gap: 14, borderBottom: '1px solid #E8EDEC' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 11, fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>
                  <span style={{ width: 5, height: 5, borderRadius: 3, background: 'var(--accent)' }} />
                  {t('fee.sheet.recordPayment')}
                </span>
                <button
                  type="button"
                  aria-label={t('fee.close')}
                  onClick={close}
                  style={{ width: 36, height: 36, borderRadius: 18, border: '1px solid #D5DDE0', background: '#FFFFFF', color: '#56657A', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                  <Icon name="close" size={16} strokeWidth={1.75} />
                </button>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                <div style={{ width: 48, height: 48, borderRadius: 24, background: '#0C1B38', color: '#E6D3A8', display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: SERIF, fontSize: 20 }}>{data.student.initials}</div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <span className="v-serif" style={{ fontFamily: SERIF, fontSize: 28, lineHeight: 1.1, letterSpacing: '-0.015em' }}>{data.student.name}</span>
                  <span style={{ fontSize: 13, color: '#56657A' }}>{data.student.meta}</span>
                </div>
              </div>
            </div>

            {/* Sheet body */}
            <div style={{ flexGrow: 1, overflow: 'auto', padding: '20px 28px', display: 'flex', flexDirection: 'column', gap: 20 }}>
              {/* Statement */}
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                {data.lines.map((line) => (
                  <div key={line.label} style={{ display: 'grid', gridTemplateColumns: '1fr 80px 96px', padding: '8px 0', fontSize: 13, borderBottom: '1px solid #E8EDEC' }}>
                    <span>{line.label}</span>
                    <span style={{ textAlign: 'right', color: '#56657A', fontVariantNumeric: 'tabular-nums' }}>{line.amount}</span>
                    {line.paid ? (
                      <span style={{ textAlign: 'right', color: 'var(--accent)', fontWeight: 500 }}>{line.right}</span>
                    ) : (
                      <span style={{ textAlign: 'right', fontWeight: 500, fontVariantNumeric: 'tabular-nums' }}>{line.right}</span>
                    )}
                  </div>
                ))}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', padding: '12px 0 0' }}>
                  <span style={{ fontWeight: 500 }}>{t('fee.totalDueNow')}</span>
                  <span className="v-serif" style={{ fontFamily: SERIF, fontSize: 24, fontVariantNumeric: 'lining-nums tabular-nums' }}>{data.totalDue}</span>
                </div>
              </div>

              {/* Amount received */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                <label htmlFor="amt" style={{ fontSize: 13, fontWeight: 500 }}>{t('fee.amountReceived')}</label>
                <div style={amountBox}>
                  <span className="v-serif" style={{ fontFamily: SERIF, fontSize: 28, color: '#56657A', paddingLeft: 16 }}>₹</span>
                  <input
                    id="amt"
                    inputMode="numeric"
                    value={state.amount}
                    onChange={(e) => onAmount(e.target.value)}
                    className="v-serif"
                    style={{ flexGrow: 1, minWidth: 0, border: 0, background: 'transparent', height: '100%', padding: '0 12px 0 6px', fontFamily: SERIF, fontSize: 30, fontVariantNumeric: 'lining-nums tabular-nums', color: '#13233F' }}
                  />
                </div>
                {over ? (
                  <span style={{ fontSize: 13, color: '#8E2F2A', animation: 'vIn8 200ms ease both' }}>{t('fee.over', { due: due.toLocaleString('en-IN') })}</span>
                ) : null}
                <div style={{ display: 'flex', gap: 8 }}>
                  {chips.map(([label, v]) => (
                    <button
                      key={label}
                      type="button"
                      onClick={() => setState((s) => ({ ...s, amount: String(v) }))}
                      style={{
                        height: 36,
                        padding: '0 14px',
                        borderRadius: 18,
                        fontSize: 13,
                        fontWeight: 500,
                        fontVariantNumeric: 'tabular-nums',
                        ...(amt === v
                          ? { border: '1px solid #0C1B38', background: '#0C1B38', color: '#FFFFFF' }
                          : { border: '1px solid #C9D3D2', background: 'transparent', color: '#13233F' }),
                      }}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Payment mode */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                <span style={{ fontSize: 13, fontWeight: 500 }}>{t('fee.mode')}</span>
                <div role="radiogroup" aria-label={t('fee.mode.aria')} style={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', border: '1px solid #C9D3D2', borderRadius: 6, overflow: 'hidden' }}>
                  {modeDefs.map(([id, label], i) => (
                    <button
                      key={id}
                      type="button"
                      role="radio"
                      aria-checked={state.mode === id}
                      onClick={() => setState((s) => ({ ...s, mode: id }))}
                      style={{
                        height: 44,
                        border: 0,
                        fontSize: 14,
                        fontWeight: 500,
                        ...(i > 0 ? { borderLeft: '1px solid #C9D3D2' } : null),
                        ...(state.mode === id ? { background: 'var(--accent)', color: '#FFFFFF' } : { background: '#FFFFFF', color: '#13233F' }),
                      }}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Reference field (hidden for Cash) */}
              {showRef ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 8, animation: 'vIn8 220ms ease both' }}>
                  <label htmlFor="ref" style={{ fontSize: 13, fontWeight: 500 }}>{refLabel}</label>
                  <input
                    id="ref"
                    value={state.reference}
                    onChange={(e) => setState((s) => ({ ...s, reference: e.target.value }))}
                    placeholder={refHint}
                    style={{ height: 46, borderRadius: 6, border: '1px solid #C9D3D2', background: '#FFFFFF', padding: '0 14px', fontFamily: 'inherit', fontSize: 15, color: '#13233F' }}
                  />
                </div>
              ) : null}
            </div>

            {/* Sheet footer */}
            <div style={{ background: '#E2EAEB', padding: '16px 28px 20px', display: 'flex', flexDirection: 'column', gap: 14 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: '#56657A' }}>{t('fee.balanceAfter')}</span>
                  <span style={{ fontSize: 12, color: '#56657A' }}>{t('fee.balanceSub', { due: due.toLocaleString('en-IN'), pay: amt.toLocaleString('en-IN') })}</span>
                </div>
                <span className="v-serif" style={{ fontFamily: SERIF, fontSize: 30, fontVariantNumeric: 'lining-nums tabular-nums' }}>{balFmt}</span>
              </div>
              <button
                type="button"
                disabled={cantSave}
                onClick={save}
                style={{
                  height: 52,
                  borderRadius: 6,
                  border: 0,
                  fontSize: 15,
                  fontWeight: 500,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '0 20px',
                  background: cantSave ? '#C9D3D2' : 'var(--accent)',
                  color: cantSave ? '#56657A' : '#FFFFFF',
                }}
              >
                <span>{saveLabel}</span>
                <Icon name="arrowUpRight" strokeWidth={1.75} />
              </button>
              <span style={{ fontSize: 12, color: '#56657A', textAlign: 'center' }}>{t('fee.receiptNote')}</span>
            </div>
          </>
        ) : (
          /* --- Success panel --- */
          <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 18, padding: 40, textAlign: 'center', background: 'radial-gradient(120% 70% at 50% 0%, #1A3560 0%, #0C1B38 60%)', color: '#FFFFFF' }}>
            <div style={{ width: 76, height: 76, borderRadius: 38, border: '1px solid rgba(197,171,122,0.5)', color: '#C5AB7A', display: 'flex', alignItems: 'center', justifyContent: 'center', animation: 'vPopFee 460ms cubic-bezier(.2,.8,.2,1) both' }}>
              <Icon name="check" size={34} strokeWidth={1.8} />
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', animation: 'vIn8 320ms 120ms ease both' }}>
              <span className="v-serif" style={{ fontFamily: SERIF, fontSize: 40, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('fee.success.title')}</span>
              <span className="v-serif" style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 40, lineHeight: 1.1, letterSpacing: '-0.02em', color: '#C5AB7A' }}>{state.receiptNo ? t('fee.success.receiptDyn', { no: state.receiptNo }) : t('fee.success.receipt')}</span>
            </div>
            <span style={{ fontSize: 14, color: '#C9D2DE', animation: 'vIn8 320ms 200ms ease both' }}>{t('fee.success.line', { pay: amt.toLocaleString('en-IN'), mode: modeLabel, bal: Math.max(due - amt, 0).toLocaleString('en-IN') })}</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: '#BFE8D6', border: '1px solid rgba(95,208,160,0.35)', borderRadius: 16, padding: '5px 12px', animation: 'vIn8 320ms 280ms ease both' }}>
              <span style={{ width: 6, height: 6, borderRadius: 3, background: '#5FD0A0' }} />
              {t('fee.confirmed')}
            </span>
            <div style={{ display: 'flex', gap: 10, marginTop: 10, animation: 'vIn8 320ms 360ms ease both' }}>
              <button type="button" onClick={onCommsNotice} style={{ height: 46, padding: '0 20px', borderRadius: 23, border: 0, background: '#C5AB7A', color: '#0C1B38', fontSize: 14, fontWeight: 600 }}>{t('fee.print')}</button>
              <button type="button" onClick={onCommsNotice} style={{ height: 46, padding: '0 20px', borderRadius: 23, border: '1px solid rgba(255,255,255,0.3)', background: 'transparent', color: '#FFFFFF', fontSize: 14, fontWeight: 500 }}>{t('fee.whatsapp')}</button>
            </div>
            <button type="button" onClick={reset} style={{ border: 0, background: 'transparent', color: '#C9D2DE', fontSize: 14, height: 40, textDecoration: 'underline', textUnderlineOffset: 4 }}>{t('fee.another')}</button>
          </div>
        )}
      </section>
    </div>
  )
}
