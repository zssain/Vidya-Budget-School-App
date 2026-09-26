// Accounts (P15 Step 4, prototype `accounts` states 1/2/3). Finance → Accounts,
// tabs: Cash book · Day book · Expenses · Profit · Salaries. Every number comes
// from the ledger, so cash-book "money in" = day book total and debits = credits.
// Accountant sees Cash book / Day book / Expenses; Principal sees all tabs.

import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { CashBookDto, ExpenseDto, OpeningBalanceDto, ProfitDto } from '@/lib/api'
import DayBookScreen from './DayBookScreen'
import RecordExpenseSheet from './RecordExpenseSheet'
import type { Role } from '@/lib/nav'

// The Salaries tab is added in Step 5 (salary register). For now Accounts covers
// Cash book · Day book · Expenses · Profit.
type Tab = 'cashbook' | 'daybook' | 'expenses' | 'profit'

function seg(sel: boolean): CSSProperties {
  return { minWidth: '64px', padding: '0 16px', height: '38px', fontSize: '13px', fontWeight: 500, border: 'none', borderRight: '1px solid var(--line-strong)', background: sel ? 'var(--accent)' : 'transparent', color: sel ? 'var(--white)' : 'var(--ink)', cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }

/** Prototype Strip: N metric cards on a panel background. */
export function Strip({ items }: { items: [string, string, string?][] }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: `repeat(${items.length}, minmax(0,1fr))`, background: 'var(--panel)', borderRadius: '16px' }}>
      {items.map(([v, l, n], i) => (
        <div key={l} style={{ display: 'flex', flexDirection: 'column', gap: '8px', padding: '20px 24px', borderLeft: i ? '1px solid var(--line-stat)' : undefined }}>
          <span style={{ fontFamily: 'var(--font-serif)', fontSize: '32px', lineHeight: 1, letterSpacing: '-0.02em', fontVariantNumeric: 'tabular-nums' }}>{v}</span>
          <span style={{ fontSize: '14px' }}>{l}</span>
          {n ? <span style={{ fontSize: '12px', color: 'var(--muted)' }}>{n}</span> : null}
        </div>
      ))}
    </div>
  )
}

export default function AccountsScreen({ role }: { role: Role }) {
  const tabs: Tab[] = role === 'accountant' ? ['cashbook', 'daybook', 'expenses'] : ['cashbook', 'daybook', 'expenses', 'profit']
  const [tab, setTab] = useState<Tab>('cashbook')
  const [expenseOpen, setExpenseOpen] = useState(false)

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('accounts.eyebrow')}
        title={t('accounts.title')}
        sub={t('accounts.sub')}
        actions={
          tab !== 'daybook' ? (
            <button type="button" style={primaryBtn()} onClick={() => setExpenseOpen(true)}>{t('accounts.recordExpense')}</button>
          ) : undefined
        }
      />

      <div style={{ display: 'inline-flex', alignSelf: 'flex-start', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
        {tabs.map((k) => (
          <button key={k} type="button" style={seg(tab === k)} onClick={() => setTab(k)}>{t(`accounts.tab.${k}`)}</button>
        ))}
      </div>

      {tab === 'cashbook' ? <CashBookTab role={role} /> : null}
      {tab === 'daybook' ? <DayBookScreen /> : null}
      {tab === 'expenses' ? <ExpensesTab role={role} /> : null}
      {tab === 'profit' ? <ProfitTab /> : null}

      {expenseOpen ? <RecordExpenseSheet onClose={() => setExpenseOpen(false)} onSaved={() => { setExpenseOpen(false) }} /> : null}
    </div>
  )
}
// The Expenses/Cash-book tabs re-read on next mount; a saved expense shows after
// switching tabs or reopening (kept simple — no global store, §13).

// ---- Cash book (prototype accounts state 1) ---------------------------------

function todayIso(): string {
  return new Date().toISOString().slice(0, 10)
}

