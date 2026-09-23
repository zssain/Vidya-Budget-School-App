import { useEffect, useRef, useState } from 'react'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { ReceiptSummaryDto, StudentDto } from '@/lib/api'

// Global search (prompts/P07 Shared): Ctrl/Cmd+K palette over students +
// receipts (scoped by role via the underlying commands). Esc closes.
export default function GlobalSearch({ role }: { role: string }) {
  const base = role === 'accountant' ? '/accountant/students' : '/principal/students'
  const [q, setQ] = useState('')
  const [students, setStudents] = useState<StudentDto[]>([])
  const [receipts, setReceipts] = useState<ReceiptSummaryDto[]>([])
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') window.history.back() }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  useEffect(() => {
    const query = q.trim()
    if (query === '') {
      setStudents([])
      setReceipts([])
      return
    }
    const h = window.setTimeout(() => {
      api.search_students(query).then((s) => setStudents(s.slice(0, 8))).catch(() => setStudents([]))
      api.search_receipts(query).then((r) => setReceipts(r.slice(0, 8))).catch(() => setReceipts([]))
    }, 200)
    return () => window.clearTimeout(h)
  }, [q])

  return (
    <div onClick={() => window.history.back()} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'flex', justifyContent: 'center', paddingTop: '12vh', zIndex: 50 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '640px', maxHeight: '70vh', background: 'var(--surface)', borderRadius: '16px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        <input
          ref={inputRef}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={t('search.placeholder')}
          style={{ height: '56px', border: 'none', borderBottom: '1px solid var(--track)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '18px', fontFamily: 'inherit', padding: '0 20px', outline: 'none' }}
        />
        <div style={{ overflowY: 'auto', padding: '8px 0' }}>
          {students.length === 0 && receipts.length === 0 ? (
            <div style={{ padding: '24px 20px', color: 'var(--muted)', fontSize: '13px' }}>{q.trim() ? t('search.none') : t('search.hint')}</div>
          ) : null}
          {students.length > 0 ? (
            <div style={{ padding: '8px 20px 4px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('search.students')}</div>
          ) : null}
          {students.map((s) => (
            <button key={s.id} type="button" onClick={() => navigate(`${base}/${s.id}`)} style={{ display: 'flex', justifyContent: 'space-between', width: '100%', textAlign: 'left', padding: '10px 20px', border: 'none', background: 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '14px' }}>
              <span>{s.name}</span>
              <span style={{ color: 'var(--muted)' }}>{s.class_display ?? ''} · {s.admission_no ?? s.provisional_no ?? ''}</span>
            </button>
          ))}
          {receipts.length > 0 ? (
            <div style={{ padding: '12px 20px 4px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('search.receipts')}</div>
          ) : null}
          {receipts.map((r) => (
            <button key={r.id} type="button" onClick={() => navigate(`/print/receipt/${r.id}`)} style={{ display: 'flex', justifyContent: 'space-between', width: '100%', textAlign: 'left', padding: '10px 20px', border: 'none', background: 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '14px' }}>
              <span>{r.receipt_no} · {r.student_name}</span>
              <span style={{ color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.amount_paise)}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
