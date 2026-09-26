// School calendar (P16 Step 6, prototype `calendar`). Month grid (weekly offs
// muted, today highlighted), events coloured by kind, a "Coming up" list, the
// month's working-day count, add/edit (Principal), and "Share on WhatsApp". Built
// from the mock tokens. Teachers/accountants view only (edit controls hidden).

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { t } from '@/lib/i18n'
import { shareWhatsApp } from '@/lib/files'
import * as api from '@/lib/api'
import type { CalendarDto, CalendarEventDto, CalendarEventInput } from '@/lib/api'
import type { Role } from '@/lib/nav'

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}

const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']

// kind → [bg, fg] tokens (event = accent, exam = marks pill, holiday = unpaid pill).
const TONE: Record<string, [string, string]> = {
  event: ['var(--accent-12)', 'var(--accent)'],
  exam: ['var(--pill-marks-bg)', 'var(--pill-marks-fg)'],
  holiday: ['var(--pill-unpaid-bg)', 'var(--pill-unpaid-fg)'],
}

function iso(y: number, m: number, d: number): string {
  return `${y}-${String(m + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`
}

export default function CalendarScreen({ role }: { role: Role }) {
  const now = new Date()
  const [year, setYear] = useState(now.getFullYear())
  const [month, setMonth] = useState(now.getMonth()) // 0-based
  const [data, setData] = useState<CalendarDto | null>(null)
  const [workDays, setWorkDays] = useState<number | null>(null)
  const [addOpen, setAddOpen] = useState(false)
  const canEdit = role === 'principal'

  const load = useCallback(() => {
    api.get_calendar().then(setData).catch(() => setData(null))
    const last = new Date(year, month + 1, 0).getDate()
    api.working_days(iso(year, month, 1), iso(year, month, last)).then(setWorkDays).catch(() => setWorkDays(null))
  }, [year, month])
  useEffect(load, [load])

  const todayIsThisMonth = now.getFullYear() === year && now.getMonth() === month
  const daysInMonth = new Date(year, month + 1, 0).getDate()
  const firstWeekday = new Date(year, month, 1).getDay() // 0=Sun … 6=Sat

  // Events touching this month.
  const events = data?.events ?? []
  const eventOn = (d: number): CalendarEventDto | undefined => {
    const day = iso(year, month, d)
    return events.find((e) => e.starts_on <= day && day <= e.ends_on)
  }
  // week[] is Monday=0…Sunday=6; JS getDay is Sun=0…Sat=6. Map: sun→6, else day-1.
  const isWeeklyOff = (jsWeekday: number) => data ? !data.week[jsWeekday === 0 ? 6 : jsWeekday - 1] : jsWeekday === 0

  const cells: (number | null)[] = []
  for (let i = 0; i < firstWeekday; i++) cells.push(null)
  for (let d = 1; d <= daysInMonth; d++) cells.push(d)
  while (cells.length % 7) cells.push(null)

  const prev = () => { if (month === 0) { setYear(year - 1); setMonth(11) } else setMonth(month - 1) }
  const next = () => { if (month === 11) { setYear(year + 1); setMonth(0) } else setMonth(month + 1) }

  // "Coming up" — events on/after today, next few.
  const todayStr = iso(now.getFullYear(), now.getMonth(), now.getDate())
  const coming = [...events].filter((e) => e.ends_on >= todayStr).sort((a, b) => a.starts_on.localeCompare(b.starts_on)).slice(0, 5)

  const monthLabel = `${MONTHS[month]} ${year}`
  const share = () => {
    const lines = events
      .filter((e) => e.starts_on.slice(0, 7) === iso(year, month, 1).slice(0, 7) || e.ends_on.slice(0, 7) === iso(year, month, 1).slice(0, 7))
      .map((e) => `• ${e.starts_on}${e.ends_on !== e.starts_on ? `–${e.ends_on}` : ''}: ${e.title}`)
    const text = `${t('cal.screen.shareHead', { month: monthLabel })}\n${lines.join('\n')}`
    void shareWhatsApp(null, text)
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={`${t('cal.screen.eyebrow')} · ${year}`}
        title={t('cal.screen.title')}
        sub={t('cal.screen.sub')}
        actions={
          <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
            <button type="button" style={secondaryBtn()} onClick={share}>{t('cal.screen.share')}</button>
            {canEdit && <button type="button" style={primaryBtn()} onClick={() => setAddOpen(true)}>{t('cal.screen.addEvent')}</button>}
          </div>
        }
      />

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: '24px' }}>
        <section style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', padding: '18px 22px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <button type="button" aria-label={t('cal.screen.prev')} onClick={prev} style={{ ...secondaryBtn(), height: 32, padding: '0 10px' }}>‹</button>
              <h2 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: '26px' }}>{monthLabel}</h2>
              <button type="button" aria-label={t('cal.screen.next')} onClick={next} style={{ ...secondaryBtn(), height: 32, padding: '0 10px' }}>›</button>
            </div>
            <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{workDays != null ? t('cal.screen.workingDays', { n: workDays }) : ''}</span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: '6px' }}>
            {['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'].map((d) => (
              <span key={d} style={{ fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', padding: '0 4px 4px' }}>{t(`cal.day.${d}`)}</span>
            ))}
            {cells.map((d, i) => {
              if (d == null) return <div key={i} />
              const jsWeekday = i % 7
              const off = isWeeklyOff(jsWeekday)
              const today = todayIsThisMonth && d === now.getDate()
              const ev = eventOn(d)
              return (
                <div key={i} style={{ height: '84px', borderRadius: '8px', padding: '8px', background: off ? 'var(--bg)' : 'var(--white)', border: `1px solid ${today ? 'var(--accent)' : 'var(--track)'}`, boxShadow: today ? '0 0 0 3px var(--accent-12)' : 'none', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                  <span style={{ fontFamily: 'var(--font-serif)', fontSize: '16px', color: off ? 'var(--muted)' : 'var(--ink)' }}>{d}</span>
                  {ev && (
                    <span style={{ fontSize: '10px', fontWeight: 600, padding: '3px 6px', borderRadius: '4px', background: (TONE[ev.kind] ?? TONE.event)[0], color: (TONE[ev.kind] ?? TONE.event)[1], whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{ev.title}</span>
                  )}
                </div>
              )
            })}
          </div>
        </section>

        <section style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '20px 24px 12px' }}>
            <div style={{ fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('cal.screen.comingUp')}</div>
            <div style={{ fontFamily: 'var(--font-serif)', fontSize: '22px' }}>{t('cal.screen.nextWeeks')}</div>
          </div>
          {coming.length === 0 && <div style={{ padding: '0 24px 16px', color: 'var(--muted)', fontSize: 13 }}>{t('cal.screen.none')}</div>}
          {coming.map((e) => (
            <div key={e.id} style={{ display: 'flex', gap: '12px', padding: '12px 24px', borderTop: '1px solid var(--track)', alignItems: 'flex-start' }}>
              <span style={{ width: '8px', height: '8px', marginTop: '6px', borderRadius: '4px', background: (TONE[e.kind] ?? TONE.event)[1], flexShrink: 0 }} />
              <div style={{ flexGrow: 1 }}>
                <b style={{ fontWeight: 500 }}>{e.title}</b>
                <div style={{ fontSize: '12px', color: 'var(--muted)' }}>{e.starts_on}{e.ends_on !== e.starts_on ? ` – ${e.ends_on}` : ''}</div>
              </div>
              {canEdit && <button type="button" onClick={() => { if (confirm(t('cal.events.confirmDelete'))) api.delete_calendar_event(e.id).then(load) }} style={{ border: 0, background: 'transparent', color: 'var(--muted)', fontSize: 12, cursor: 'pointer' }}>✕</button>}
            </div>
          ))}
          <div style={{ margin: 'auto 24px 20px', fontSize: '12px', color: 'var(--muted)', lineHeight: 1.5 }}>{t('cal.screen.note')}</div>
        </section>
      </div>

      {addOpen && <AddEventSheet onClose={() => setAddOpen(false)} onSaved={() => { setAddOpen(false); load() }} defaultDate={iso(year, month, 1)} />}
    </div>
  )
}

function AddEventSheet({ onClose, onSaved, defaultDate }: { onClose: () => void; onSaved: () => void; defaultDate: string }) {
  const [from, setFrom] = useState(defaultDate)
  const [to, setTo] = useState(defaultDate)
  const [kind, setKind] = useState<'holiday' | 'exam' | 'event'>('event')
  const [title, setTitle] = useState('')
  const [nonWorking, setNonWorking] = useState(false)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState(false)

  const save = () => {
    if (!title.trim()) return
    setBusy(true); setErr(false)
    const input: CalendarEventInput = { starts_on: from, ends_on: to, kind, title: title.trim(), is_non_working: nonWorking || kind === 'holiday' }
    api.add_calendar_event(input).then(onSaved).catch(() => { setErr(true); setBusy(false) })
  }
  const field = { height: 44, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, padding: '0 12px', boxSizing: 'border-box' as const, width: '100%' }

  return (
    <>
      <div style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 20 }} onClick={onClose} />
      <div style={{ position: 'fixed', top: 16, right: 16, bottom: 16, width: 440, background: 'var(--surface)', borderRadius: 20, zIndex: 21, display: 'flex', flexDirection: 'column', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }} role="dialog" aria-label={t('cal.screen.addEvent')}>
        <div style={{ padding: '22px 24px 16px', borderBottom: '1px solid var(--track)', fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('cal.screen.addEvent')}</div>
        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1 }}>
          <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('cal.events.titleLabel')}</label><input value={title} onChange={(e) => setTitle(e.target.value)} style={field} /></div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('cal.events.from')}</label><input type="date" value={from} onChange={(e) => setFrom(e.target.value)} style={field} /></div>
            <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('cal.events.to')}</label><input type="date" value={to} onChange={(e) => setTo(e.target.value)} style={field} /></div>
          </div>
          <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('cal.events.kind')}</label>
            <select value={kind} onChange={(e) => setKind(e.target.value as 'holiday' | 'exam' | 'event')} style={{ ...field, cursor: 'pointer' }}>
              <option value="event">{t('cal.events.kind.event')}</option>
              <option value="exam">{t('cal.events.kind.exam')}</option>
              <option value="holiday">{t('cal.events.kind.holiday')}</option>
            </select>
          </div>
          <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
            <input type="checkbox" checked={nonWorking} onChange={(e) => setNonWorking(e.target.checked)} />{t('cal.events.nonWorking')}
          </label>
          {err && <span style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{t('cal.events.error')}</span>}
        </div>
        <div style={{ background: 'var(--panel)', padding: '16px 24px', display: 'flex', gap: 10 }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('cal.events.cancel')}</button>
          <div style={{ flexGrow: 1 }} />
          <button type="button" style={primaryBtn()} onClick={save} disabled={busy || !title.trim()}>{t('cal.events.save')}</button>
        </div>
      </div>
    </>
  )
}
