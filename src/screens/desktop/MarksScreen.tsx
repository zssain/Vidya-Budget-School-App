import { useCallback, useEffect, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import type { PillVariant } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ClassSubjectDto, ExamDto, ExamSubjectDto, MarksRowDto } from '@/lib/api'

// Marks & reports (prompts/P07 §6): exams list, class×subject status grid, marks
// entry grid (Tab/Enter down, AB toggle, 0–max inline, Save draft / Submit
// subject — locks that subject only). Report-card links open the print route.

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '38px', padding: '0 14px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px', boxSizing: 'border-box' }
}
function statusVariant(s: string): PillVariant {
  return s === 'submitted' ? 'paid' : s === 'draft' ? 'partpaid' : 'neutral'
}

export default function MarksScreen() {
  const [exams, setExams] = useState<ExamDto[]>([])
  const [entry, setEntry] = useState<ExamSubjectDto | null>(null)
  const [newExam, setNewExam] = useState(false)
  const [toast, setToast] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_exams().then(setExams).catch(() => setExams([]))
  }, [])
  useEffect(load, [load])

  const flash = (m: string) => {
    setToast(m)
    window.setTimeout(() => setToast(null), 2600)
  }

  if (entry) {
    return <MarksEntry examSubject={entry} onBack={() => { setEntry(null); load() }} onFlash={flash} />
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('marks.eyebrow')}
        title={t('marks.title')}
        sub={t('marks.sub')}
        actions={
          <>
            <button type="button" style={secondaryBtn()} onClick={() => navigate('/principal/grade-scale')}>{t('grade.title').replace('.', '')}</button>
            <button type="button" style={primaryBtn()} onClick={() => setNewExam(true)}>{t('marks.newExam')}</button>
          </>
        }
      />

      {exams.length === 0 ? (
        <div style={{ padding: '40px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('marks.noExams')}</div>
      ) : (
        exams.map((ex) => (
          <div key={ex.id} style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
            <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--track)', display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
              <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '22px' }}>{ex.name}</span>
              <span style={{ fontSize: '12px', color: 'var(--muted)' }}>{ex.term_name ?? ''}{ex.starts_on ? ` · ${ex.starts_on}` : ''}</span>
            </div>
            {ex.subjects.length === 0 ? (
              <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: '13px' }}>—</div>
            ) : (
              ex.subjects.map((s, i) => (
                <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: '16px', padding: '12px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none' }}>
                  <span style={{ flex: 1, fontWeight: 500 }}>{s.class_display} · {s.subject_name} <span style={{ color: 'var(--muted)', fontWeight: 400 }}>({t('marks.maxMarks', { max: s.max_marks })})</span></span>
                  <Pill variant={statusVariant(s.status)}>{t(`marks.status.${s.status}`)}</Pill>
                  <button type="button" style={secondaryBtn()} onClick={() => setEntry(s)}>{t('marks.enter')}</button>
                  <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/reportcards/${s.class_id}?exam=${ex.id}`)}>
                    {t('marks.reportCards')}
                  </button>
                </div>
              ))
            )}
          </div>
        ))
      )}

      {newExam ? <NewExamDialog onClose={() => setNewExam(false)} onCreated={() => { setNewExam(false); load(); flash(t('marks.saved')) }} /> : null}
      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}

function MarksEntry({ examSubject, onBack, onFlash }: { examSubject: ExamSubjectDto; onBack: () => void; onFlash: (m: string) => void }) {
  const max = examSubject.max_marks
  const [rows, setRows] = useState<MarksRowDto[]>([])
  const [status, setStatus] = useState('not_started')
  const inputs = useRef<(HTMLInputElement | null)[]>([])

  const load = useCallback(() => {
    api.get_marks_sheet(examSubject.id).then((s) => { setRows(s.rows); setStatus(s.status) }).catch(() => setRows([]))
  }, [examSubject.id])
  useEffect(load, [load])

  const locked = status === 'submitted'

  const setMark = (id: string, marks: number | null, absent: boolean) => {
    setRows((prev) => prev.map((r) => (r.student_id === id ? { ...r, marks, absent } : r)))
  }
  const entries = () => rows.map((r) => ({ student_id: r.student_id, marks: r.absent ? null : r.marks, absent: r.absent }))

  const invalid = (r: MarksRowDto) => !r.absent && r.marks != null && (r.marks < 0 || r.marks > max)
  const anyInvalid = rows.some(invalid)

  const save = async (submit: boolean) => {
    if (anyInvalid) return
    try {
      if (submit) await api.submit_marks(examSubject.id, entries())
      else await api.save_marks_draft(examSubject.id, entries())
      onFlash(t('marks.saved'))
      load()
    } catch (e) {
      onFlash(t((e as api.CmdError).message_key, (e as api.CmdError).vars as Record<string, string | number>))
    }
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <button type="button" onClick={onBack} style={{ alignSelf: 'flex-start', border: 'none', background: 'transparent', color: 'var(--muted)', fontSize: '13px', cursor: 'pointer' }}>← {t('marks.back')}</button>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
        <h1 style={{ margin: 0, fontFamily: "'Newsreader', Georgia, serif", fontWeight: 400, fontSize: '34px', letterSpacing: '-0.02em' }}>{examSubject.class_display} · {examSubject.subject_name}</h1>
        {locked ? <Pill variant="paid">{t('marks.submitted')}</Pill> : null}
      </div>

      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        <div style={{ display: 'grid', gridTemplateColumns: '64px 1fr 140px 80px', gap: '12px', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
          <span>{t('marks.col.roll')}</span>
          <span>{t('marks.col.student')}</span>
          <span>{t('marks.col.marks')} / {max}</span>
          <span>{t('marks.ab')}</span>
        </div>
        {rows.map((r, i) => (
          <div key={r.student_id} style={{ display: 'grid', gridTemplateColumns: '64px 1fr 140px 80px', gap: '12px', alignItems: 'center', padding: '8px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px' }}>
            <span style={{ color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{r.roll_no ?? ''}</span>
            <span>{r.name}</span>
            <span>
              <input
                ref={(el) => { inputs.current[i] = el }}
                inputMode="numeric"
                disabled={locked || r.absent}
                value={r.absent ? '' : r.marks == null ? '' : String(r.marks)}
                onChange={(e) => setMark(r.student_id, e.target.value === '' ? null : Number(e.target.value.replace(/\D/g, '')), false)}
                onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); inputs.current[i + 1]?.focus() } }}
                aria-label={r.name}
                style={{ width: '110px', height: '38px', borderRadius: '6px', border: `1px solid ${invalid(r) ? 'var(--danger)' : 'var(--line-strong)'}`, background: locked || r.absent ? 'var(--panel)' : 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 10px', fontVariantNumeric: 'tabular-nums' }}
              />
              {invalid(r) ? <span style={{ marginLeft: '8px', fontSize: '12px', color: 'var(--danger)' }}>{t('marks.rangeErr', { max })}</span> : null}
            </span>
            <button
              type="button"
              disabled={locked}
              aria-pressed={r.absent}
              onClick={() => setMark(r.student_id, null, !r.absent)}
              style={{ width: '48px', height: '34px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: r.absent ? 'var(--gold)' : 'var(--white)', color: r.absent ? 'var(--navy)' : 'var(--muted)', fontWeight: 600, cursor: locked ? 'default' : 'pointer' }}
            >
              {t('marks.ab')}
            </button>
          </div>
        ))}
      </div>

      {!locked ? (
        <div style={{ display: 'flex', gap: '12px' }}>
          <button type="button" onClick={() => save(false)} disabled={anyInvalid} style={{ ...secondaryBtn(), height: '46px', padding: '0 20px', opacity: anyInvalid ? 0.5 : 1 }}>{t('marks.saveDraft')}</button>
          <button type="button" onClick={() => save(true)} disabled={anyInvalid} style={{ ...primaryBtn(), height: '46px', padding: '0 20px', opacity: anyInvalid ? 0.5 : 1 }}>{t('marks.submit')}</button>
        </div>
      ) : null}
    </div>
  )
}

function NewExamDialog({ onClose, onCreated }: { onClose: () => void; onCreated: () => void }) {
  const [name, setName] = useState('')
  const [starts, setStarts] = useState('')
  const [ends, setEnds] = useState('')
  const [subjects, setSubjects] = useState<ClassSubjectDto[]>([])
  const [picked, setPicked] = useState<Record<string, string>>({}) // class_subject_id → max marks
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    api.list_class_subjects().then(setSubjects).catch(() => setSubjects([]))
  }, [])

  const toggle = (id: string) => {
    setPicked((prev) => {
      const next = { ...prev }
      if (id in next) delete next[id]
      else next[id] = '100'
      return next
    })
  }

  const create = async () => {
    setBusy(true)
    try {
      await api.create_exam({
        name: name.trim(),
        starts_on: starts || null,
        ends_on: ends || null,
        subjects: Object.entries(picked).map(([class_subject_id, m]) => ({ class_subject_id, max_marks: Number(m) || 100 })),
      })
      onCreated()
    } catch {
      setBusy(false)
    }
  }

  const label: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }

  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '480px', maxHeight: '80vh', overflowY: 'auto', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
        <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '24px' }}>{t('marks.newExam')}</div>
        <label style={label}>{t('exam.name')}<input value={name} onChange={(e) => setName(e.target.value)} style={field()} /></label>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
          <label style={label}>{t('exam.starts')}<input type="date" value={starts} onChange={(e) => setStarts(e.target.value)} style={field()} /></label>
          <label style={label}>{t('exam.ends')}<input type="date" value={ends} onChange={(e) => setEnds(e.target.value)} style={field()} /></label>
        </div>
        <div style={{ ...label, gap: '8px' }}>
          {t('exam.subjects')}
          {subjects.map((cs) => (
            <div key={cs.id} style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: 1, fontSize: '14px', color: 'var(--ink)', cursor: 'pointer' }}>
                <input type="checkbox" checked={cs.id in picked} onChange={() => toggle(cs.id)} />
                {cs.class_display} · {cs.subject_name}
              </label>
              {cs.id in picked ? (
                <input inputMode="numeric" value={picked[cs.id]} onChange={(e) => setPicked((p) => ({ ...p, [cs.id]: e.target.value.replace(/\D/g, '') }))} style={{ ...field(), height: '38px', width: '80px' }} aria-label={t('exam.max')} />
              ) : null}
            </div>
          ))}
        </div>
        <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" onClick={onClose} style={secondaryBtn()}>{t('exam.cancel')}</button>
          <button type="button" onClick={create} disabled={busy || name.trim() === '' || Object.keys(picked).length === 0} style={{ ...primaryBtn(), opacity: busy || name.trim() === '' || Object.keys(picked).length === 0 ? 0.5 : 1 }}>{t('exam.create')}</button>
        </div>
      </div>
    </div>
  )
}
