// Apply for leave (P17 Step 3, prototype `staffday` states 2–3). Phone form: leave
// type (segmented, from the staff member's balances), from/to dates, reason, the
// remaining balance for the chosen type, and a note that the Principal will arrange
// a substitute. Send → request_leave (a `leave` request in the P13 registry) →
// Sent state + toast. My requests then shows it like any other request.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, LeaveBalance } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { navigate } from '@/lib/router'
import { t, useLang } from '@/lib/i18n'

function today(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

export default function ApplyLeaveScreen() {
  useLang()
  const [balances, setBalances] = useState<LeaveBalance[]>([])
  const [typeId, setTypeId] = useState('')
  const [from, setFrom] = useState(today())
  const [to, setTo] = useState(today())
  const [reason, setReason] = useState('')
  const [sent, setSent] = useState(false)
  const [toast, setToast] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const load = useCallback(() => {
    api.my_staff_day(today()).then((d) => {
      setBalances(d.leave_balances)
      if (d.leave_balances.length > 0) setTypeId((cur) => cur || d.leave_balances[0].leave_type_id)
    }).catch(() => setBalances([]))
  }, [])
  useEffect(load, [load])

  const sel = balances.find((b) => b.leave_type_id === typeId)

  const send = async () => {
    setErr(null)
    if (reason.trim().length === 0) { setErr(t('staffhr.leaveform.errReason')); return }
    try {
      await api.request_leave({ leave_type_id: typeId, from, to, reason: reason.trim() })
      setSent(true)
      setToast(true)
      setTimeout(() => setToast(false), 2200)
    } catch (e) {
      const rule = ((e as CmdError).vars as { rule?: string } | null)?.rule ?? ''
      if (rule === 'overlap') setErr(t('staffhr.leaveform.errOverlap'))
      else if (rule === 'no_working_days') setErr(t('staffhr.leaveform.errNoWorking'))
      else if (rule === 'from_after_to') setErr(t('staffhr.leaveform.errDates'))
      else if ((e as CmdError).code === 'REQUEST_ALREADY_PENDING') setErr(t('staffhr.leaveform.errPending'))
      else setErr(t('staffhr.leaveform.errDates'))
    }
  }

  const field: React.CSSProperties = { borderRadius: 12, border: '1px solid var(--line)', background: 'var(--white)', color: 'var(--ink)', padding: '12px 14px', fontSize: 15, boxSizing: 'border-box', width: '100%' }

  const balanceText = sel ? (sel.balance == null ? t('staffhr.leaveform.unlimited') : t('staffhr.leaveform.balanceVal', { balance: Math.max(0, sel.balance), quota: sel.yearly_quota ?? 0 })) : '—'

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: '10px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button type="button" aria-label={t('staffhr.day.back')} onClick={() => navigate('/teacher/checkin')} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('staffhr.leaveform.title')}</h1>
      </header>

      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('staffhr.leaveform.type')}</span>
          <div style={{ display: 'grid', gridTemplateColumns: `repeat(${Math.max(1, balances.length)}, 1fr)`, border: '1px solid var(--line-strong)', borderRadius: 6, overflow: 'hidden' }}>
            {balances.map((b, i) => {
              const on = b.leave_type_id === typeId
              return (
                <button key={b.leave_type_id} type="button" onClick={() => setTypeId(b.leave_type_id)} disabled={sent} style={{ height: 44, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: 0, borderLeft: i === 0 ? 0 : '1px solid var(--line-strong)', background: on ? 'var(--accent)' : 'var(--white)', color: on ? 'var(--white)' : 'var(--ink)' }}>{b.name}</button>
              )
            })}
          </div>
        </div>

        <div data-hl="dates" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('staffhr.leaveform.from')}</span>
            <input type="date" value={from} disabled={sent} onChange={(e) => setFrom(e.target.value)} style={field} />
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('staffhr.leaveform.to')}</span>
            <input type="date" value={to} disabled={sent} onChange={(e) => setTo(e.target.value)} style={field} />
          </label>
        </div>

        <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('staffhr.leaveform.reason')}</span>
          <input value={reason} disabled={sent} onChange={(e) => setReason(e.target.value)} placeholder={t('staffhr.leaveform.reasonPlaceholder')} style={field} />
        </label>

        <div data-hl="balance" style={{ borderRadius: 12, background: 'var(--panel)', padding: '12px 14px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>{sel ? t('staffhr.leaveform.balance', { name: sel.name }) : '—'}</span>
          <b style={{ fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 20 }}>{balanceText}</b>
        </div>
        <span style={{ fontSize: 12, color: 'var(--muted)' }}>{t('staffhr.leaveform.subNote')}</span>
        {err ? <span style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span> : null}

        <div style={{ marginTop: 'auto' }}>
          {sent ? (
            <button type="button" disabled style={{ width: '100%', height: 52, borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--muted)', fontSize: 15, fontWeight: 500 }}>{t('staffhr.leaveform.sent')}</button>
          ) : (
            <button type="button" onClick={send} disabled={!typeId} data-hl="send" style={{ width: '100%', height: 52, borderRadius: 8, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 15, fontWeight: 600, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8 }}>
              {t('staffhr.leaveform.send')} <Icon name="arrowUpRight" size={18} />
            </button>
          )}
        </div>
      </div>

      {toast ? (
        <div role="status" style={{ position: 'absolute', left: 16, right: 16, bottom: 90, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 14px', fontSize: 14, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Icon name="check" size={16} strokeWidth={2.2} /> {t('staffhr.leaveform.toast')}
        </div>
      ) : null}
    </div>
  )
}
