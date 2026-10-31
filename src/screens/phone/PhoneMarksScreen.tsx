// Marks (repair T1). Two phone screens:
//  - /teacher/marks — the exam subjects this teacher may enter (list_exams filtered
//    to the class subjects they teach, from dashboard_teacher), with each sheet's
//    status;
//  - /teacher/marks/:examSubjectId — the sheet (get_marks_sheet): one number field +
//    Absent toggle per student, Save draft (save_marks_draft) / Submit
//    (submit_marks). A submitted sheet is read-only (per-subject lock; a correction
//    goes through the Principal).
// Same commands natively and on the iPhone app (src/lib/web/marks.ts).
import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, MarksSheetDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import PhoneScreen, { PHONE_CARD, PHONE_PRIMARY, PHONE_ROW, PHONE_SECONDARY, PhoneEmpty, PhonePill } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { cellsFromSheet, entriesFromCells, marksOptions, type MarkCell, type MarksOption } from '@/lib/teacherPhone'

const tone = (status: string) => (status === 'submitted' ? 'ok' : status === 'draft' ? 'warn' : 'muted') as 'ok' | 'warn' | 'muted'

export function PhoneMarksListScreen() {
  useLang()
  const [opts, setOpts] = useState<MarksOption[] | null>(null)
  useEffect(() => {
    Promise.all([api.dashboard_teacher(), api.list_exams()])
      .then(([d, exams]) => setOpts(marksOptions(exams, d.class_subject_ids)))
      .catch(() => setOpts([]))
  }, [])

  // Group rows under their exam (already newest first).
  const groups: { exam_id: string; exam_name: string; rows: MarksOption[] }[] = []
  for (const o of opts ?? []) {
    const g = groups.find((x) => x.exam_id === o.exam_id)
    if (g) g.rows.push(o)
    else groups.push({ exam_id: o.exam_id, exam_name: o.exam_name, rows: [o] })
  }

  return (
    <PhoneScreen title={t('tp.marks.title')}>
      {opts === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : groups.length === 0 ? (
        <PhoneEmpty>{t('tp.marks.none')}</PhoneEmpty>
      ) : (
        groups.map((g) => (
          <section key={g.exam_id} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{g.exam_name}</span>
            <div style={PHONE_CARD}>
              {g.rows.map((o, i) => (
                <button
                  key={o.subject.id}
                  type="button"
                  onClick={() => navigate(`/teacher/marks/${o.subject.id}`)}
                  style={{ ...PHONE_ROW, width: '100%', textAlign: 'left', border: 0, borderTop: i === 0 ? 0 : PHONE_ROW.borderTop, background: 'transparent', color: 'inherit' }}
                >
                  <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0 }}>
                    <b style={{ fontWeight: 600, fontSize: 15 }}>{o.subject.subject_name}</b>
                    <span style={{ fontSize: 12, color: 'var(--muted)' }}>
                      {o.subject.class_display ?? ''} · {t('tp.marks.outOf', { n: o.subject.max_marks })}
                    </span>
                  </span>
                  <PhonePill tone={tone(o.subject.status)}>{t(`tp.status.${o.subject.status}`)}</PhonePill>
                  <Icon name="chevronRight" size={18} strokeWidth={1.6} />
                </button>
              ))}
            </div>
          </section>
        ))
      )}
    </PhoneScreen>
  )
}

