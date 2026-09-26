// Salary register (P15 Step 5, prototype `salary`). Principal only. Monthly
// salary per staff, unpaid-leave deduction (monthly ÷ working days × unpaid days,
// half-up — OWNER default), advances and net pay; Pay creates salary vouchers;
// slips print. Days present are entered by the Principal until Staff HR (P17).

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { SalaryRegisterDto, StaffDaysInput } from '@/lib/api'
import { Strip } from './AccountsScreen'

function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }
const COLS = '1.6fr 1fr 1fr 1fr 0.9fr 1fr 1fr 0.9fr'

function thisMonth(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
}

export default function SalaryRegister() {
  const [month, setMonth] = useState(thisMonth())
  const [days, setDays] = useState<Record<string, number>>({})
  const [data, setData] = useState<SalaryRegisterDto | null>(null)
  const [payMode, setPayMode] = useState<'cash' | 'bank'>('bank')
  const [advanceOpen, setAdvanceOpen] = useState(false)
  const [busy, setBusy] = useState(false)

  const daysList = useCallback((): StaffDaysInput[] => Object.entries(days).map(([staff_id, days_present]) => ({ staff_id, days_present })), [days])
  const load = useCallback(() => { api.salary_register(month, daysList()).then(setData).catch(() => setData(null)) }, [month, daysList])
  useEffect(load, [load])

  const pay = async () => {
    setBusy(true)
    try {
      await api.pay_salaries(month, payMode, daysList())
      load()
    } finally {
      setBusy(false)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
        <input type="month" value={month} onChange={(e) => setMonth(e.target.value)} style={{ ...secondaryBtn(), padding: '0 12px' }} />
        <button type="button" style={secondaryBtn()} onClick={() => setAdvanceOpen(true)}>{t('salary.giveAdvance')}</button>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: '10px', alignItems: 'center' }}>
          <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/salaryslips?month=${month}&auto=1`)}>{t('salary.printSlips')}</button>
          <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
            {(['cash', 'bank'] as const).map((m) => (
              <button key={m} type="button" style={{ height: '40px', padding: '0 12px', border: 'none', borderRight: '1px solid var(--line-strong)', background: payMode === m ? 'var(--accent)' : 'var(--surface)', color: payMode === m ? 'var(--white)' : 'var(--ink)', fontSize: '13px', cursor: 'pointer' }} onClick={() => setPayMode(m)}>{t(`accounts.paid.${m}`)}</button>
            ))}
          </div>
          <button type="button" disabled={busy || !data || data.pending === 0} style={{ ...primaryBtn(), opacity: busy || !data || data.pending === 0 ? 0.5 : 1 }} onClick={pay}>
            {t('salary.payN', { n: String(data?.pending ?? 0) })}
          </button>
        </div>
      </div>

      {data ? (
        <>
          <Strip items={[
            [formatMoney(data.total_salaries_paise), t('salary.total')],
            [formatMoney(data.advances_recovered_paise), t('salary.advances')],
            [formatMoney(data.unpaid_deducted_paise), t('salary.deducted')],
            [formatMoney(data.net_to_pay_paise), t('salary.net')],
          ]} />
          <div style={CARD}>
            <div style={{ display: 'grid', gridTemplateColumns: COLS, padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
              <span>{t('salary.col.staff')}</span>
              <span>{t('salary.col.role')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.monthly')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.days')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.advance')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.deduction')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.net')}</span>
              <span style={{ textAlign: 'right' }}>{t('salary.col.status')}</span>
            </div>
            {data.rows.map((r, i) => (
              <div key={r.staff_id} style={{ display: 'grid', gridTemplateColumns: COLS, alignItems: 'center', padding: '13px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px', fontVariantNumeric: 'tabular-nums' }}>
                <span style={{ fontWeight: 500 }}>{r.name}</span>
                <span style={{ color: 'var(--muted)' }}>{t(`salary.role.${r.role}`)}</span>
                <span style={{ textAlign: 'right' }}>{formatMoney(r.monthly_paise)}</span>
                <span style={{ textAlign: 'right', display: 'flex', gap: '4px', justifyContent: 'flex-end', alignItems: 'center' }}>
                  {r.paid ? (
                    <span>{r.days_present} / {r.working_days}</span>
                  ) : (
                    <>
                      <input inputMode="numeric" value={String(days[r.staff_id] ?? r.days_present)} onChange={(e) => setDays({ ...days, [r.staff_id]: Math.max(0, Math.min(r.working_days, Number(e.target.value.replace(/[^0-9]/g, '') || '0'))) })} style={{ width: '38px', height: '30px', textAlign: 'right', borderRadius: '4px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '13px', fontVariantNumeric: 'tabular-nums' }} />
                      <span style={{ color: 'var(--muted)' }}>/ {r.working_days}</span>
                    </>
                  )}
                </span>
                <span style={{ textAlign: 'right' }}>{r.advance_recovery_paise > 0 ? formatMoney(r.advance_recovery_paise) : '—'}</span>
                <span style={{ textAlign: 'right' }}>
                  {r.deduction_paise > 0 ? (
                    <span>{formatMoney(r.deduction_paise)}<span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>{t('salary.unpaidDays', { n: String(r.unpaid_leave_days) })}</span></span>
                  ) : '—'}
                </span>
                <span style={{ textAlign: 'right', fontWeight: 600 }}>{formatMoney(r.net_paise)}</span>
                <span style={{ textAlign: 'right' }}>{r.paid ? <Pill variant="paid">{t('salary.paid')}</Pill> : <Pill variant="partpaid">{t('salary.pending')}</Pill>}</span>
              </div>
            ))}
          </div>
          <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('salary.footer')}</div>
        </>
      ) : null}

      {advanceOpen ? <GiveAdvanceSheet rows={data?.rows ?? []} onClose={() => setAdvanceOpen(false)} onSaved={() => { setAdvanceOpen(false); load() }} /> : null}
    </div>
  )
}

function GiveAdvanceSheet({ rows, onClose, onSaved }: { rows: SalaryRegisterDto['rows']; onClose: () => void; onSaved: () => void }) {
  const [staffId, setStaffId] = useState(rows[0]?.staff_id ?? '')
  const [amount, setAmount] = useState('')
  const [recover, setRecover] = useState('')
  const [mode, setMode] = useState<'cash' | 'bank'>('cash')
  const [busy, setBusy] = useState(false)
  const field: CSSProperties = { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', padding: '0 12px', boxSizing: 'border-box' }
  const save = async () => {
    setBusy(true)
    try {
      await api.give_advance({ staff_id: staffId, amount_paise: Math.round(Number(amount || '0') * 100), recover_per_month_paise: Math.round(Number(recover || '0') * 100), mode })
      onSaved()
    } finally {
      setBusy(false)
    }
  }
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '420px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '24px' }}>{t('salary.giveAdvance')}</div>
        <div style={{ padding: '8px 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
          <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>{t('salary.col.staff')}
            <select value={staffId} onChange={(e) => setStaffId(e.target.value)} style={field}>
              {rows.map((r) => <option key={r.staff_id} value={r.staff_id}>{r.name}</option>)}
            </select>
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>{t('salary.advanceAmount')}<input inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ''))} style={field} /></label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>{t('salary.recoverPerMonth')}<input inputMode="decimal" value={recover} onChange={(e) => setRecover(e.target.value.replace(/[^0-9.]/g, ''))} style={field} /></label>
          <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
            {(['cash', 'bank'] as const).map((m) => (
              <button key={m} type="button" style={{ flex: 1, height: '40px', border: 'none', borderRight: '1px solid var(--line-strong)', background: mode === m ? 'var(--accent)' : 'var(--surface)', color: mode === m ? 'var(--white)' : 'var(--ink)', fontSize: '13px', cursor: 'pointer' }} onClick={() => setMode(m)}>{t(`accounts.paid.${m}`)}</button>
            ))}
          </div>
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('accounts.cancel')}</button>
          <button type="button" disabled={busy || !staffId || Number(amount) <= 0} style={{ ...primaryBtn(), opacity: busy || !staffId || Number(amount) <= 0 ? 0.5 : 1 }} onClick={save}>{t('salary.giveAdvance')}</button>
        </div>
      </div>
    </div>
  )
}
