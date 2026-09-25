import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { t } from '@/lib/i18n'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { ReportCardDto, SchoolDto } from '@/lib/api'
import { pageCss, PrintLogo, PrintSchoolName, PrintAddress, PrintToolbar } from './printKit'

// Report card(s) (prompts/P07 §8): A4, one student per page (page-break-after).
// Single student (mode='student') or a whole class batch (mode='class'). Logo +
// school + session + exam, subjects × marks + total/%/grade, term attendance,
// class-teacher remark line, "Principal" signature. Incomplete → banner.

const cell: CSSProperties = { padding: '6px 8px', borderBottom: '1px solid var(--track)', fontSize: '13px' }

function Card({ card, school }: { card: ReportCardDto; school: SchoolDto | null }) {
  const adm = card.admission_no ?? card.provisional_no ?? ''
  const att = card.attendance
  return (
    <div style={{ width: '190mm', minHeight: '260mm', pageBreakAfter: 'always', background: 'var(--white)', border: '1px solid var(--line)', borderRadius: '6px', padding: '20px 24px', margin: '0 auto 16px', color: 'var(--ink)', boxSizing: 'border-box' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px', borderBottom: '2px solid var(--navy)', paddingBottom: '10px' }}>
        <PrintLogo width={120} height={40} alt={school?.name ?? ''} />
        <div style={{ flex: 1, textAlign: 'center' }}>
          <PrintSchoolName name={school?.name ?? ''} size={18} />
          <PrintAddress address={school?.address} />
          <div style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '18px', marginTop: '4px' }}>{t('rc.title')} — {card.exam_name}</div>
        </div>
        <div style={{ width: '120px', textAlign: 'right', fontSize: '11px', color: 'var(--muted)' }}>
          {t('rc.session')}: {school?.session_label ?? ''}
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '4px 24px', margin: '12px 0', fontSize: '13px' }}>
        <div><b>{card.student_name}</b></div>
        <div style={{ textAlign: 'right' }}>{card.class_display ?? ''}{card.roll_no != null ? ` · ${t('rc.subject')}—${card.roll_no}` : ''}</div>
        <div style={{ color: 'var(--muted)' }}>{adm}</div>
        <div style={{ textAlign: 'right', color: 'var(--muted)' }}>{t('rc.attendance')}: {att.marked > 0 ? `${(att.pct_tenths / 10).toFixed(1)}%` : '—'}{att.from_date ? ` (${att.from_date} → ${att.to_date})` : ''}</div>
      </div>

      <table style={{ width: '100%', borderCollapse: 'collapse', marginTop: '8px' }}>
        <thead>
          <tr style={{ textAlign: 'left', color: 'var(--muted)', borderBottom: '1px solid var(--line)' }}>
            <th style={{ ...cell, borderBottom: 'none' }}>{t('rc.subject')}</th>
            <th style={{ ...cell, borderBottom: 'none', textAlign: 'right' }}>{t('rc.max')}</th>
            <th style={{ ...cell, borderBottom: 'none', textAlign: 'right' }}>{t('rc.obtained')}</th>
            <th style={{ ...cell, borderBottom: 'none', textAlign: 'right' }}>{t('rc.grade')}</th>
          </tr>
        </thead>
        <tbody>
          {card.subjects.map((s, i) => (
            <tr key={i}>
              <td style={cell}>{s.subject_name}</td>
              <td style={{ ...cell, textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{s.max_marks}</td>
              <td style={{ ...cell, textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{s.absent ? t('rc.absent') : s.incomplete ? '—' : s.obtained}</td>
              <td style={{ ...cell, textAlign: 'right' }}>{s.grade}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {card.incomplete ? (
        <div style={{ marginTop: '12px', padding: '10px 12px', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', borderRadius: '8px', color: 'var(--gold-text)', fontSize: '13px', textAlign: 'center' }}>
          {t('rc.incomplete')}
        </div>
      ) : (
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '32px', marginTop: '12px', fontSize: '14px' }}>
          <span>{t('rc.total')}: <b style={{ fontVariantNumeric: 'tabular-nums' }}>{card.total_obtained} / {card.total_max}</b></span>
          <span>{t('rc.percent')}: <b>{(card.pct_tenths / 10).toFixed(1)}%</b></span>
          <span>{t('rc.overallGrade')}: <b>{card.grade ?? '—'}</b></span>
        </div>
      )}

      <div style={{ marginTop: '28px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
        <div style={{ fontSize: '11px', color: 'var(--muted)' }}>{t('rc.remark')}</div>
        <div style={{ borderBottom: '1px solid var(--line-strong)', height: '28px' }} />
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '40px' }}>
        <div style={{ textAlign: 'center', fontSize: '12px' }}>
          <div style={{ borderTop: '1px solid var(--ink)', width: '160px', paddingTop: '4px' }}>{t('rc.principal')}</div>
        </div>
      </div>
    </div>
  )
}

export default function ReportCardDoc({ mode, id, examId, auto = false }: { mode: 'student' | 'class'; id: string; examId: string; auto?: boolean }) {
  const [cards, setCards] = useState<ReportCardDto[] | null>(null)
  const [school, setSchool] = useState<SchoolDto | null>(null)

  useEffect(() => {
    api.get_school().then(setSchool).catch(() => setSchool(null))
    const idsP = mode === 'class' ? api.class_student_ids(id) : Promise.resolve([id])
    idsP
      .then((ids) => Promise.all(ids.map((sid) => api.get_report_card(sid, examId).catch(() => null))))
      .then((cs) => setCards(cs.filter((c): c is ReportCardDto => c != null)))
      .catch(() => setCards([]))
  }, [mode, id, examId])

  useEffect(() => {
    if (auto && cards) {
      const h = window.setTimeout(() => void printCurrentWindow(), 400)
      return () => window.clearTimeout(h)
    }
  }, [auto, cards])

  if (!cards) return <div style={{ padding: 40, color: 'var(--muted)' }}>…</div>

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', padding: '24px', fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <style>{pageCss('a4')}</style>
      <PrintToolbar backLabel={t('marks.back')} printLabel={t('rc.print')} onBack={() => window.history.back()} onPrint={() => void printCurrentWindow()} style={{ maxWidth: '190mm', margin: '0 auto 16px' }} />
      {cards.map((c) => (
        <Card key={c.student_id} card={c} school={school} />
      ))}
    </div>
  )
}