export function PhoneMarksSheetScreen({ examSubjectId }: { examSubjectId: string }) {
  useLang()
  const [sheet, setSheet] = useState<MarksSheetDto | null>(null)
  const [cells, setCells] = useState<MarkCell[]>([])
  const [bad, setBad] = useState<Set<string>>(new Set())
  const [err, setErr] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(() => {
    api
      .get_marks_sheet(examSubjectId)
      .then((s) => {
        setSheet(s)
        setCells(cellsFromSheet(s))
      })
      .catch((e) => setErr(t((e as CmdError).message_key, (e as CmdError).vars as Record<string, string | number>)))
  }, [examSubjectId])
  useEffect(load, [load])

  const locked = sheet?.status === 'submitted'
  const flash = (msg: string) => {
    setToast(msg)
    window.setTimeout(() => setToast(null), 2200)
  }

  const write = async (submit: boolean) => {
    if (!sheet || locked) return
    setErr(null)
    const { entries, invalid } = entriesFromCells(cells, sheet.max_marks)
    setBad(new Set(invalid))
    if (invalid.length > 0) {
      setErr(t('tp.marks.rangeErr', { max: sheet.max_marks }))
      return
    }
    if (submit && !window.confirm(t('tp.marks.submitConfirm'))) return
    setBusy(true)
    try {
      if (submit) await api.submit_marks(examSubjectId, entries)
      else await api.save_marks_draft(examSubjectId, entries)
      flash(t(submit ? 'tp.marks.done' : 'tp.marks.saved'))
      load()
    } catch (e) {
      const ce = e as CmdError
      setErr(t(ce.message_key, ce.vars as Record<string, string | number>))
    } finally {
      setBusy(false)
    }
  }

  const setCell = (i: number, patch: Partial<MarkCell>) => setCells((cur) => cur.map((c, j) => (j === i ? { ...c, ...patch } : c)))

  const title = sheet ? sheet.subject_name : t('tp.marks.title')
  const sub = sheet ? `${sheet.class_display ?? ''} · ${t('tp.marks.outOf', { n: sheet.max_marks })}` : undefined

  return (
    <PhoneScreen
      title={title}
      sub={sub}
      back="/teacher/marks"
      toast={toast}
      footer={
        sheet && !locked && cells.length > 0 ? (
          <>
            <button type="button" disabled={busy} onClick={() => void write(true)} style={PHONE_PRIMARY}>{t('tp.marks.submit')}</button>
            <button type="button" disabled={busy} onClick={() => void write(false)} style={PHONE_SECONDARY}>{t('tp.marks.saveDraft')}</button>
          </>
        ) : undefined
      }
    >
      {err ? <span role="alert" style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</span> : null}
      {!sheet ? (
        err ? null : <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : (
        <>
          {locked ? (
            <div style={{ padding: '12px 14px', borderRadius: 12, background: 'var(--unmarked)', border: '1px solid var(--gold-line)', color: 'var(--gold-text)', fontSize: 13 }}>{t('tp.marks.locked')}</div>
          ) : null}
          {cells.length === 0 ? (
            <PhoneEmpty>{t('tp.marks.noStudents')}</PhoneEmpty>
          ) : (
            <div style={PHONE_CARD}>
              {cells.map((c, i) => (
                <div key={c.student_id} style={{ ...PHONE_ROW, borderTop: i === 0 ? 0 : PHONE_ROW.borderTop }}>
                  <span style={{ width: 26, fontSize: 12, color: 'var(--muted)', flexShrink: 0 }}>{c.roll_no ?? ''}</span>
                  <span style={{ flexGrow: 1, minWidth: 0, fontSize: 15, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.name}</span>
                  <input
                    aria-label={c.name}
                    inputMode="numeric"
                    pattern="[0-9]*"
                    value={c.absent ? '' : c.value}
                    disabled={locked || c.absent}
                    onChange={(e) => setCell(i, { value: e.target.value.replace(/[^0-9]/g, '').slice(0, 4) })}
                    style={{ width: 64, height: 40, borderRadius: 8, border: `1px solid ${bad.has(c.student_id) ? 'var(--danger)' : 'var(--line-strong)'}`, background: locked || c.absent ? 'var(--panel)' : 'var(--white)', color: 'var(--ink)', fontSize: 16, textAlign: 'center', flexShrink: 0 }}
                  />
                  <button
                    type="button"
                    aria-pressed={c.absent}
                    disabled={locked}
                    onClick={() => setCell(i, { absent: !c.absent })}
                    style={{ height: 40, padding: '0 10px', borderRadius: 8, border: `1px solid ${c.absent ? 'var(--pill-unpaid-fg)' : 'var(--line-strong)'}`, background: c.absent ? 'var(--pill-unpaid-bg)' : 'var(--surface)', color: c.absent ? 'var(--pill-unpaid-fg)' : 'var(--muted)', fontSize: 12, fontWeight: 600, flexShrink: 0 }}
                  >
                    {t('tp.marks.absent')}
                  </button>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </PhoneScreen>
  )
}
