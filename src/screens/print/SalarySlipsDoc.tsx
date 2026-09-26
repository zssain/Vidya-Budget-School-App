// Salary slips print doc (P15 Step 5). One A5 slip per paid staff member for the
// month, from the salary register. Auto-prints when opened with ?auto=1.

import { useEffect, useState } from 'react'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { SalaryRegisterDto, SchoolDto } from '@/lib/api'

export default function SalarySlipsDoc({ month, auto = false }: { month: string; auto?: boolean }) {
  const [data, setData] = useState<SalaryRegisterDto | null>(null)
  const [school, setSchool] = useState<SchoolDto | null>(null)

  useEffect(() => {
    api.salary_register(month).then(setData).catch(() => setData(null))
    api.get_school().then(setSchool).catch(() => setSchool(null))
  }, [month])

  useEffect(() => {
    if (auto && data) {
      const h = window.setTimeout(() => void printCurrentWindow(), 300)
      return () => window.clearTimeout(h)
    }
  }, [auto, data])

  if (!data) return <div style={{ padding: 40, color: 'var(--muted)' }}>…</div>
  const paid = data.rows.filter((r) => r.paid)

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', padding: '24px', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", color: 'var(--ink)' }}>
      <style>{'@page { size: A5; margin: 12mm; } @media print { .no-print { display: none !important; } .slip { page-break-after: always; } }'}</style>
      <div className="no-print" style={{ display: 'flex', gap: '10px', marginBottom: '16px' }}>
        <button type="button" onClick={() => window.history.back()}>{t('salary.slip.back')}</button>
        <button type="button" onClick={() => void printCurrentWindow()}>{t('salary.printSlips')}</button>
      </div>
      {paid.length === 0 ? <div style={{ color: 'var(--muted)' }}>{t('salary.slip.none')}</div> : null}
      {paid.map((r) => (
        <div key={r.staff_id} className="slip" style={{ maxWidth: '640px', margin: '0 auto 24px', background: 'var(--white)', border: '1px solid var(--line)', borderRadius: '6px', padding: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', borderBottom: '1px solid var(--track)', paddingBottom: '10px' }}>
            <div style={{ fontWeight: 600, fontSize: '16px' }}>{school?.name ?? ''}</div>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '20px' }}>{t('salary.slip.title')} · {month}</div>
          </div>
          <div style={{ marginTop: '12px', fontSize: '14px' }}>
            <div style={{ fontWeight: 600, fontSize: '15px' }}>{r.name}</div>
            <div style={{ color: 'var(--muted)', fontSize: '13px' }}>{t(`salary.role.${r.role}`)}</div>
          </div>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px', marginTop: '12px', fontVariantNumeric: 'tabular-nums' }}>
            <tbody>
              {[
                [t('salary.col.monthly'), formatMoney(r.monthly_paise)],
                [t('salary.slip.days'), `${r.days_present} / ${r.working_days}`],
                [t('salary.col.deduction'), r.deduction_paise > 0 ? `− ${formatMoney(r.deduction_paise)}` : '—'],
                [t('salary.col.advance'), r.advance_recovery_paise > 0 ? `− ${formatMoney(r.advance_recovery_paise)}` : '—'],
              ].map(([k, v], i) => (
                <tr key={i} style={{ borderBottom: '1px solid var(--track)' }}>
                  <td style={{ padding: '7px 4px', color: 'var(--muted)' }}>{k}</td>
                  <td style={{ padding: '7px 4px', textAlign: 'right' }}>{v}</td>
                </tr>
              ))}
              <tr>
                <td style={{ padding: '9px 4px', fontWeight: 600 }}>{t('salary.col.net')}</td>
                <td style={{ padding: '9px 4px', textAlign: 'right', fontWeight: 600, fontSize: '15px' }}>{formatMoney(r.net_paise)}</td>
              </tr>
            </tbody>
          </table>
          <div style={{ fontSize: '10px', color: 'var(--muted)', marginTop: '16px', textAlign: 'center' }}>{t('receipt.doc.generated')}</div>
        </div>
      ))}
    </div>
  )
}
