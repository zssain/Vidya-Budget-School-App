// Students (repair T1): a read-only list of the students in the teacher's classes
// (class-teacher and subject classes). list_students is teacher-scoped natively
// (commands::teacher_students) and, on the iPhone app, served from the records the
// school PC exported to this device (the same scope). Grouped by class, searchable.
import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { StudentDto } from '@/lib/api'
import PhoneScreen, { PHONE_CARD, PHONE_FIELD, PhoneEmpty } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { groupStudents } from '@/lib/teacherPhone'

export default function PhoneStudentsScreen() {
  useLang()
  const [rows, setRows] = useState<StudentDto[] | null>(null)
  const [q, setQ] = useState('')
  useEffect(() => {
    api.list_students().then(setRows).catch(() => setRows([]))
  }, [])
  const groups = groupStudents(rows ?? [], q)

  return (
    <PhoneScreen title={t('tp.stu.title')} sub={rows ? t('tp.stu.count', { n: rows.length }) : undefined}>
      {rows === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : rows.length === 0 ? (
        <PhoneEmpty>{t('tp.stu.none')}</PhoneEmpty>
      ) : (
        <>
          <input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={t('tp.stu.search')} aria-label={t('tp.stu.search')} style={PHONE_FIELD} />
          {groups.map((g) => (
            <section key={g.class_display ?? ''} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>
                {g.class_display ?? t('tp.stu.noClass')} · {t('tp.stu.count', { n: g.students.length })}
              </span>
              <div style={PHONE_CARD}>
                {g.students.map((s, i) => (
                  <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 16px', borderTop: i === 0 ? 0 : '1px solid var(--track)' }}>
                    <span style={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 2 }}>
                      <span style={{ fontSize: 15, fontWeight: 500 }}>{s.name}</span>
                      {s.admission_no || s.provisional_no ? <span style={{ fontSize: 12, color: 'var(--muted)' }}>{s.admission_no ?? s.provisional_no}</span> : null}
                    </span>
                    {s.roll_no != null ? <span style={{ fontSize: 12, color: 'var(--muted)', flexShrink: 0 }}>{t('tp.stu.roll', { n: s.roll_no })}</span> : null}
                  </div>
                ))}
              </div>
            </section>
          ))}
        </>
      )}
    </PhoneScreen>
  )
}
