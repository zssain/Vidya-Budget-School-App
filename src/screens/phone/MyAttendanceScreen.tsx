// Staff HR (P17 Step 2, prototype `staffday` state 1). A staff member's own
// attendance on the phone: a big check-in card (before check-in: "Check in"; after:
// "Checked in at 8:47 AM" with the route text), this month's counts, a recent-days
// list, and "Apply for leave". One check-in per working day; an away check-in (not
// on the school Wi-Fi) waits for the Principal unless "allow away" is on (§10.5).
//
// Route: on-school-Wi-Fi (LAN reachable) counts as "at school". We probe the school
// server (`server_status`) — reachable → `lan`, else `drive`. A real phone's live
// LAN probe is part of the sync layer (multi-device wiring deferred, P14–P16
// precedent); this is honest in the single-PC + LAN case.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, StaffDayDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { navigate } from '@/lib/router'
import { t, useLang } from '@/lib/i18n'

function localToday(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}
function localMinute(): number {
  const d = new Date()
  return d.getHours() * 60 + d.getMinutes()
}
function fmtTime(min: number | null | undefined): string {
  if (min == null) return '—'
  const h = Math.floor(min / 60)
  const m = min % 60
  const ampm = h < 12 ? 'AM' : 'PM'
  const h12 = h % 12 === 0 ? 12 : h % 12
  return `${h12}:${String(m).padStart(2, '0')} ${ampm}`
}
async function detectRoute(): Promise<'lan' | 'drive'> {
  try {
    await api.server_status()
    return 'lan'
  } catch {
    return 'drive'
  }
}

function Count({ n, label }: { n: number; label: string }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <span style={{ fontFamily: 'var(--font-serif)', fontSize: 26, lineHeight: 1 }}>{n}</span>
      <span style={{ fontSize: 11, color: '#9FACBF', letterSpacing: '0.02em' }}>{label}</span>
    </div>
  )
}

const STATUS_KEY: Record<string, string> = {
  present: 'staffhr.status.present', late: 'staffhr.status.late', away_pending: 'staffhr.status.away_pending',
  absent: 'staffhr.status.absent', leave: 'staffhr.status.leave', half_day: 'staffhr.status.half_day',
}

