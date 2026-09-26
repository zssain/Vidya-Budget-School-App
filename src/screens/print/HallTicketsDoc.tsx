// Hall tickets (P16 Step 5, prototype `exams` hall-ticket state). A4, 4 per page:
// logo, school, exam, student name, class, roll, room, seat, schedule, signature.

import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { t } from '@/lib/i18n'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { ExamSeatingDto, SchoolDto } from '@/lib/api'
import { pageCss, PrintLogo, PrintToolbar } from './printKit'

const ticket: CSSProperties = {
  border: '1px solid var(--line)', borderRadius: '6px', padding: '12px 14px',
  display: 'flex', flexDirection: 'column', gap: '6px', boxSizing: 'border-box', minHeight: '128mm',
}

export default function HallTicketsDoc({ examId, auto = false }: { examId: string; auto?: boolean }) {
  const [data, setData] = useState<ExamSeatingDto | null>(null)
  const [school, setSchool] = useState<SchoolDto | null>(null)

  useEffect(() => {
    api.get_school().then(setSchool).catch(() => setSchool(null))
    api.get_exam_seating(examId).then(setData).catch(() => setData(null))
  }, [examId])

  useEffect(() => {
    if (auto && data) {
      const h = window.setTimeout(() => void printCurrentWindow(), 400)
      return () => window.clearTimeout(h)
    }
  }, [auto, data])

  if (!data) return <div style={{ padding: 40, color: 'var(--muted)' }}>…</div>

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', padding: '24px', fontFamily: "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif", color: 'var(--ink)' }}>
      <style>{pageCss('a4')}</style>
      <PrintToolbar backLabel={t('exams.back')} printLabel={t('exams.printTickets')} onBack={() => window.history.back()} onPrint={() => void printCurrentWindow()} style={{ maxWidth: '190mm', margin: '0 auto 16px' }} />
      <div style={{ maxWidth: '190mm', margin: '0 auto', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8mm' }}>
        {data.hall_tickets.map((h) => (
          <div key={h.student_id} style={ticket}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <PrintLogo width={96} height={31} alt={school?.name ?? ''} />
              <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontStyle: 'italic', color: 'var(--accent)' }}>{t('exams.hallTicket')}</span>
            </div>
            <span style={{ fontSize: '11px', color: 'var(--muted)' }}>{school?.name ?? ''} · {data.exam_name}</span>
            <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '20px' }}>{h.student_name}</span>
            <span style={{ fontSize: '12px' }}>{h.class_display ?? ''}{h.roll_no != null ? ` · ${t('exams.roll')} ${h.roll_no}` : ''}</span>
            <span style={{ alignSelf: 'flex-start', padding: '4px 10px', borderRadius: '6px', background: 'var(--navy)', color: 'var(--white)', fontSize: '12px', fontWeight: 600 }}>
              {h.room_name} · {t('exams.seat')} {h.seat_no}
            </span>
            {h.schedule.map((s, i) => (
              <div key={i} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', borderTop: '1px solid var(--track)', paddingTop: '4px' }}>
                <span>{s.date}{s.starts_at ? ` · ${s.starts_at}` : ''}</span>
                <span>{s.subject_name ?? ''}</span>
              </div>
            ))}
            <span style={{ fontSize: '10px', color: 'var(--muted)', marginTop: 'auto' }}>{t('exams.signature')} ____________</span>
          </div>
        ))}
      </div>
    </div>
  )
}
