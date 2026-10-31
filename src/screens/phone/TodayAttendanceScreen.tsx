// Attendance today (repair T1): the Attendance tile's list when the teacher has
// more than one class to mark today — their own classes, then any they cover today
// (substitution / attendance duty), each with today's status and roster size, from
// dashboard_teacher. Tap → the class's attendance sheet.
import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { TeacherClassDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import PhoneScreen, { PHONE_CARD, PHONE_ROW, PhoneEmpty, PhonePill } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { navigate } from '@/lib/router'

export default function TodayAttendanceScreen() {
  useLang()
  const [classes, setClasses] = useState<TeacherClassDto[] | null>(null)
  useEffect(() => {
    api.dashboard_teacher().then((d) => setClasses(d.classes)).catch(() => setClasses([]))
  }, [])

  return (
    <PhoneScreen title={t('tp.att.title')}>
      {classes === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : classes.length === 0 ? (
        <PhoneEmpty>{t('tp.att.none')}</PhoneEmpty>
      ) : (
        <div style={PHONE_CARD}>
          {classes.map((c, i) => (
            <button
              key={c.class_id}
              type="button"
              onClick={() => navigate(`/teacher/attendance/${c.class_id}`)}
              style={{ ...PHONE_ROW, width: '100%', textAlign: 'left', border: 0, borderTop: i === 0 ? 0 : PHONE_ROW.borderTop, background: 'transparent', color: 'inherit' }}
            >
              <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0 }}>
                <b style={{ fontWeight: 600, fontSize: 15 }}>{c.display}</b>
                <span style={{ fontSize: 12, color: 'var(--muted)' }}>
                  {c.duty ? `${t('tp.att.covering')} · ` : ''}
                  {t('tp.att.students', { n: c.student_count })}
                </span>
              </span>
              <PhonePill tone={c.status === 'submitted' ? 'ok' : c.status === 'draft' ? 'warn' : 'muted'}>{t(`tp.status.${c.status}`)}</PhonePill>
              <Icon name="chevronRight" size={18} strokeWidth={1.6} />
            </button>
          ))}
        </div>
      )}
    </PhoneScreen>
  )
}