export default function MyAttendanceScreen() {
  useLang()
  const [day, setDay] = useState<StaffDayDto | null>(null)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const load = useCallback(() => {
    api.my_staff_day(localToday()).then(setDay).catch(() => setDay(null))
  }, [])
  useEffect(load, [load])

  const doCheckIn = async () => {
    setErr(null)
    setBusy(true)
    try {
      const route = await detectRoute()
      await api.staff_check_in({ route, date: localToday(), minute: localMinute(), device_ms: Date.now() })
      load()
    } catch (e) {
      // VALIDATION carries { field, rule } in `vars`: not_working_day / on_leave / already.
      const rule = ((e as CmdError).vars as { rule?: string } | null)?.rule ?? ''
      if (rule === 'on_leave') setErr(t('staffhr.day.onLeaveToday'))
      else if (rule === 'already') setErr(t('staffhr.day.already'))
      else setErr(t('staffhr.day.notWorking'))
    } finally {
      setBusy(false)
    }
  }

  const doCheckOut = async () => {
    setBusy(true)
    try {
      await api.staff_check_out(localToday(), localMinute())
      load()
    } catch (e) {
      void (e as CmdError)
    } finally {
      setBusy(false)
    }
  }

  const today = day?.today ?? null
  const checkedIn = !!today && today.check_in_min != null
  const away = today?.status === 'away_pending'
  const late = today?.status === 'late'
  const onLeave = today?.status === 'leave'
  const start = day?.start_time ?? '09:00'

  return (
    <div style={{ position: 'relative', width: '390px', height: '844px', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: '10px 16px 18px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        <button type="button" aria-label={t('staffhr.day.back')} onClick={() => navigate('/teacher/home')} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('staffhr.day.title')}</h1>
        <div style={{ display: 'flex', gap: 26, marginTop: 4 }}>
          <Count n={day?.month_present ?? 0} label={t('staffhr.day.present')} />
          <Count n={day?.month_leave ?? 0} label={t('staffhr.day.leave')} />
          <Count n={day?.month_late ?? 0} label={t('staffhr.day.late')} />
        </div>
      </header>

      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        {/* Check-in card */}
        <div data-hl="checkin" style={{ borderRadius: 16, background: 'var(--navy)', color: 'var(--white)', padding: 22, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, textAlign: 'center' }}>
          <span style={{ width: 60, height: 60, borderRadius: 30, border: '1px solid rgba(197,171,122,0.5)', color: 'var(--gold)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Icon name={checkedIn ? 'check' : 'clock'} size={26} strokeWidth={1.8} />
          </span>
          {onLeave ? (
            <span style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('staffhr.day.onLeaveToday')}</span>
          ) : checkedIn ? (
            <>
              <span style={{ fontFamily: 'var(--font-serif)', fontSize: 26 }}>{t('staffhr.day.checkedInAt', { time: fmtTime(today!.check_in_min) })}</span>
              <span style={{ fontSize: 13, color: '#9FACBF' }}>
                {away
                  ? t('staffhr.day.awayWaiting')
                  : late
                    ? t('staffhr.day.onWifiLate', { start })
                    : t('staffhr.day.onWifi', { start })}
              </span>
              {today!.check_out_min != null ? (
                <span style={{ fontSize: 12, color: '#9FACBF' }}>{t('staffhr.day.checkedOutAt', { time: fmtTime(today!.check_out_min) })}</span>
              ) : (
                <button type="button" onClick={doCheckOut} disabled={busy} style={{ marginTop: 6, height: 40, padding: '0 20px', borderRadius: 20, border: '1px solid rgba(255,255,255,0.3)', background: 'transparent', color: 'var(--white)', fontSize: 13, fontWeight: 500 }}>{t('staffhr.day.checkOut')}</button>
              )}
              {today!.clock_warning ? <span style={{ fontSize: 12, color: 'var(--gold)' }}>{t('staffhr.day.clockWarn')}</span> : null}
            </>
          ) : (
            <button type="button" onClick={doCheckIn} disabled={busy} data-hl="checkinbtn" style={{ marginTop: 4, height: 50, padding: '0 34px', borderRadius: 25, border: '1px solid var(--gold)', background: 'var(--gold)', color: 'var(--navy)', fontSize: 16, fontWeight: 600 }}>
              {busy ? t('staffhr.day.checkingIn') : t('staffhr.day.checkIn')}
            </button>
          )}
        </div>
        {err ? <span style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span> : null}

        {/* This month — recent days */}
        <div style={{ borderRadius: 16, background: 'var(--surface)', border: '1px solid var(--line)', overflow: 'hidden' }}>
          <div style={{ padding: '16px 20px 10px' }}>
            <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{day?.date?.slice(0, 7) ?? ''}</div>
            <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('staffhr.day.month')}</div>
          </div>
          {(day?.recent ?? []).length === 0 ? (
            <div style={{ padding: '12px 20px 18px', fontSize: 13, color: 'var(--muted)' }}>{t('staffhr.day.noRecent')}</div>
          ) : (
            day!.recent.map((r) => (
              <div key={r.date} style={{ display: 'flex', justifyContent: 'space-between', gap: 10, padding: '12px 20px', borderTop: '1px solid var(--track)', fontSize: 13 }}>
                <b style={{ fontWeight: 500 }}>{r.date}</b>
                <span style={{ color: 'var(--muted)', textAlign: 'right' }}>
                  {r.status === 'leave'
                    ? r.note || t('staffhr.status.leave')
                    : r.check_in_min != null
                      ? `${t('staffhr.reg.in')} ${fmtTime(r.check_in_min)}${r.check_out_min != null ? ` · ${t('staffhr.reg.out')} ${fmtTime(r.check_out_min)}` : ''}`
                      : t(STATUS_KEY[r.status] ?? 'staffhr.status.absent')}
                </span>
              </div>
            ))
          )}
        </div>

        <div style={{ marginTop: 'auto' }}>
          <button type="button" onClick={() => navigate('/teacher/leave')} data-hl="applyleave" style={{ width: '100%', height: 52, borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 15, fontWeight: 500, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8 }}>
            <Icon name="clock" size={18} /> {t('staffhr.day.applyLeave')}
          </button>
        </div>
      </div>
    </div>
  )
}
