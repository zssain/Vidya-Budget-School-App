// Staff attendance register (P17 Step 2) — the Principal's Staff & access →
// Staff attendance tab. Day list (status pills; accept/reject away check-ins) and
// a month grid, with print. Real data: staff_attendance_day / _month,
// accept_away_checkin / reject_away_checkin. Principal only (server re-checks).

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, StaffAttnRow, StaffAttnMonthDto } from '@/lib/api'
import { t, useLang } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, overflow: 'hidden' }

function today(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}
function fmtTime(min: number | null): string {
  if (min == null) return '—'
  const h = Math.floor(min / 60)
  const m = min % 60
  const ampm = h < 12 ? 'AM' : 'PM'
  const h12 = h % 12 === 0 ? 12 : h % 12
  return `${h12}:${String(m).padStart(2, '0')} ${ampm}`
}

const PILL: Record<string, React.CSSProperties> = {
  present: { background: 'var(--accent-12)', color: 'var(--accent)' },
  late: { background: 'var(--pill-partpaid-bg)', color: 'var(--pill-partpaid-fg)' },
  away_pending: { background: 'var(--pill-partpaid-bg)', color: 'var(--pill-partpaid-fg)' },
  absent: { background: 'var(--pill-unpaid-bg)', color: 'var(--pill-unpaid-fg)' },
  leave: { background: 'var(--pill-marks-bg)', color: 'var(--pill-marks-fg)' },
  half_day: { background: 'var(--panel)', color: 'var(--muted)' },
}
function StatusPill({ status }: { status: string | null }) {
  const s = status ?? 'none'
  const style = PILL[s] ?? { background: 'var(--panel)', color: 'var(--muted)' }
  const label = status ? t(`staffhr.status.${status}`) : t('staffhr.reg.notMarked')
  return <span style={{ display: 'inline-flex', alignItems: 'center', height: 24, padding: '0 10px', borderRadius: 4, fontSize: 12, fontWeight: 500, ...style }}>{label}</span>
}

export default function StaffAttendanceRegister() {
  useLang()
  const [view, setView] = useState<'day' | 'month'>('day')
  const [date, setDate] = useState(today())
  const [month, setMonth] = useState(today().slice(0, 7))
  const [dayRows, setDayRows] = useState<StaffAttnRow[]>([])
  const [monthData, setMonthData] = useState<StaffAttnMonthDto | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const load = useCallback(() => {
    setErr(null)
    if (view === 'day') api.staff_attendance_day(date).then(setDayRows).catch((e) => setErr(t((e as CmdError).message_key)))
    else api.staff_attendance_month(month).then(setMonthData).catch((e) => setErr(t((e as CmdError).message_key)))
  }, [view, date, month])
  useEffect(load, [load])

  const decide = (id: string, accept: boolean) => {
    const p = accept ? api.accept_away_checkin(id) : api.reject_away_checkin(id)
    p.then(load).catch((e) => setErr(t((e as CmdError).message_key)))
  }

  const seg = (active: boolean): React.CSSProperties => ({ height: 36, padding: '0 16px', borderRadius: 6, fontSize: 13, fontWeight: 500, cursor: 'pointer', border: `1px solid ${active ? 'var(--accent)' : 'var(--line-strong)'}`, background: active ? 'var(--accent)' : 'var(--white)', color: active ? 'var(--white)' : 'var(--ink)' })
  const inputStyle: React.CSSProperties = { height: 36, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', padding: '0 10px', fontSize: 14, color: 'var(--ink)' }

  return (
    <div>
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', marginBottom: 16, flexWrap: 'wrap' }}>
        <button type="button" onClick={() => setView('day')} style={seg(view === 'day')}>{t('staffhr.reg.day')}</button>
        <button type="button" onClick={() => setView('month')} style={seg(view === 'month')}>{t('staffhr.reg.month')}</button>
        {view === 'day' ? (
          <input type="date" value={date} onChange={(e) => setDate(e.target.value)} style={inputStyle} />
        ) : (
          <input type="month" value={month} onChange={(e) => setMonth(e.target.value)} style={inputStyle} />
        )}
        {view === 'month' && monthData ? <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('staffhr.reg.workingDays', { n: monthData.working_days })}</span> : null}
        <button type="button" onClick={() => window.print()} style={{ ...seg(false), marginLeft: 'auto' }}>{t('staffhr.reg.print')}</button>
      </div>
      {err ? <p style={{ color: 'var(--danger)', fontSize: 13 }}>{err}</p> : null}

      {view === 'day' ? (
        <div style={CARD}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1.2fr 2fr auto', padding: '12px 24px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
            <span>{t('staffhr.reg.name')}</span><span>{t('staffhr.reg.status')}</span><span>{t('staffhr.reg.times')}</span><span />
          </div>
          {dayRows.map((r) => (
            <div key={r.staff_id} style={{ display: 'grid', gridTemplateColumns: '2fr 1.2fr 2fr auto', alignItems: 'center', gap: 8, padding: '12px 24px', borderBottom: '1px solid var(--track)' }}>
              <span style={{ fontWeight: 500 }}>{r.name}</span>
              <span><StatusPill status={r.status} /></span>
              <span style={{ fontSize: 13, color: 'var(--muted)' }}>
                {r.check_in_min != null ? `${t('staffhr.reg.in')} ${fmtTime(r.check_in_min)}` : '—'}
                {r.check_out_min != null ? ` · ${t('staffhr.reg.out')} ${fmtTime(r.check_out_min)}` : ''}
                {r.clock_warning ? <span style={{ color: 'var(--gold-text)', marginLeft: 6 }}>· {t('staffhr.reg.clockWarn')}</span> : null}
              </span>
              <span style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                {r.status === 'away_pending' && r.id ? (
                  <>
                    <button type="button" onClick={() => decide(r.id!, true)} style={{ height: 30, padding: '0 12px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 12, fontWeight: 500, cursor: 'pointer' }}>{t('staffhr.reg.accept')}</button>
                    <button type="button" onClick={() => decide(r.id!, false)} style={{ height: 30, padding: '0 12px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--danger)', fontSize: 12, cursor: 'pointer' }}>{t('staffhr.reg.reject')}</button>
                  </>
                ) : null}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <div style={CARD}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr repeat(5, 1fr)', padding: '12px 24px', fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
            <span>{t('staffhr.reg.name')}</span>
            <span>{t('staffhr.status.present')}</span>
            <span>{t('staffhr.status.late')}</span>
            <span>{t('staffhr.status.leave')}</span>
            <span>{t('staffhr.status.absent')}</span>
            <span>{t('staffhr.status.away_pending')}</span>
          </div>
          {(monthData?.rows ?? []).map((r) => (
            <div key={r.staff_id} style={{ display: 'grid', gridTemplateColumns: '2fr repeat(5, 1fr)', alignItems: 'center', padding: '12px 24px', borderBottom: '1px solid var(--track)', fontSize: 14, fontVariantNumeric: 'tabular-nums' }}>
              <span style={{ fontWeight: 500 }}>{r.name}</span>
              <span>{r.present}</span>
              <span>{r.late}</span>
              <span>{r.leave}</span>
              <span style={{ color: r.absent > 0 ? 'var(--danger)' : 'var(--ink)' }}>{r.absent}</span>
              <span style={{ color: r.away_pending > 0 ? 'var(--gold-text)' : 'var(--ink)' }}>{r.away_pending}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
