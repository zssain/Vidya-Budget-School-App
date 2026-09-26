// Teacher "My timetable" (P16 Step 1). Phone screen (390×844): the teacher's own
// weekly periods, today first, each row showing the period time, subject and
// class. Reached from the teacher home. Built from the Attendance phone pattern
// (navy header + light scrollable body).

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { TeacherTimetableDto, TimetableSlotDto, ClassDto } from '@/lib/api'
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
  const [dutyOpen, setDutyOpen] = useState(false)
  const [dutySent, setDutySent] = useState(false)
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

      <div style={{ flexShrink: 0, background: '#FDFDFB', borderTop: '1px solid #D5DDE0', padding: '12px 16px 16px' }}>
        <button type="button" onClick={() => setDutyOpen(true)} style={{ width: '100%', height: 52, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 15, fontWeight: 500 }}>
          {t('tt.duty.request')}
        </button>
      </div>

      {dutyOpen && <DutyForm onClose={() => setDutyOpen(false)} onSent={() => { setDutyOpen(false); setDutySent(true) }} />}
      {dutySent && (
        <div role="status" style={{ position: 'absolute', left: 16, right: 16, bottom: 90, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 14px', fontSize: 14, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Icon name="check" size={16} strokeWidth={2.2} color="var(--gold)" />{t('tt.duty.sent')}
        </div>
      )}
    </div>
  )
}

function DutyForm({ onClose, onSent }: { onClose: () => void; onSent: () => void }) {
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [classId, setClassId] = useState('')
  const today = new Date().toISOString().slice(0, 10)
  const [from, setFrom] = useState(today)
  const [to, setTo] = useState(today)
  const [reason, setReason] = useState('')
  const [busy, setBusy] = useState(false)

  useEffect(() => { api.list_classes().then(setClasses).catch(() => setClasses([])) }, [])

  const send = () => {
    if (!classId || reason.trim().length < 5) return
    setBusy(true)
    api.create_request({
      kind: 'attendance_duty',
      target_table: 'class',
      target_id: classId,
      base_version: 0,
      reason: reason.trim(),
      before_json: '{}',
      after_json: JSON.stringify({ class_id: classId, from_date: from, to_date: to }),
    }).then(onSent).catch(() => setBusy(false))
  }

  const inputStyle = { height: 46, borderRadius: 8, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 15, padding: '0 12px', boxSizing: 'border-box' as const, width: '100%' }

  return (
    <>
      <div style={{ position: 'absolute', inset: 0, background: 'rgba(11,26,51,0.45)' }} onClick={onClose} />
      <div style={{ position: 'absolute', left: 0, right: 0, bottom: 0, background: 'var(--white)', borderRadius: '22px 22px 0 0', padding: '16px 18px 24px', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <span style={{ alignSelf: 'center', width: 40, height: 4, borderRadius: 2, background: 'var(--line)' }} />
        <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('tt.duty.title')}</div>
        <label style={{ fontSize: 13, fontWeight: 500 }}>{t('tt.duty.class')}</label>
        <select value={classId} onChange={(e) => setClassId(e.target.value)} style={inputStyle}>
          <option value="">{t('tt.duty.pickClass')}</option>
          {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
        </select>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <div>
            <label style={{ fontSize: 13, fontWeight: 500 }}>{t('tt.duty.from')}</label>
            <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} style={inputStyle} />
          </div>
          <div>
            <label style={{ fontSize: 13, fontWeight: 500 }}>{t('tt.duty.to')}</label>
            <input type="date" value={to} onChange={(e) => setTo(e.target.value)} style={inputStyle} />
          </div>
        </div>
        <label style={{ fontSize: 13, fontWeight: 500 }}>{t('tt.duty.reason')}</label>
        <input value={reason} onChange={(e) => setReason(e.target.value)} style={inputStyle} />
        <div style={{ display: 'flex', gap: 10, marginTop: 4 }}>
          <button type="button" onClick={onClose} style={{ flex: 1, height: 50, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 15, fontWeight: 500 }}>{t('tt.duty.cancel')}</button>
          <button type="button" onClick={send} disabled={busy || !classId || reason.trim().length < 5} style={{ flex: 2, height: 50, borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 15, fontWeight: 500, opacity: busy || !classId || reason.trim().length < 5 ? 0.5 : 1 }}>{t('tt.duty.send')}</button>
        </div>
      </div>
    </>
  )
}