function CashBookTab({ role }: { role: Role }) {
  const [date, setDate] = useState(todayIso())
  const [data, setData] = useState<CashBookDto | null>(null)
  const [opening, setOpening] = useState<OpeningBalanceDto | null>(null)

  const load = useCallback(() => {
    api.cash_book(date).then(setData).catch(() => setData(null))
    api.get_opening_balance().then(setOpening).catch(() => setOpening(null))
  }, [date])
  useEffect(load, [load])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {opening && !opening.set && role !== 'accountant' ? <OpeningBalanceBanner onSet={load} /> : null}
      <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
        <input type="date" value={date} onChange={(e) => setDate(e.target.value)} style={{ ...secondaryBtn(), padding: '0 12px' }} />
        <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/daybook?date=${date}&auto=1`)}>{t('accounts.printDay')}</button>
      </div>
      {data ? (
        <>
          <Strip items={[
            [formatMoney(data.opening_paise), t('accounts.cb.opening')],
            [formatMoney(data.money_in_paise), t('accounts.cb.in')],
            [formatMoney(data.money_out_paise), t('accounts.cb.out')],
            [formatMoney(data.in_hand_paise), t('accounts.cb.inHand'), t('accounts.cb.inHandNote')],
          ]} />
          <div style={CARD}>
            <div style={{ display: 'grid', gridTemplateColumns: '0.7fr 1fr 2.2fr 1fr 1fr 1fr', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
              <span>{t('accounts.cb.time')}</span>
              <span>{t('accounts.cb.ref')}</span>
              <span>{t('accounts.cb.details')}</span>
              <span style={{ textAlign: 'right' }}>{t('accounts.cb.moneyIn')}</span>
              <span style={{ textAlign: 'right' }}>{t('accounts.cb.moneyOut')}</span>
              <span style={{ textAlign: 'right' }}>{t('accounts.cb.balance')}</span>
            </div>
            {data.rows.length === 0 ? (
              <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('accounts.cb.none')}</div>
            ) : (
              data.rows.map((r, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: '0.7fr 1fr 2.2fr 1fr 1fr 1fr', alignItems: 'center', padding: '13px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px', fontVariantNumeric: 'tabular-nums' }}>
                  <span style={{ color: 'var(--muted)' }}>{r.time}</span>
                  <span style={{ color: r.ref_no.startsWith('R-') ? 'var(--accent)' : 'var(--gold-text)', fontWeight: 500 }}>{r.ref_no}</span>
                  <span style={{ minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.details}</span>
                  <span style={{ textAlign: 'right', color: 'var(--accent)' }}>{r.in_paise > 0 ? formatMoney(r.in_paise) : ''}</span>
                  <span style={{ textAlign: 'right', color: 'var(--danger)' }}>{r.out_paise > 0 ? formatMoney(r.out_paise) : ''}</span>
                  <span style={{ textAlign: 'right', fontWeight: 500 }}>{formatMoney(r.balance_paise)}</span>
                </div>
              ))
            )}
          </div>
        </>
      ) : null}
    </div>
  )
}

function OpeningBalanceBanner({ onSet }: { onSet: () => void }) {
  const [open, setOpen] = useState(false)
  const [cash, setCash] = useState('')
  const [bank, setBank] = useState('')
  const [busy, setBusy] = useState(false)
  const save = async () => {
    setBusy(true)
    try {
      await api.set_opening_balance(Math.round(Number(cash || '0') * 100), Math.round(Number(bank || '0') * 100))
      setOpen(false)
      onSet()
    } finally {
      setBusy(false)
    }
  }
  const field: CSSProperties = { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', padding: '0 12px', boxSizing: 'border-box' }
  return (
    <div style={{ display: 'flex', gap: '12px', alignItems: 'center', padding: '14px 18px', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', borderRadius: '12px', color: 'var(--gold-text)', fontSize: '14px' }}>
      <span style={{ flex: 1 }}>{t('accounts.opening.prompt')}</span>
      <button type="button" style={primaryBtn()} onClick={() => setOpen(true)}>{t('accounts.opening.set')}</button>
      {open ? (
        <div onClick={() => setOpen(false)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: '400px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
            <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '24px', color: 'var(--ink)' }}>{t('accounts.opening.title')}</div>
            <div style={{ padding: '8px 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
              <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>{t('accounts.opening.cash')}<input inputMode="decimal" value={cash} onChange={(e) => setCash(e.target.value.replace(/[^0-9.]/g, ''))} style={field} /></label>
              <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>{t('accounts.opening.bank')}<input inputMode="decimal" value={bank} onChange={(e) => setBank(e.target.value.replace(/[^0-9.]/g, ''))} style={field} /></label>
            </div>
            <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              <button type="button" style={secondaryBtn()} onClick={() => setOpen(false)}>{t('accounts.cancel')}</button>
              <button type="button" style={{ ...primaryBtn(), opacity: busy ? 0.5 : 1 }} disabled={busy} onClick={save}>{t('accounts.opening.save')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}

// ---- Expenses list ----------------------------------------------------------

function monthStart(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-01`
}

