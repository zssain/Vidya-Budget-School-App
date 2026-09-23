import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { AttendanceMonthDto, AttendanceSheetDto, ClassDto } from '@/lib/api'

// Desktop attendance register (prompts/P07 §5): class + date day grid (P/A/L
// buttons in a table), month view (days × students with totals + %), Principal
// direct corrections (audited with reason).

type View = 'day' | 'month'
type Mark = 'P' | 'A' | 'L'

function todayIso(): string {
  return new Date().toISOString().slice(0, 10)
}

const MARK_ON: Record<Mark, CSSProperties> = {
  P: { background: 'var(--accent)', color: 'var(--white)', borderColor: 'var(--accent)' },
  A: { background: 'var(--danger)', color: 'var(--white)', borderColor: 'var(--danger)' },
  L: { background: 'var(--gold)', color: 'var(--navy)', borderColor: 'var(--gold)' },
}

function markBtn(active: boolean, m: Mark): CSSProperties {
  return {
    width: '44px',
    height: '44px',
    borderRadius: '8px',
    border: '1px solid var(--line)',
    background: 'var(--white)',
    color: 'var(--muted)',
    fontWeight: 600,
    fontSize: '14px',
    cursor: 'pointer',
    ...(active ? MARK_ON[m] : {}),
  }
}

function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px' }
}
function seg(sel: boolean): CSSProperties {
  return { minWidth: '64px', padding: '0 16px', height: '44px', fontSize: '13px', fontWeight: 500, border: 'none', borderRight: '1px solid var(--line-strong)', background: sel ? 'var(--accent)' : 'transparent', color: sel ? 'var(--white)' : 'var(--ink)', cursor: 'pointer' }
}

const MARK_BG: Record<string, string> = { P: 'var(--accent-12)', A: 'var(--pill-unpaid-bg)', L: 'var(--unmarked)' }
const MARK_FG: Record<string, string> = { P: 'var(--accent)', A: 'var(--pill-unpaid-fg)', L: 'var(--gold-text)' }

