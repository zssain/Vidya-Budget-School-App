// Exam seating & hall tickets (P16 Step 5, prototype `exams`). Marks & exams →
// Seating: pick an exam, manage rooms + invigilators, generate deterministic
// seating (two classes per room, alternating), preview a room grid, and print
// seating charts / hall tickets. Built from the mock tokens.

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ExamDto, ExamSeatingDto, StaffDto, SeatingResult, CmdError } from '@/lib/api'

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function selectStyle(): CSSProperties {
  return { height: '40px', padding: '0 12px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', cursor: 'pointer' }
}
const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }

export default function ExamsScreen() {
  const [exams, setExams] = useState<ExamDto[]>([])
  const [examId, setExamId] = useState('')
  const [data, setData] = useState<ExamSeatingDto | null>(null)
  const [off, setOff] = useState(false)
  const [staff, setStaff] = useState<StaffDto[]>([])
  const [result, setResult] = useState<SeatingResult | null>(null)
  const [addOpen, setAddOpen] = useState(false)

  useEffect(() => {
    api.list_exams().then((ex) => { setExams(ex); if (ex.length > 0) setExamId((p) => p || ex[0].id) }).catch(() => setExams([]))
    api.list_staff().then((s) => setStaff(s.filter((x) => x.role === 'teacher' || x.role === 'principal'))).catch(() => setStaff([]))
  }, [])

  const load = useCallback(() => {
    if (!examId) return
    api.get_exam_seating(examId)
      .then((d) => { setData(d); setOff(false) })
      .catch((e) => { if ((e as CmdError).code === 'MODULE_OFF') { setOff(true); setData(null) } })
  }, [examId])
  useEffect(load, [load])

  if (off) {
    return (
      <div style={{ padding: '32px 40px' }}>
        <PageTitle eyebrow={t('exams.eyebrow')} title={t('exams.title')} sub={t('exams.sub')} />
        <div style={{ marginTop: 24, padding: 24, background: 'var(--panel)', borderRadius: 12, color: 'var(--muted)' }}>{t('exams.moduleOff')}</div>
      </div>
    )
  }

  const generate = () => {
    setResult(null)
    api.generate_seating(examId).then((r) => { setResult(r); load() }).catch(() => {})
  }

  const totalSeated = data?.rooms.reduce((n, r) => n + r.seats.length, 0) ?? 0
  const previewRoom = data?.rooms.find((r) => r.seats.length > 0) ?? data?.rooms[0]

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 20, minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('exams.eyebrow')}
        title={t('exams.title')}
        sub={t('exams.sub')}
        actions={
          <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
            <select aria-label={t('exams.eyebrow')} value={examId} style={selectStyle()} onChange={(e) => setExamId(e.target.value)}>
              <option value="">{t('exams.pickExam')}</option>
              {exams.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
            </select>
            <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/seatingcharts/${examId}?auto=1`)} disabled={totalSeated === 0}>{t('exams.printCharts')}</button>
            <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/halltickets/${examId}?auto=1`)} disabled={totalSeated === 0}>{t('exams.printTickets')}</button>
            <button type="button" style={primaryBtn()} onClick={generate}>{t('exams.generate')}</button>
          </div>
        }
      />

      {result && result.errors.length > 0 && (
        <div style={{ padding: '12px 16px', borderRadius: 8, background: 'var(--pill-unpaid-bg)', color: 'var(--pill-unpaid-fg)', fontSize: 13, display: 'flex', flexDirection: 'column', gap: 4 }}>
          {result.errors.map((e, i) => <span key={i}>{t('exams.capacityShort', { room: e.room_name, missing: e.missing, needed: e.needed, capacity: e.capacity })}</span>)}
        </div>
      )}
      {result && result.errors.length === 0 && (
        <div style={{ padding: '10px 14px', borderRadius: 8, background: 'var(--accent-10)', color: 'var(--accent)', fontSize: 13 }}>
          {t('exams.generated', { seated: result.seated, rooms: result.rooms_used })}
          {result.unpaired_classes > 0 ? ` ${t('exams.unpaired', { n: result.unpaired_classes })}` : ''}
        </div>
      )}

      {data && (
        <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: 24 }}>
          {/* Room grid preview */}
          <div style={CARD}>
            <div style={{ padding: '20px 24px 8px' }}>
              <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('exams.roomGrid')}</div>
              <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{previewRoom?.name ?? '—'}</div>
            </div>
            {previewRoom && previewRoom.seats.length > 0 ? (
              <div style={{ padding: '4px 24px 22px', display: 'flex', flexDirection: 'column', gap: 10 }}>
                <span style={{ alignSelf: 'center', padding: '4px 30px', borderRadius: 4, background: 'var(--navy)', color: 'var(--white)', fontSize: 11, letterSpacing: '0.14em' }}>{t('exams.board')}</span>
                <div style={{ display: 'grid', gridTemplateColumns: `repeat(${Math.min(previewRoom.cols, 10)}, 1fr)`, gap: 6 }}>
                  {previewRoom.seats.slice(0, 60).map((s) => {
                    const a = s.class_slot === 'a'
                    return (
                      <span key={s.seat_no} title={s.student_name} style={{ height: 30, borderRadius: 5, background: a ? 'var(--accent-12)' : 'var(--pill-partpaid-bg)', color: a ? 'var(--accent)' : 'var(--pill-partpaid-fg)', fontSize: 10, fontWeight: 600, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                        {s.roll_no ?? s.seat_no}
                      </span>
                    )
                  })}
                </div>
              </div>
            ) : (
              <div style={{ padding: '24px', color: 'var(--muted)', fontSize: 13 }}>{t('exams.empty')}</div>
            )}
          </div>

          {/* Rooms + invigilators */}
          <div style={CARD}>
            <div style={{ padding: '20px 24px 8px', display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
              <div style={{ fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('exams.rooms')}</div>
              <button type="button" style={{ border: 0, background: 'transparent', color: 'var(--accent)', fontSize: 13, cursor: 'pointer' }} onClick={() => setAddOpen(true)}>+ {t('exams.addRoom')}</button>
            </div>
            {data.rooms.map((r) => (
              <div key={r.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 24px', borderTop: '1px solid var(--track)' }}>
                <b style={{ fontWeight: 600, flexGrow: 1 }}>{r.name}</b>
                <span style={{ fontSize: 12, color: 'var(--muted)' }}>{t('exams.capacity', { n: r.rows * r.cols })}</span>
                <span style={{ fontSize: 12, color: 'var(--muted)' }}>{r.invigilator_name ?? t('exams.noInvigilator')}</span>
                <button type="button" aria-label={t('exams.delete')} onClick={() => api.delete_exam_room(r.id).then(load)} style={{ border: 0, background: 'transparent', color: 'var(--muted)', cursor: 'pointer', fontSize: 12 }}>✕</button>
              </div>
            ))}
            {data.rooms.length === 0 && <div style={{ padding: '16px 24px', color: 'var(--muted)', fontSize: 13 }}>{t('exams.empty')}</div>}
          </div>
        </div>
      )}

      {addOpen && examId && (
        <AddRoomSheet examId={examId} staff={staff} onClose={() => setAddOpen(false)} onSaved={() => { setAddOpen(false); load() }} />
      )}
    </div>
  )
}

function AddRoomSheet({ examId, staff, onClose, onSaved }: { examId: string; staff: StaffDto[]; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState('')
  const [rows, setRows] = useState(10)
  const [cols, setCols] = useState(6)
  const [invig, setInvig] = useState('')
  const [busy, setBusy] = useState(false)

  const save = () => {
    if (!name.trim()) return
    setBusy(true)
    api.save_exam_room({ exam_id: examId, name: name.trim(), rows, cols, invigilator_id: invig || null }).then(onSaved).catch(() => setBusy(false))
  }
  const field = { height: 44, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, padding: '0 12px', boxSizing: 'border-box' as const, width: '100%' }

  return (
    <>
      <div style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 20 }} onClick={onClose} />
      <div style={{ position: 'fixed', top: 16, right: 16, bottom: 16, width: 440, background: 'var(--surface)', borderRadius: 20, zIndex: 21, display: 'flex', flexDirection: 'column', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }} role="dialog" aria-label={t('exams.addRoom')}>
        <div style={{ padding: '22px 24px 16px', borderBottom: '1px solid var(--track)', fontFamily: 'var(--font-serif)', fontSize: 22 }}>{t('exams.addRoom')}</div>
        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1 }}>
          <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('exams.roomName')}</label><input value={name} onChange={(e) => setName(e.target.value)} style={field} /></div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('exams.rows')}</label><input type="number" min={1} value={rows} onChange={(e) => setRows(Math.max(1, Number(e.target.value)))} style={field} /></div>
            <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('exams.cols')}</label><input type="number" min={1} value={cols} onChange={(e) => setCols(Math.max(1, Number(e.target.value)))} style={field} /></div>
          </div>
          <div><label style={{ fontSize: 13, fontWeight: 500 }}>{t('exams.invigilator')}</label>
            <select value={invig} onChange={(e) => setInvig(e.target.value)} style={{ ...field, cursor: 'pointer' }}>
              <option value="">{t('exams.noInvigilator')}</option>
              {staff.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
            </select>
          </div>
        </div>
        <div style={{ background: 'var(--panel)', padding: '16px 24px', display: 'flex', gap: 10 }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('exams.cancel')}</button>
          <div style={{ flexGrow: 1 }} />
          <button type="button" style={primaryBtn()} onClick={save} disabled={busy || !name.trim()}>{t('exams.save')}</button>
        </div>
      </div>
    </>
  )
}
