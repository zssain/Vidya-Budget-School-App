// Report cards (repair T1): the class teacher's report-card remarks on the phone
// (the desktop's ReportRemarksScreen, simplified). Pick an exam × own class (from
// list_exams + dashboard_teacher.class_teacher_of), then one remark per student with
// the school's neutral suggestions; Save → save_report_remark. Locked when the
// Principal has made the cards final.
import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, ReportRemarksDto } from '@/lib/api'
import PhoneScreen, { PHONE_CARD, PHONE_FIELD, PhoneEmpty } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { remarkOptions, type RemarkOption } from '@/lib/teacherPhone'

export default function PhoneRemarksScreen() {
  useLang()
  const [opts, setOpts] = useState<RemarkOption[] | null>(null)
  const [isCT, setIsCT] = useState(true)
  const [sel, setSel] = useState('') // "examId|classId"
  const [data, setData] = useState<ReportRemarksDto | null>(null)
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const [savedId, setSavedId] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    Promise.all([api.dashboard_teacher(), api.list_exams()])
      .then(([d, exams]) => {
        const names = Object.fromEntries(d.classes.map((c) => [c.class_id, c.display]))
        setIsCT(d.class_teacher_of.length > 0)
        const o = remarkOptions(exams, d.class_teacher_of, names)
        setOpts(o)
        if (o.length > 0) setSel((cur) => cur || `${o[0].exam_id}|${o[0].class_id}`)
      })
      .catch(() => setOpts([]))
  }, [])

  const [examId, classId] = sel.split('|')
  const load = useCallback(() => {
    if (!examId || !classId) return
    setErr(null)
    api
      .get_report_remarks(examId, classId)
      .then((d) => {
        setData(d)
        setDrafts(Object.fromEntries(d.students.map((s) => [s.student_id, s.remark ?? ''])))
      })
      .catch((e) => {
        setData(null)
        setErr(t((e as CmdError).message_key))
      })
  }, [examId, classId])
  useEffect(load, [load])

  const save = (studentId: string) => {
    setErr(null)
    api
      .save_report_remark({ exam_id: examId, student_id: studentId, text: drafts[studentId] ?? '' })
      .then(() => {
        setSavedId(studentId)
        window.setTimeout(() => setSavedId(null), 1500)
      })
      .catch((e) => {
        const ce = e as CmdError
        setErr(((ce.vars as { rule?: string } | null)?.rule === 'too_long') ? t('tp.rem.tooLong') : t(ce.message_key))
      })
  }

  return (
    <PhoneScreen title={t('tp.rem.title')}>
      {opts === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : !isCT ? (
        <PhoneEmpty>{t('tp.rem.notClassTeacher')}</PhoneEmpty>
      ) : opts.length === 0 ? (
        <PhoneEmpty>{t('tp.rem.noExams')}</PhoneEmpty>
      ) : (
        <>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>
              {t('tp.rem.exam')} · {t('tp.rem.class')}
            </span>
            <select value={sel} onChange={(e) => setSel(e.target.value)} style={PHONE_FIELD}>
              {opts.map((o) => (
                <option key={`${o.exam_id}|${o.class_id}`} value={`${o.exam_id}|${o.class_id}`}>
                  {o.exam_name} · {o.class_display}
                </option>
              ))}
            </select>
          </label>
          {err ? <span role="alert" style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span> : null}
          {data?.locked ? (
            <div style={{ padding: '12px 14px', borderRadius: 12, background: 'var(--unmarked)', border: '1px solid var(--gold-line)', color: 'var(--gold-text)', fontSize: 13 }}>{t('tp.rem.locked')}</div>
          ) : null}
          {data ? (
            <div style={PHONE_CARD}>
              {data.students.map((s, i) => (
                <div key={s.student_id} style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: '14px 16px', borderTop: i === 0 ? 0 : '1px solid var(--track)' }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 6 }}>
                    <b style={{ fontWeight: 600, fontSize: 15 }}>{s.name}</b>
                    {s.roll_no != null ? <span style={{ fontSize: 12, color: 'var(--muted)' }}>· {s.roll_no}</span> : null}
                  </div>
                  <textarea
                    aria-label={s.name}
                    rows={2}
                    maxLength={300}
                    value={drafts[s.student_id] ?? ''}
                    disabled={data.locked}
                    placeholder={t('tp.rem.placeholder')}
                    onChange={(e) => setDrafts({ ...drafts, [s.student_id]: e.target.value })}
                    style={{ ...PHONE_FIELD, resize: 'vertical', fontFamily: 'inherit', background: data.locked ? 'var(--panel)' : 'var(--white)' }}
                  />
                  {!data.locked ? (
                    <div style={{ display: 'flex', gap: 8 }}>
                      {data.templates.length > 0 ? (
                        <select
                          aria-label={t('tp.rem.suggest')}
                          value=""
                          onChange={(e) => {
                            if (e.target.value) setDrafts({ ...drafts, [s.student_id]: e.target.value })
                          }}
                          style={{ ...PHONE_FIELD, padding: '8px 10px', fontSize: 13, flexGrow: 1, minWidth: 0 }}
                        >
                          <option value="">{t('tp.rem.suggest')}</option>
                          {data.templates.map((tm) => (
                            <option key={tm.key} value={tm.text}>{tm.text.slice(0, 48)}</option>
                          ))}
                        </select>
                      ) : null}
                      <button
                        type="button"
                        onClick={() => save(s.student_id)}
                        style={{ height: 40, padding: '0 16px', borderRadius: 6, border: '1px solid var(--accent)', background: savedId === s.student_id ? 'var(--surface)' : 'var(--accent)', color: savedId === s.student_id ? 'var(--accent)' : 'var(--white)', fontSize: 14, fontWeight: 600, flexShrink: 0, marginLeft: 'auto' }}
                      >
                        {savedId === s.student_id ? t('tp.rem.saved') : t('tp.rem.save')}
                      </button>
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          ) : null}
        </>
      )}
    </PhoneScreen>
  )
}
