// Report-card remarks entry (P16 Step 4). Marks & exams → Remarks. Pick an exam +
// class, enter one remark per student (with neutral template suggestions), then the
// Principal makes the cards final (locks the remarks). Built from the mock tokens.

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ClassDto, ExamDto, ReportRemarksDto } from '@/lib/api'

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function selectStyle(): CSSProperties {
  return { height: '40px', padding: '0 12px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', cursor: 'pointer' }
}

export default function ReportRemarksScreen() {
  const [exams, setExams] = useState<ExamDto[]>([])
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [examId, setExamId] = useState('')
  const [classId, setClassId] = useState('')
  const [data, setData] = useState<ReportRemarksDto | null>(null)
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const [savedId, setSavedId] = useState<string | null>(null)

  useEffect(() => {
    api.list_exams().then(setExams).catch(() => setExams([]))
    api.list_classes().then(setClasses).catch(() => setClasses([]))
  }, [])

  const load = useCallback(() => {
    if (!examId || !classId) { setData(null); return }
    api.get_report_remarks(examId, classId)
      .then((d) => { setData(d); setDrafts(Object.fromEntries(d.students.map((s) => [s.student_id, s.remark ?? '']))) })
      .catch(() => setData(null))
  }, [examId, classId])
  useEffect(load, [load])

  const save = (studentId: string) => {
    if (!data) return
    api.save_report_remark({ exam_id: examId, student_id: studentId, text: drafts[studentId] ?? '' })
      .then(() => { setSavedId(studentId); window.setTimeout(() => setSavedId(null), 1500) })
      .catch(() => {})
  }

  const finalize = () => {
    if (!examId || !confirm(t('remarks.finalizeConfirm'))) return
    api.finalize_report_cards(examId).then(load).catch(() => {})
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('remarks.eyebrow')}
        title={t('remarks.title')}
        sub={t('remarks.sub')}
        actions={
          <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
            <select aria-label={t('remarks.exam')} value={examId} style={selectStyle()} onChange={(e) => setExamId(e.target.value)}>
              <option value="">{t('remarks.pickExam')}</option>
              {exams.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
            </select>
            <select aria-label={t('remarks.class')} value={classId} style={selectStyle()} onChange={(e) => setClassId(e.target.value)}>
              <option value="">{t('remarks.pickClass')}</option>
              {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
            </select>
            {data && examId && classId && (
              <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/reportcards/${classId}?exam=${examId}&auto=1`)}>{t('remarks.print')}</button>
            )}
            {data && !data.locked && (
              <button type="button" style={primaryBtn()} onClick={finalize}>{t('remarks.finalize')}</button>
            )}
          </div>
        }
      />

      {!data && <div style={{ padding: '24px', background: 'var(--panel)', borderRadius: '12px', color: 'var(--muted)' }}>{t('remarks.empty')}</div>}

      {data?.locked && (
        <div style={{ padding: '12px 16px', borderRadius: '8px', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', color: 'var(--gold-text)', fontSize: 13 }}>{t('remarks.locked')}</div>
      )}

      {data && (
        <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
          {data.students.map((s, i) => (
            <div key={s.student_id} style={{ display: 'grid', gridTemplateColumns: '200px 1fr 150px auto', gap: 12, alignItems: 'center', padding: '12px 20px', borderTop: i === 0 ? 'none' : '1px solid var(--track)' }}>
              <div>
                <b style={{ fontWeight: 500 }}>{s.name}</b>
                {s.roll_no != null && <span style={{ fontSize: 12, color: 'var(--muted)' }}> · {s.roll_no}</span>}
              </div>
              <input
                value={drafts[s.student_id] ?? ''}
                onChange={(e) => setDrafts({ ...drafts, [s.student_id]: e.target.value })}
                disabled={data.locked}
                placeholder={t('remarks.remark')}
                style={{ height: 40, borderRadius: 6, border: '1px solid var(--line-strong)', background: data.locked ? 'var(--panel)' : 'var(--white)', color: 'var(--ink)', fontSize: 14, padding: '0 12px' }}
              />
              <select
                aria-label={t('remarks.suggestion')}
                value=""
                disabled={data.locked}
                onChange={(e) => { if (e.target.value) setDrafts({ ...drafts, [s.student_id]: e.target.value }) }}
                style={selectStyle()}
              >
                <option value="">{t('remarks.suggestion')}</option>
                {data.templates.map((tm) => <option key={tm.key} value={tm.text}>{tm.text.slice(0, 40)}</option>)}
              </select>
              <button type="button" disabled={data.locked} style={secondaryBtn()} onClick={() => save(s.student_id)}>
                {savedId === s.student_id ? t('remarks.saved') : t('remarks.save')}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