function ExpensesTab({ role }: { role: Role }) {
  const [from, setFrom] = useState(monthStart())
  const [to, setTo] = useState(todayIso())
  const [rows, setRows] = useState<ExpenseDto[]>([])
  const [reversing, setReversing] = useState<ExpenseDto | null>(null)
  const load = useCallback(() => { api.list_expenses(from, to).then(setRows).catch(() => setRows([])) }, [from, to])
  useEffect(load, [load])
  const total = rows.filter((r) => !r.reversed).reduce((n, r) => n + r.amount_paise, 0)
  const dateField: CSSProperties = { ...secondaryBtn(), padding: '0 12px' }
  const cols = role === 'accountant' ? '1fr 1.4fr 2.6fr 1fr 1fr' : '1fr 1.4fr 2.2fr 1fr 1fr 0.9fr'
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
        <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} style={dateField} />
        <span style={{ color: 'var(--muted)' }}>→</span>
        <input type="date" value={to} onChange={(e) => setTo(e.target.value)} style={dateField} />
        <span style={{ marginLeft: 'auto', fontSize: '14px', fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>{t('accounts.exp.total', { total: formatMoney(total) })}</span>
      </div>
      <div style={CARD}>
        <div style={{ display: 'grid', gridTemplateColumns: cols, padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('accounts.exp.date')}</span>
          <span>{t('accounts.exp.category')}</span>
          <span>{t('accounts.exp.details')}</span>
          <span style={{ textAlign: 'right' }}>{t('accounts.exp.amount')}</span>
          <span style={{ textAlign: 'right' }}>{t('accounts.exp.voucher')}</span>
          {role !== 'accountant' ? <span /> : null}
        </div>
        {rows.length === 0 ? (
          <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('accounts.exp.none')}</div>
        ) : (
          rows.map((r, i) => (
            <div key={r.id} style={{ display: 'grid', gridTemplateColumns: cols, alignItems: 'center', padding: '13px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px', fontVariantNumeric: 'tabular-nums', opacity: r.reversed ? 0.55 : 1 }}>
              <span style={{ color: 'var(--muted)' }}>{r.spent_on}</span>
              <span>{r.category_name}</span>
              <span style={{ minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.details ?? '—'}{r.reversed ? ` · ${t('accounts.exp.reversed')}` : ''}</span>
              <span style={{ textAlign: 'right', fontWeight: 500, textDecoration: r.reversed ? 'line-through' : undefined }}>{formatMoney(r.amount_paise)}</span>
              <span style={{ textAlign: 'right', color: 'var(--gold-text)' }}>{r.voucher_no ?? '—'}</span>
              {role !== 'accountant' ? (
                <span style={{ textAlign: 'right' }}>
                  {!r.reversed ? <button type="button" style={{ ...secondaryBtn(), height: '32px', padding: '0 10px' }} onClick={() => setReversing(r)}>{t('accounts.exp.reverse')}</button> : null}
                </span>
              ) : null}
            </div>
          ))
        )}
      </div>
      {reversing ? <ReverseExpenseModal expense={reversing} onClose={() => setReversing(null)} onDone={() => { setReversing(null); load() }} /> : null}
    </div>
  )
}

function ReverseExpenseModal({ expense, onClose, onDone }: { expense: ExpenseDto; onClose: () => void; onDone: () => void }) {
  const [reason, setReason] = useState('')
  const [busy, setBusy] = useState(false)
  const go = async () => {
    setBusy(true)
    try {
      await api.reverse_expense(expense.id, reason)
      onDone()
    } finally {
      setBusy(false)
    }
  }
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '400px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '22px' }}>{t('accounts.exp.reverse')} · {expense.category_name} · {formatMoney(expense.amount_paise)}</div>
        <div style={{ padding: '8px 24px 20px' }}>
          <textarea value={reason} onChange={(e) => setReason(e.target.value)} placeholder={t('accounts.exp.reverseReason')} rows={3} style={{ width: '100%', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '10px 12px', boxSizing: 'border-box', resize: 'vertical' }} />
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('accounts.cancel')}</button>
          <button type="button" style={{ ...primaryBtn(), opacity: busy || reason.trim() === '' ? 0.5 : 1 }} disabled={busy || reason.trim() === ''} onClick={go}>{t('accounts.exp.reverse')}</button>
        </div>
      </div>
    </div>
  )
}

