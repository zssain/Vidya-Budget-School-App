import { useEffect, useState } from 'react'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { DayBookDto, SchoolDto } from '@/lib/api'

// Printable day book (A4). Hidden print route /print/daybook?date=.
export default function DayBookDoc({ date, auto = false }: { date: string; auto?: boolean }) {
  const [data, setData] = useState<DayBookDto | null>(null)
  const [school, setSchool] = useState<SchoolDto | null>(null)

  useEffect(() => {
    api.day_book(date).then(setData).catch(() => setData(null))
    api.get_school().then(setSchool).catch(() => setSchool(null))
  }, [date])

  useEffect(() => {
    if (auto && data) {
      const h = window.setTimeout(() => void printCurrentWindow(), 300)
      return () => window.clearTimeout(h)
    }
  }, [auto, data])

  if (!data) return <div style={{ padding: 40, color: 'var(--muted)' }}>…</div>

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', padding: '24px', fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif", color: 'var(--ink)' }}>
      <style>{'@page { size: A4; margin: 12mm; } @media print { .no-print { display: none !important; } }'}</style>
      <div className="no-print" style={{ display: 'flex', gap: '10px', marginBottom: '16px' }}>
        <button type="button" onClick={() => window.history.back()} style={{ height: '38px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', cursor: 'pointer' }}>{t('students.back')}</button>
        <button type="button" onClick={() => void printCurrentWindow()} style={{ height: '38px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }}>{t('daybook.print')}</button>
      </div>

      <div style={{ maxWidth: '900px', margin: '0 auto', background: 'var(--white)', border: '1px solid var(--line)', borderRadius: '6px', padding: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', borderBottom: '1px solid var(--track)', paddingBottom: '10px' }}>
          <div style={{ fontWeight: 600, fontSize: '16px' }}>{school?.name ?? ''}</div>
          <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '20px' }}>{t('daybook.docTitle')} · {data.date}</div>
        </div>
        <div style={{ display: 'flex', gap: '24px', margin: '12px 0', fontSize: '13px', fontVariantNumeric: 'tabular-nums' }}>
          <span>{t('daybook.stat.cash')}: {formatMoney(data.cash_paise)}</span>
          <span>{t('daybook.stat.upi')}: {formatMoney(data.upi_paise)}</span>
          <span>{t('daybook.stat.cheque')}: {formatMoney(data.cheque_paise)}</span>
          <span style={{ fontWeight: 600 }}>{t('daybook.stat.total')}: {formatMoney(data.total_paise)}</span>
          <span>{t('daybook.stat.reversals')}: {formatMoney(data.reversals_paise)}</span>
        </div>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
          <thead>
            <tr style={{ textAlign: 'left', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
              <th style={{ padding: '6px 4px' }}>{t('daybook.col.time')}</th>
              <th style={{ padding: '6px 4px' }}>{t('daybook.col.receipt')}</th>
              <th style={{ padding: '6px 4px' }}>{t('daybook.col.student')}</th>
              <th style={{ padding: '6px 4px' }}>{t('daybook.col.mode')}</th>
              <th style={{ padding: '6px 4px', textAlign: 'right' }}>{t('daybook.col.amount')}</th>
              <th style={{ padding: '6px 4px' }}>{t('daybook.col.by')}</th>
            </tr>
          </thead>
          <tbody>
            {data.entries.map((e, i) => (
              <tr key={i} style={{ borderBottom: '1px solid var(--track)' }}>
                <td style={{ padding: '6px 4px' }}>{e.time}</td>
                <td style={{ padding: '6px 4px' }}>{e.receipt_no}</td>
                <td style={{ padding: '6px 4px' }}>{e.student_name}</td>
                <td style={{ padding: '6px 4px' }}>{e.mode.toUpperCase()}</td>
                <td style={{ padding: '6px 4px', textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(e.amount_paise)}{e.confirmed ? '' : ' *'}</td>
                <td style={{ padding: '6px 4px' }}>{e.collected_by ?? '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div style={{ fontSize: '10px', color: 'var(--muted)', marginTop: '12px', textAlign: 'center' }}>{t('receipt.doc.generated')}</div>
      </div>
    </div>
  )
}
