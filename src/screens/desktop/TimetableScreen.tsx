// Timetable (P16 Step 1, prototype `timetable` state 1). Principal week view: a
// class picker, a days × periods grid (today highlighted), a slot editor sheet
// (class-subject + teacher, clash errors inline), copy a week to another class,
// and in-place print. Built from the mock's tokens (no component library).

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { t } from '@/lib/i18n'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { ClassDto, TimetableDto, ClassSubjectOptionDto, CmdError } from '@/lib/api'

const WEEKDAYS = [1, 2, 3, 4, 5, 6]

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function selectStyle(): CSSProperties {
  return { height: '40px', padding: '0 12px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', cursor: 'pointer' }
}

/** ISO weekday of today (Mon=1 … Sun=7). */
function todayIso(): number {
  const d = new Date().getDay() // 0=Sun … 6=Sat
  return d === 0 ? 7 : d
}

interface EditTarget {
  weekday: number
  period_no: number
}

export default function TimetableScreen() {
  const [classes, setClasses] = useState<ClassDto[]>([])
  const [classId, setClassId] = useState<string>('')
  const [data, setData] = useState<TimetableDto | null>(null)
  const [off, setOff] = useState(false)
  const [edit, setEdit] = useState<EditTarget | null>(null)
  const [flash, setFlash] = useState<string | null>(null)
  const [copyOpen, setCopyOpen] = useState(false)
  const today = todayIso()

  useEffect(() => {
    api.list_classes().then((cs) => {
      setClasses(cs)
      if (cs.length > 0) setClassId((prev) => prev || cs[0].id)
    }).catch(() => setClasses([]))
  }, [])

  const load = useCallback(() => {
    if (!classId) return
    api.get_timetable(classId)
      .then((d) => { setData(d); setOff(false) })
      .catch((e) => { setData(null); if ((e as CmdError).code === 'MODULE_OFF') setOff(true) })
  }, [classId])
  useEffect(load, [load])

  if (off) {
    return (
      <div style={{ padding: '32px 40px' }}>
        <PageTitle eyebrow={t('tt.eyebrow')} title={t('tt.title')} sub={t('tt.sub')} />
        <div style={{ marginTop: '24px', padding: '24px', background: 'var(--panel)', borderRadius: '12px', color: 'var(--muted)' }}>{t('tt.moduleOff')}</div>
      </div>
    )
  }

  const slotAt = (weekday: number, period: number) =>
    data?.slots.find((s) => s.weekday === weekday && s.period_no === period)

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={`${t('tt.eyebrow')}${data?.class_display ? ` · ${data.class_display}` : ''}`}
        title={t('tt.title')}
        sub={t('tt.sub')}
        actions={
          <div data-appshell-chrome style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
            <select aria-label={t('tt.class')} value={classId} style={selectStyle()} onChange={(e) => setClassId(e.target.value)}>
              {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
            </select>
            <button type="button" style={secondaryBtn()} onClick={() => setCopyOpen(true)}>{t('tt.copyWeek')}</button>
            <button type="button" style={secondaryBtn()} onClick={() => void printCurrentWindow()}>{t('tt.print')}</button>
          </div>
        }
      />

      {flash && <div style={{ padding: '10px 14px', borderRadius: '8px', background: 'var(--accent-10)', color: 'var(--accent)', fontSize: 13 }}>{flash}</div>}

      {data && (
        <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden', display: 'grid', gridTemplateColumns: '110px repeat(6, 1fr)' }}>
          <span style={{ padding: '12px 16px' }} />
          {WEEKDAYS.map((wd) => (
            <span key={wd} style={{ padding: '12px 16px', borderLeft: '1px solid var(--track)', background: wd === today ? 'var(--accent-10)' : 'transparent', color: wd === today ? 'var(--accent)' : 'var(--ink)', fontWeight: 600, fontSize: 14 }}>
              {t(`tt.d${wd}`)}
              {wd === today && <span style={{ display: 'block', fontSize: 11, fontWeight: 500, color: 'var(--accent)' }}>{t('tt.today')}</span>}
            </span>
          ))}
          {data.periods.map((p) => (
            <div key={p.id} style={{ display: 'contents' }}>
              <span style={{ padding: '14px 16px', borderTop: '1px solid var(--track)' }}>
                <b style={{ fontFamily: 'var(--font-serif)', fontWeight: 400, fontSize: 20 }}>{p.no}</b>
                <span style={{ display: 'block', fontSize: 12, color: 'var(--muted)' }}>{p.starts_at}</span>
              </span>
              {WEEKDAYS.map((wd) => {
                const slot = slotAt(wd, p.no)
                return (
                  <button
                    key={wd}
                    type="button"
                    onClick={() => setEdit({ weekday: wd, period_no: p.no })}
                    style={{ textAlign: 'left', padding: '10px 12px', borderTop: '1px solid var(--track)', borderLeft: '1px solid var(--track)', background: wd === today ? 'var(--accent-6)' : 'transparent', cursor: 'pointer', font: 'inherit' }}
                  >
                    <span style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: '8px 10px', borderRadius: 8, background: slot ? 'var(--bg)' : 'transparent', minHeight: 34 }}>
                      {slot ? (
                        <>
                          <b style={{ fontWeight: 600, fontSize: 13 }}>{slot.subject_name}</b>
                          <span style={{ fontSize: 12, color: 'var(--muted)' }}>{slot.teacher_name}</span>
                        </>
                      ) : (
                        <span style={{ fontSize: 12, color: 'var(--muted)' }}>+ {t('tt.add')}</span>
                      )}
                    </span>
                  </button>
                )
              })}
            </div>
          ))}
        </div>
      )}

      {edit && data && (
        <SlotEditor
          classId={data.class_id}
          target={edit}
          subjects={data.subjects}
          current={slotAt(edit.weekday, edit.period_no) ?? null}
          onClose={() => setEdit(null)}
          onSaved={() => { setEdit(null); load() }}
        />
      )}

      {copyOpen && (
        <CopyWeekSheet
          fromClassId={classId}
          classes={classes.filter((c) => c.id !== classId)}
          onClose={() => setCopyOpen(false)}
          onDone={(msg) => { setCopyOpen(false); setFlash(msg); load() }}
        />
      )}
    </div>
  )
}