// ---- Profit summary (prototype accounts state 3) ----------------------------

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
function monthLabel(m: string): string {
  const mm = Number(m.slice(5, 7))
  return MONTHS[mm - 1] ?? m
}

function ProfitTab() {
  const [data, setData] = useState<ProfitDto | null>(null)
  useEffect(() => { api.profit_summary().then(setData).catch(() => setData(null)) }, [])
  const max = useMemo(() => {
    if (!data) return 1
    return Math.max(1, ...data.months.flatMap((m) => [m.income_paise, m.expense_paise]))
  }, [data])
  if (!data) return null
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button type="button" style={secondaryBtn()} onClick={() => window.print()}>{t('accounts.print')}</button>
      </div>
      <Strip items={[
        [formatMoney(data.income_paise), t('accounts.pf.income')],
        [formatMoney(data.expense_paise), t('accounts.pf.expenses')],
        [formatMoney(data.surplus_paise), t('accounts.pf.surplus')],
        [formatMoney(data.fees_due_paise), t('accounts.pf.due')],
      ]} />
      {/* CSS bar chart, income vs expense per month (vGrow animation, prototype). */}
      <div style={{ background: 'var(--navy)', color: 'var(--white)', borderRadius: '16px', padding: '20px 28px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
          <span style={{ fontFamily: 'var(--font-serif)', fontSize: '24px' }}>{t('accounts.pf.chartTitle')}</span>
          <div style={{ display: 'flex', gap: '18px', fontSize: '12px', color: 'var(--on-navy-muted)' }}>
            <span style={{ display: 'inline-flex', gap: '6px', alignItems: 'center' }}><span style={{ width: 10, height: 10, borderRadius: 2, background: 'var(--teal-soft)' }} />{t('accounts.pf.income')}</span>
            <span style={{ display: 'inline-flex', gap: '6px', alignItems: 'center' }}><span style={{ width: 10, height: 10, borderRadius: 2, background: 'var(--gold)' }} />{t('accounts.pf.expenses')}</span>
          </div>
        </div>
        <div style={{ display: 'flex', gap: '30px', height: 200, paddingTop: 10, alignItems: 'flex-end' }}>
          {data.months.map((m, i) => (
            <div key={m.month} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
              <div style={{ display: 'flex', gap: '6px', alignItems: 'flex-end', height: 170 }}>
                <span style={{ width: 26, height: `${(m.income_paise / max) * 170}px`, background: 'var(--teal-soft)', borderRadius: '4px 4px 2px 2px', transformOrigin: 'bottom', animation: `vGrow 700ms ${200 + i * 80}ms cubic-bezier(.2,.8,.2,1) both` }} />
                <span style={{ width: 26, height: `${(m.expense_paise / max) * 170}px`, background: 'var(--gold)', borderRadius: '4px 4px 2px 2px', transformOrigin: 'bottom', animation: `vGrow 700ms ${240 + i * 80}ms cubic-bezier(.2,.8,.2,1) both` }} />
              </div>
              <span style={{ fontSize: 12, color: 'var(--on-navy-muted)' }}>{monthLabel(m.month)}</span>
            </div>
          ))}
        </div>
      </div>
      <div style={CARD}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', padding: '11px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('accounts.pf.month')}</span>
          <span style={{ textAlign: 'right' }}>{t('accounts.pf.income')}</span>
          <span style={{ textAlign: 'right' }}>{t('accounts.pf.expenses')}</span>
          <span style={{ textAlign: 'right' }}>{t('accounts.pf.surplus')}</span>
        </div>
        {data.months.map((m, i) => (
          <div key={m.month} style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', padding: '11px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px', fontVariantNumeric: 'tabular-nums' }}>
            <span>{monthLabel(m.month)} {m.month.slice(0, 4)}</span>
            <span style={{ textAlign: 'right' }}>{formatMoney(m.income_paise)}</span>
            <span style={{ textAlign: 'right' }}>{formatMoney(m.expense_paise)}</span>
            <span style={{ textAlign: 'right', fontWeight: 600, color: m.surplus_paise < 0 ? 'var(--danger)' : 'var(--accent)' }}>{formatMoney(m.surplus_paise)}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
