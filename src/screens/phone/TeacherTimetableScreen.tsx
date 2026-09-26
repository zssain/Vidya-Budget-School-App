// Teacher "My timetable" (P16 Step 1). Phone screen (390×844): the teacher's own
// weekly periods, today first, each row showing the period time, subject and
// class. Reached from the teacher home. Built from the Attendance phone pattern
// (navy header + light scrollable body).

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { TeacherTimetableDto, TimetableSlotDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { navigate } from '@/lib/router'
import { t, useLang } from '@/lib/i18n'

function todayIso(): number {
  const d = new Date().getDay()
  return d === 0 ? 7 : d
}

export default function TeacherTimetableScreen() {
  useLang()
  const [data, setData] = useState<TeacherTimetableDto | null>(null)
  const today = todayIso()

  useEffect(() => {
    api.my_timetable().then(setData).catch(() => setData({ periods: [], slots: [] }))
  }, [])

  const timeFor = (period: number) => data?.periods.find((p) => p.no === period)?.starts_at ?? ''
  // Weekdays ordered with today first, then the rest of the week (Mon..Sat).
  const order = [today, ...[1, 2, 3, 4, 5, 6].filter((d) => d !== today)].filter((d) => d >= 1 && d <= 6)

  const dayRows = (wd: number): TimetableSlotDto[] =>
    (data?.slots ?? []).filter((s) => s.weekday === wd).sort((a, b) => a.period_no - b.period_no)

  return (
    <div style={{ position: 'relative', width: '390px', height: '844px', display: 'flex', flexDirection: 'column', background: '#F5F7F6', color: '#13233F', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", fontSize: 14, overflow: 'hidden' }}>
      <header style={{ flexShrink: 0, background: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)', color: '#FFFFFF', padding: '10px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button type="button" aria-label={t('tt.mine.back')} onClick={() => navigate('/teacher/home')} style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#FFFFFF', border: 0, background: 'transparent' }}>
          <Icon name="back" size={22} strokeWidth={1.7} />
        </button>
        <h1 style={{ margin: 0, fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{t('tt.mine.title')}</h1>
      </header>

      <div style={{ flexGrow: 1, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
        {data && data.slots.length === 0 && (
          <span style={{ fontSize: 15, color: 'var(--muted)', textAlign: 'center', marginTop: 40 }}>{t('tt.mine.none')}</span>
        )}
        {order.map((wd) => {
          const rows = dayRows(wd)
          const isToday = wd === today
          return (
            <section key={wd} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 13, color: isToday ? 'var(--accent)' : 'var(--ink)' }}>{t(`tt.d${wd}`)}</span>
                {isToday && <span style={{ fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 10, background: 'var(--accent-10)', color: 'var(--accent)' }}>{t('tt.mine.today')}</span>}
              </div>
              {rows.length === 0 ? (
                <span style={{ fontSize: 13, color: 'var(--muted)', padding: '4px 2px' }}>{isToday ? t('tt.mine.noneToday') : '—'}</span>
              ) : (
                <div style={{ borderRadius: 14, background: 'var(--surface)', border: '1px solid var(--line)', overflow: 'hidden' }}>
                  {rows.map((s, i) => (
                    <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 14px', borderTop: i === 0 ? 'none' : '1px solid var(--track)' }}>
                      <div style={{ width: 52, flexShrink: 0 }}>
                        <b style={{ fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 18 }}>{s.period_no}</b>
                        <span style={{ display: 'block', fontSize: 11, color: 'var(--muted)' }}>{timeFor(s.period_no)}</span>
                      </div>
                      <div style={{ flexGrow: 1, minWidth: 0 }}>
                        <b style={{ fontWeight: 600, fontSize: 15 }}>{s.subject_name}</b>
                        <span style={{ display: 'block', fontSize: 12, color: 'var(--muted)' }}>{s.class_display}</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          )
        })}
      </div>
    </div>
  )
}