function overlay(): CSSProperties {
  return { position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 20 }
}
function panel(): CSSProperties {
  return { position: 'fixed', top: 16, right: 16, bottom: 16, width: 460, background: 'var(--surface)', borderRadius: '20px', zIndex: 21, display: 'flex', flexDirection: 'column', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }
}

function SlotEditor({ classId, target, subjects, current, onClose, onSaved }: {
  classId: string
  target: EditTarget
  subjects: ClassSubjectOptionDto[]
  current: api.TimetableSlotDto | null
  onClose: () => void
  onSaved: () => void
}) {
  const usable = subjects.filter((s) => s.teacher_id !== '')
  const [csId, setCsId] = useState<string>(current?.class_subject_id ?? '')
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const save = () => {
    const chosen = usable.find((s) => s.id === csId)
    if (!chosen) { setErr(t('tt.clash.generic')); return }
    setBusy(true)
    setErr(null)
    api.save_timetable_slot({
      id: current?.id ?? null,
      class_id: classId,
      weekday: target.weekday,
      period_no: target.period_no,
      class_subject_id: chosen.id,
      teacher_id: chosen.teacher_id,
    })
      .then(onSaved)
      .catch((e) => {
        const ce = e as CmdError
        const rule = (ce.vars as Record<string, string> | undefined)?.rule
        setErr(rule ? t(`tt.clash.${rule}`) : t('tt.clash.generic'))
        setBusy(false)
      })
  }

  const clear = () => {
    if (!current) { onClose(); return }
    setBusy(true)
    api.delete_timetable_slot(current.id).then(onSaved).catch(() => setBusy(false))
  }

  return (
    <>
      <div style={overlay()} onClick={onClose} />
      <div style={panel()} role="dialog" aria-label={t('tt.editor.title')}>
        <div style={{ padding: '22px 24px 16px', borderBottom: '1px solid var(--track)' }}>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('tt.editor.title')}</div>
          <div style={{ fontSize: 13, color: 'var(--muted)', marginTop: 4 }}>{t('tt.editor.slot', { day: t(`tt.d${target.weekday}`), period: target.period_no })}</div>
        </div>
        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1 }}>
          <label style={{ fontSize: 13, fontWeight: 500 }}>{t('tt.editor.subject')}</label>
          <select value={csId} onChange={(e) => setCsId(e.target.value)} style={{ ...selectStyle(), width: '100%', height: 46 }}>
            <option value="">{t('tt.editor.pick')}</option>
            {usable.map((s) => (
              <option key={s.id} value={s.id}>{s.subject_name} — {s.teacher_name}</option>
            ))}
          </select>
          {err && <div style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</div>}
        </div>
        <div style={{ background: 'var(--panel)', padding: '16px 24px', display: 'flex', gap: 10 }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('tt.editor.cancel')}</button>
          {current && <button type="button" style={secondaryBtn()} onClick={clear} disabled={busy}>{t('tt.editor.clear')}</button>}
          <div style={{ flexGrow: 1 }} />
          <button type="button" style={primaryBtn()} onClick={save} disabled={busy || csId === ''}>{t('tt.editor.save')}</button>
        </div>
      </div>
    </>
  )
}

function CopyWeekSheet({ fromClassId, classes, onClose, onDone }: {
  fromClassId: string
  classes: ClassDto[]
  onClose: () => void
  onDone: (msg: string) => void
}) {
  const [toId, setToId] = useState('')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const run = () => {
    if (!toId) return
    setBusy(true)
    setErr(null)
    api.copy_timetable_week(fromClassId, toId)
      .then((r) => onDone(t('tt.copyDone', { copied: r.copied, skipped: r.skipped })))
      .catch((e) => {
        const ce = e as CmdError
        const rule = (ce.vars as Record<string, string> | undefined)?.rule
        setErr(rule ? t(`tt.clash.${rule}`) : t('tt.clash.generic'))
        setBusy(false)
      })
  }

  return (
    <>
      <div style={overlay()} onClick={onClose} />
      <div style={panel()} role="dialog" aria-label={t('tt.copyTo')}>
        <div style={{ padding: '22px 24px 16px', borderBottom: '1px solid var(--track)' }}>
          <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('tt.copyTo')}</div>
        </div>
        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1 }}>
          <select value={toId} onChange={(e) => setToId(e.target.value)} style={{ ...selectStyle(), width: '100%', height: 46 }}>
            <option value="">{t('tt.selectClass')}</option>
            {classes.map((c) => <option key={c.id} value={c.id}>{c.display}</option>)}
          </select>
          {err && <div style={{ fontSize: 13, color: 'var(--pill-unpaid-fg)' }}>{err}</div>}
        </div>
        <div style={{ background: 'var(--panel)', padding: '16px 24px', display: 'flex', gap: 10 }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('tt.editor.cancel')}</button>
          <div style={{ flexGrow: 1 }} />
          <button type="button" style={primaryBtn()} onClick={run} disabled={busy || toId === ''}>{t('tt.copyConfirm')}</button>
        </div>
      </div>
    </>
  )
}
