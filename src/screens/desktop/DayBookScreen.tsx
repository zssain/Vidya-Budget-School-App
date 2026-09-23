import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { pickSavePath } from '@/lib/files'
import * as api from '@/lib/api'
import type { DayBookDto } from '@/lib/api'

// Day book (prompts/P07 §11): StatStrip (Cash/UPI/Cheque/Total/Reversals),
// provisional shown separately, chronological list, print + CSV export.

function todayIso(): string {
  return new Date().toISOString().slice(0, 10)
}

const GRID = '64px 1.1fr 1.6fr 0.8fr 0.7fr 0.7fr 1fr 1.1fr 0.9fr'

function statCell(i: number): CSSProperties {
  return { display: 'flex', flexDirection: 'column', gap: '8px', padding: '22px 24px', ...(i > 0 ? { borderLeft: '1px solid var(--line-stat)' } : {}) }
}

export default function DayBookScreen() {
  const [date, setDate] = useState(todayIso())
  const [data, setData] = useState<DayBookDto | null>(null)
  const [toast, setToast] = useState<string | null>(null)

  const load = useCallback(() => {
    api.day_book(date).then(setData).catch(() => setData(null))
  }, [date])
  useEffect(load, [load])

  const flash = (m: string) => {
    setToast(m)
    window.setTimeout(() => setToast(null), 2600)
  }

  const exportCsv = async () => {
    const p = await pickSavePath(`daybook-${date}.csv`)
    if (!p) return
    try {
      const n = await api.export_csv('daybook', p, date)
      flash(t('daybook.exported', { n }))
    } catch {
      /* ignore */
    }
  }

  const stats = data
    ? [
        { label: t('daybook.stat.cash'), value: data.cash_paise },
        { label: t('daybook.stat.upi'), value: data.upi_paise },
        { label: t('daybook.stat.cheque'), value: data.cheque_paise },
        { label: t('daybook.stat.total'), value: data.total_paise },
        { label: t('daybook.stat.reversals'), value: data.reversals_paise },
      ]
    : []

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('daybook.eyebrow')}
        title={t('daybook.title')}
        sub={t('daybook.sub')}
        actions={
          <>
            <input type="date" value={date} onChange={(e) => setDate(e.target.value)} style={{ height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px' }} />
            <button type="button" onClick={() => navigate(`/print/daybook?date=${date}&auto=1`)} style={{ height: '44px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('daybook.print')}</button>
            <button type="button" onClick={exportCsv} style={{ height: '44px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('daybook.export')}</button>
          </>
        }
      />

      {/* StatStrip */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', background: 'var(--panel)', borderRadius: '16px' }}>
        {stats.map((s, i) => (
          <div key={s.label} style={statCell(i)}>
            <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '32px', lineHeight: 1, letterSpacing: '-0.02em', color: 'var(--ink)', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(s.value)}</span>
            <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{s.label}</span>
          </div>
        ))}
      </div>

      {data && data.provisional_paise > 0 ? (
        <div style={{ fontSize: '13px', color: 'var(--gold-text)' }}>{t('daybook.provisional', { amount: formatMoney(data.provisional_paise) })}</div>
      ) : null}

      {/* List */}
      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        <div style={{ display: 'grid', gridTemplateColumns: GRID, gap: '12px', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('daybook.col.time')}</span>
          <span>{t('daybook.col.receipt')}</span>
          <span>{t('daybook.col.student')}</span>
          <span>{t('daybook.col.class')}</span>
          <span>{t('daybook.col.mode')}</span>
          <span>{t('daybook.col.ref')}</span>
          <span style={{ textAlign: 'right' }}>{t('daybook.col.amount')}</span>
          <span>{t('daybook.col.by')}</span>
          <span style={{ textAlign: 'right' }}>·</span>
        </div>
        {!data || data.entries.length === 0 ? (
          <div style={{ padding: '40px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('daybook.none')}</div>
        ) : (
          data.entries.map((e, i) => (
            <div key={i} style={{ display: 'grid', gridTemplateColumns: GRID, gap: '12px', alignItems: 'center', padding: '12px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '13px' }}>
              <span style={{ color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{e.time}</span>
              <span style={{ fontWeight: 500 }}>{e.receipt_no}</span>
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.student_name}</span>
              <span style={{ color: 'var(--muted)' }}>{e.class_display ?? '—'}</span>
              <span style={{ color: 'var(--muted)' }}>{e.mode.toUpperCase()}</span>
              <span style={{ color: 'var(--muted)' }}>{e.reference_last4 ?? '—'}</span>
              <span style={{ textAlign: 'right', fontVariantNumeric: 'tabular-nums', textDecoration: e.reversed ? 'line-through' : 'none' }}>{formatMoney(e.amount_paise)}</span>
              <span style={{ color: 'var(--muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.collected_by ?? '—'}</span>
              <span style={{ textAlign: 'right' }}>
                {e.reversed ? <Pill variant="reversal">{t('daybook.reversed')}</Pill> : e.confirmed ? <Pill variant="paid">{t('daybook.confirmed')}</Pill> : <Pill variant="partpaid">{t('daybook.waiting')}</Pill>}
              </span>
            </div>
          ))
        )}
      </div>

      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}
