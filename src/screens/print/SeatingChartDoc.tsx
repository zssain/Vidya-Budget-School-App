// Seating charts (P16 Step 5). A4, one room per section: the room's grid of seats
// (roll · class), the invigilator and a signature line. Deterministic layout.

import { useEffect, useState } from 'react'
import { t } from '@/lib/i18n'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { ExamSeatingDto, SchoolDto } from '@/lib/api'
import { pageCss, PrintLogo, PrintSchoolName, PrintToolbar } from './printKit'

export default function SeatingChartDoc({ examId, auto = false }: { examId: string; auto?: boolean }) {
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
      <PrintToolbar backLabel={t('exams.back')} printLabel={t('exams.printCharts')} onBack={() => window.history.back()} onPrint={() => void printCurrentWindow()} style={{ maxWidth: '190mm', margin: '0 auto 16px' }} />
      {data.rooms.filter((r) => r.seats.length > 0).map((room) => (
        <div key={room.id} style={{ width: '190mm', margin: '0 auto 16px', pageBreakAfter: 'always', background: 'var(--white)', border: '1px solid var(--line)', borderRadius: '6px', padding: '20px 24px', boxSizing: 'border-box' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '16px', borderBottom: '2px solid var(--navy)', paddingBottom: '10px' }}>
            <PrintLogo width={110} height={36} alt={school?.name ?? ''} />
            <div style={{ flex: 1, textAlign: 'center' }}>
              <PrintSchoolName name={school?.name ?? ''} size={16} />
              <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '16px' }}>{data.exam_name} — {room.name}</div>
            </div>
            <div style={{ width: '140px', textAlign: 'right', fontSize: '11px', color: 'var(--muted)' }}>{t('exams.invigilator')}: {room.invigilator_name ?? '—'}</div>
          </div>
          <div style={{ textAlign: 'center', margin: '10px 0', fontSize: '11px', letterSpacing: '0.14em', color: 'var(--muted)' }}>{t('exams.board')}</div>
          <div style={{ display: 'grid', gridTemplateColumns: `repeat(${Math.min(room.cols, 10)}, 1fr)`, gap: '4px' }}>
            {room.seats.map((s) => (
              <div key={s.seat_no} style={{ border: '1px solid var(--track)', borderRadius: '4px', padding: '4px', fontSize: '9px', textAlign: 'center', background: s.class_slot === 'a' ? 'var(--accent-6)' : 'var(--pill-partpaid-bg)' }}>
                <div style={{ fontWeight: 600 }}>{s.class_display ?? ''} · {s.roll_no ?? '—'}</div>
                <div style={{ color: 'var(--muted)' }}>{t('exams.seat')} {s.seat_no}</div>
              </div>
            ))}
          </div>
          <div style={{ marginTop: '24px', fontSize: '11px', color: 'var(--muted)' }}>{t('exams.signature')} ____________</div>
        </div>
      ))}
    </div>
  )
}
