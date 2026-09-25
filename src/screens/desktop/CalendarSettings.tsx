// School calendar management (P13): weekly off days + holidays/events, shown in
// Settings → Session & terms. Derived screen (the prototype's month-view Calendar
// is P16): built from the app's card/table patterns and tokens. Principal only —
// the commands re-check the permission and audit every change.

import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CalendarDto, CalendarEventDto, CalendarEventInput, CmdError } from '@/lib/api'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 14px' }
const DAY_KEYS = ['cal.day.mon', 'cal.day.tue', 'cal.day.wed', 'cal.day.thu', 'cal.day.fri', 'cal.day.sat', 'cal.day.sun'] as const

const emptyInput = (): CalendarEventInput => ({
  starts_on: '',
  ends_on: '',
  kind: 'holiday',
  title: '',
  is_non_working: true,
})

export default function CalendarSettings() {
  const [cal, setCal] = useState<CalendarDto | null>(null)
  const [week, setWeek] = useState<boolean[]>([true, true, true, true, true, true, false])
  const [weekSaved, setWeekSaved] = useState(false)
  const [editing, setEditing] = useState<{ id: string | null; input: CalendarEventInput } | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(() => {
    api.get_calendar()
      .then((c) => {
        setCal(c)
        setWeek(c.week)
      })
      .catch(() => setCal({ week, events: [] }))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const toggleDay = (i: number) => {
    setWeek((w) => w.map((v, j) => (j === i ? !v : v)))
    setWeekSaved(false)
  }

  const saveWeek = async () => {
    try {
      await api.set_weekly_offs(week)
      setWeekSaved(true)
    } catch (e) {
      void (e as CmdError)
    }
  }

  const saveEvent = async () => {
    if (!editing) return
    setError(null)
    try {
      if (editing.id) await api.update_calendar_event(editing.id, editing.input)
      else await api.add_calendar_event(editing.input)
      setEditing(null)
      load()
    } catch (e) {
      void (e as CmdError)
      setError(t('cal.events.error'))
    }
  }

  const removeEvent = async (id: string) => {
    // eslint-disable-next-line no-alert
    if (!window.confirm(t('cal.events.confirmDelete'))) return
    try {
      await api.delete_calendar_event(id)
      load()
    } catch (e) {
      void (e as CmdError)
    }
  }

  const set = (patch: Partial<CalendarEventInput>) =>
    setEditing((e) => (e ? { ...e, input: { ...e.input, ...patch } } : e))

  const events: CalendarEventDto[] = cal?.events ?? []

  return (
    <>
      {/* Weekly off days */}
      <div style={CARD}>
        <h3 style={H3}>{t('cal.weekly.title')}</h3>
        <p style={{ fontSize: 13, color: 'var(--muted)', margin: '0 0 14px' }}>{t('cal.weekly.hint')}</p>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {DAY_KEYS.map((k, i) => (
            <button
              key={k}
              type="button"
              role="switch"
              aria-checked={week[i]}
              aria-label={t(k)}
              onClick={() => toggleDay(i)}
              style={{
                minWidth: 64,
                padding: '10px 8px',
                borderRadius: 10,
                cursor: 'pointer',
                fontSize: 13,
                fontWeight: 500,
                border: `1px solid ${week[i] ? 'var(--accent)' : 'var(--line-strong)'}`,
                background: week[i] ? 'var(--accent)' : 'var(--white)',
                color: week[i] ? 'var(--white)' : 'var(--muted)',
                display: 'flex',
                flexDirection: 'column',
                gap: 2,
                alignItems: 'center',
              }}
            >
              <span>{t(k)}</span>
              <span style={{ fontSize: 10, opacity: 0.85 }}>{week[i] ? t('cal.weekly.working') : t('cal.weekly.off')}</span>
            </button>
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginTop: 16 }}>
          <button
            type="button"
            onClick={saveWeek}
            style={{ height: 40, padding: '0 18px', borderRadius: 8, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 14, fontWeight: 500, cursor: 'pointer' }}
          >
            {t('cal.weekly.save')}
          </button>
          {weekSaved ? <span style={{ fontSize: 13, color: 'var(--accent)' }}>{t('cal.weekly.saved')}</span> : null}
        </div>
      </div>

      {/* Holidays & events */}
      <div style={CARD}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 14 }}>
          <h3 style={{ ...H3, margin: 0 }}>{t('cal.events.title')}</h3>
          <button
            type="button"
            onClick={() => { setError(null); setEditing({ id: null, input: emptyInput() }) }}
            style={{ height: 34, padding: '0 14px', borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}
          >
            + {t('cal.events.add')}
          </button>
        </div>
        {events.length === 0 ? (
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0 }}>{t('cal.events.empty')}</p>
        ) : (
          events.map((ev) => (
            <div key={ev.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 0', borderTop: '1px solid var(--track)' }}>
              <div style={{ flexGrow: 1 }}>
                <div style={{ fontSize: 14, fontWeight: 500 }}>{ev.title}</div>
                <div style={{ fontSize: 12, color: 'var(--muted)' }}>
                  {t(`cal.events.kind.${ev.kind}`)} · {ev.starts_on}
                  {ev.ends_on !== ev.starts_on ? ` – ${ev.ends_on}` : ''}
                  {ev.is_non_working ? ` · ${t('cal.events.nonWorking')}` : ''}
                </div>
              </div>
              <button type="button" onClick={() => { setError(null); setEditing({ id: ev.id, input: { starts_on: ev.starts_on, ends_on: ev.ends_on, kind: ev.kind, title: ev.title, title_hi: ev.title_hi, title_te: ev.title_te, is_non_working: ev.is_non_working } }) }} style={linkBtn}>{t('cal.events.edit')}</button>
              <button type="button" onClick={() => removeEvent(ev.id)} style={{ ...linkBtn, color: 'var(--danger)' }}>{t('cal.events.delete')}</button>
            </div>
          ))
        )}
      </div>

      {editing ? (
        <div onClick={() => setEditing(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: 440, background: 'var(--surface)', borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 22 }}>{t('cal.events.title')}</div>
            <label style={fieldLabel}>{t('cal.events.titleLabel')}
              <input value={editing.input.title} onChange={(e) => set({ title: e.target.value })} style={inputStyle} />
            </label>
            <div style={{ display: 'flex', gap: 12 }}>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('cal.events.from')}
                <input type="date" value={editing.input.starts_on} onChange={(e) => set({ starts_on: e.target.value, ends_on: editing.input.ends_on || e.target.value })} style={inputStyle} />
              </label>
              <label style={{ ...fieldLabel, flex: 1 }}>{t('cal.events.to')}
                <input type="date" value={editing.input.ends_on} onChange={(e) => set({ ends_on: e.target.value })} style={inputStyle} />
              </label>
            </div>
            <label style={fieldLabel}>{t('cal.events.kind')}
              <select value={editing.input.kind} onChange={(e) => set({ kind: e.target.value })} style={inputStyle}>
                <option value="holiday">{t('cal.events.kind.holiday')}</option>
                <option value="exam">{t('cal.events.kind.exam')}</option>
                <option value="event">{t('cal.events.kind.event')}</option>
              </select>
            </label>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
              <input type="checkbox" checked={editing.input.is_non_working} onChange={(e) => set({ is_non_working: e.target.checked })} />
              {t('cal.events.nonWorking')}
            </label>
            {error ? <div style={{ fontSize: 12, color: 'var(--danger)' }}>{error}</div> : null}
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 4 }}>
              <button type="button" onClick={() => setEditing(null)} style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, cursor: 'pointer' }}>{t('cal.events.cancel')}</button>
              <button type="button" onClick={saveEvent} style={{ height: 40, padding: '0 18px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }}>{t('cal.events.save')}</button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  )
}

const linkBtn: React.CSSProperties = { background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 13, fontWeight: 500, cursor: 'pointer' }
const fieldLabel: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13, color: 'var(--muted)' }
const inputStyle: React.CSSProperties = { height: 38, padding: '0 12px', borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14 }