export default function AttendanceRegisterScreen() {
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [classId, setClassId] = useState('')
  const [date, setDate] = useState(todayIso())
  const [view, setView] = useState<View>('day')
  const [sheet, setSheet] = useState<AttendanceSheetDto | null>(null)
  const [marks, setMarks] = useState<Record<string, Mark>>({})
  const [month, setMonth] = useState<AttendanceMonthDto | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [correct, setCorrect] = useState<{ studentId: string; mark: Mark } | null>(null)
  const [reason, setReason] = useState('')

  const flash = (m: string) => {
    setToast(m)
    window.setTimeout(() => setToast(null), 2600)
  }

  useEffect(() => {
    api.list_classes().then((cs) => {
      setClasses(cs)
      if (cs[0] && classId === '') setClassId(cs[0].id)
    }).catch(() => setClasses([]))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const loadDay = useCallback(() => {
    if (!classId) return
    api.get_attendance_sheet(classId, date).then((s) => {
      setSheet(s)
      const m: Record<string, Mark> = {}
      for (const r of s.rows) if (r.mark) m[r.student_id] = r.mark as Mark
      setMarks(m)
    }).catch(() => setSheet(null))
  }, [classId, date])

  const loadMonth = useCallback(() => {
    if (!classId) return
    api.attendance_month(classId, date.slice(0, 7)).then(setMonth).catch(() => setMonth(null))
  }, [classId, date])

  useEffect(() => {
    if (view === 'day') loadDay()
    else loadMonth()
  }, [view, loadDay, loadMonth])

  const submitted = sheet?.status === 'submitted'

  const toggle = (studentId: string, m: Mark) => {
    if (submitted) {
      setCorrect({ studentId, mark: m })
      return
    }
    setMarks((prev) => {
      const next = { ...prev }
      if (next[studentId] === m) delete next[studentId]
      else next[studentId] = m
      return next
    })
  }

  const marksList = () => Object.entries(marks).map(([student_id, mark]) => ({ student_id, mark }))
  const unmarked = (sheet?.rows.length ?? 0) - Object.keys(marks).length

  const saveDraft = async () => {
    if (!classId) return
    try { await api.save_attendance_draft(classId, date, marksList()); flash(t('areg.saved')); loadDay() } catch (e) { flash(t((e as api.CmdError).message_key)) }
  }
  const submit = async () => {
    if (!classId) return
    try { await api.submit_attendance(classId, date, marksList()); flash(t('areg.saved')); loadDay() } catch (e) { flash(t((e as api.CmdError).message_key, (e as api.CmdError).vars as Record<string, string | number>)) }
  }
  const doCorrect = async () => {
    if (!correct || !classId) return
    try {
      await api.correct_attendance(classId, date, correct.studentId, correct.mark, reason)
      setCorrect(null); setReason(''); loadDay(); flash(t('areg.saved'))
    } catch (e) { flash(t((e as api.CmdError).message_key)) }
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('areg.eyebrow')}
        title={t('areg.title')}
        sub={t('areg.sub')}
        actions={
          <div style={{ display: 'inline-flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
            <button type="button" style={seg(view === 'day')} onClick={() => setView('day')}>{t('areg.view.day')}</button>
            <button type="button" style={seg(view === 'month')} onClick={() => setView('month')}>{t('areg.view.month')}</button>
          </div>
        }
      />

      <div style={{ display: 'flex', gap: '12px', alignItems: 'center', flexWrap: 'wrap' }}>
        <select value={classId} onChange={(e) => setClassId(e.target.value)} style={field()} aria-label={t('areg.pickClass')}>
          {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
        </select>
        <input type={view === 'month' ? 'month' : 'date'} value={view === 'month' ? date.slice(0, 7) : date} onChange={(e) => setDate(view === 'month' ? `${e.target.value}-01` : e.target.value)} style={field()} />
        {view === 'day' && sheet ? (
          <Pill variant={submitted ? 'paid' : sheet.status === 'draft' ? 'partpaid' : 'neutral'}>
            {submitted ? t('areg.submitted') : sheet.status === 'draft' ? t('areg.draft') : t('areg.notStarted')}
          </Pill>
        ) : null}
      </div>

      {view === 'day' ? (
        <>
          {submitted ? <div style={{ fontSize: '12px', color: 'var(--muted)' }}>{t('areg.correctHint')}</div> : null}
          <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
            {(sheet?.rows ?? []).map((r, i) => {
              const cur = marks[r.student_id]
              return (
                <div key={r.student_id} style={{ display: 'flex', alignItems: 'center', gap: '14px', padding: '7px 7px 7px 14px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', background: !cur && !submitted ? 'var(--unmarked)' : 'transparent' }}>
                  <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '16px', color: 'var(--muted)', width: '32px' }}>{r.roll_no ?? ''}</span>
                  <span style={{ flex: 1, fontSize: '15px' }}>{r.name}</span>
                  {(['P', 'A', 'L'] as Mark[]).map((m) => (
                    <button key={m} type="button" aria-pressed={cur === m} aria-label={`${r.name} ${m}`} style={markBtn(cur === m, m)} onClick={() => toggle(r.student_id, m)}>
                      {t(`areg.${m === 'P' ? 'present' : m === 'A' ? 'absent' : 'leave'}`)}
                    </button>
                  ))}
                </div>
              )
            })}
          </div>
          {!submitted ? (
            <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
              <button type="button" onClick={saveDraft} style={{ height: '46px', padding: '0 20px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('areg.saveDraft')}</button>
              <button type="button" onClick={submit} disabled={unmarked > 0} style={{ height: '46px', padding: '0 20px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: unmarked > 0 ? 'default' : 'pointer', opacity: unmarked > 0 ? 0.5 : 1 }}>{t('areg.submit')}</button>
              {unmarked > 0 ? <span style={{ fontSize: '13px', color: 'var(--gold-text)' }}>{t('areg.notMarked', { n: unmarked })}</span> : null}
            </div>
          ) : null}
        </>
      ) : (
        <div style={{ overflowX: 'auto', background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px' }}>
          {!month || month.days.length === 0 ? (
            <div style={{ padding: '40px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('areg.monthEmpty')}</div>
          ) : (
            <table style={{ borderCollapse: 'collapse', fontSize: '12px', width: '100%' }}>
              <thead>
                <tr style={{ color: 'var(--muted)' }}>
                  <th style={{ textAlign: 'left', padding: '10px 8px', position: 'sticky', left: 0, background: 'var(--surface)' }}>{t('areg.col.student')}</th>
                  {month.days.map((d) => <th key={d} style={{ padding: '10px 4px' }}>{d.slice(8)}</th>)}
                  <th style={{ padding: '10px 8px' }}>{t('areg.col.p')}</th>
                  <th style={{ padding: '10px 8px' }}>{t('areg.col.a')}</th>
                  <th style={{ padding: '10px 8px' }}>{t('areg.col.l')}</th>
                  <th style={{ padding: '10px 8px' }}>{t('areg.col.pct')}</th>
                </tr>
              </thead>
              <tbody>
                {month.students.map((s) => (
                  <tr key={s.id} style={{ borderTop: '1px solid var(--track)' }}>
                    <td style={{ textAlign: 'left', padding: '8px', position: 'sticky', left: 0, background: 'var(--surface)', whiteSpace: 'nowrap' }}>{s.roll_no != null ? `${s.roll_no}. ` : ''}{s.name}</td>
                    {s.marks.map((m, di) => (
                      <td key={di} style={{ padding: '6px 4px', textAlign: 'center' }}>
                        {m ? <span style={{ display: 'inline-block', minWidth: '20px', borderRadius: '4px', background: MARK_BG[m], color: MARK_FG[m], fontWeight: 600 }}>{m}</span> : <span style={{ color: 'var(--line-strong)' }}>·</span>}
                      </td>
                    ))}
                    <td style={{ textAlign: 'center', color: 'var(--accent)' }}>{s.present}</td>
                    <td style={{ textAlign: 'center', color: 'var(--pill-unpaid-fg)' }}>{s.absent}</td>
                    <td style={{ textAlign: 'center', color: 'var(--gold-text)' }}>{s.leave}</td>
                    <td style={{ textAlign: 'center', fontWeight: 600 }}>{(s.pct_tenths / 10).toFixed(1)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {correct ? (
        <div onClick={() => setCorrect(null)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: '400px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
            <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '22px' }}>{t('areg.correctTitle')} → {correct.mark}</div>
            <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>
              {t('areg.correctReason')}
              <input value={reason} onChange={(e) => setReason(e.target.value)} style={field()} />
            </label>
            <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              <button type="button" onClick={() => setCorrect(null)} style={{ height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', cursor: 'pointer' }}>{t('areg.cancel')}</button>
              <button type="button" onClick={doCorrect} disabled={!reason.trim()} style={{ height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer', opacity: reason.trim() ? 1 : 0.5 }}>{t('areg.confirm')}</button>
            </div>
          </div>
        </div>
      ) : null}

      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}
