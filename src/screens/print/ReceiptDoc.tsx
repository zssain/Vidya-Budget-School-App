import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { printCurrentWindow } from '@/lib/print'
import * as api from '@/lib/api'
import type { ReceiptDto, SchoolDto } from '@/lib/api'
import logoLight from '@/assets/vidya-horizontal-on-light.svg'

// Printable fee receipt (docs/00 §PRINTING). A hidden print route renders ONLY
// this document with @page print CSS (A5 or 80mm thermal, per the size param).
// Reprints are marked "Duplicate copy". No invented legal text.

type Size = 'a5' | '80mm'

function pageCss(size: Size): string {
  const page = size === '80mm' ? '@page { size: 80mm auto; margin: 4mm; }' : '@page { size: A5; margin: 10mm; }'
  return `${page}
@media print { .no-print { display: none !important; } body { background: #ffffff; } }`
}

const label: CSSProperties = { color: 'var(--muted)', fontSize: '12px' }
const value: CSSProperties = { color: 'var(--ink)', fontSize: '13px', fontWeight: 500, textAlign: 'right' }

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', padding: '2px 0' }}>
      <span style={label}>{k}</span>
      <span style={value}>{v}</span>
    </div>
  )
}

export default function ReceiptDoc({ id, size = 'a5', lang = 'en', duplicate = false, auto = false }: { id: string; size?: Size; lang?: 'en' | 'hi'; duplicate?: boolean; auto?: boolean }) {
  const [r, setR] = useState<ReceiptDto | null>(null)
  const [school, setSchool] = useState<SchoolDto | null>(null)
  const [err, setErr] = useState(false)

  useEffect(() => {
    api.get_receipt(id).then(setR).catch(() => setErr(true))
    api.get_school().then(setSchool).catch(() => setSchool(null))
  }, [id])

  // Auto-open the print dialog once the document has laid out.
  useEffect(() => {
    if (auto && r) {
      const h = window.setTimeout(() => void printCurrentWindow(), 300)
      return () => window.clearTimeout(h)
    }
  }, [auto, r])

  if (err) return <div style={{ padding: 40, color: 'var(--muted)' }}>—</div>
  if (!r) return <div style={{ padding: 40, color: 'var(--muted)' }}>…</div>

  const width = size === '80mm' ? '80mm' : '148mm'
  const words = lang === 'hi' ? r.amount_words_hi : r.amount_words_en
  const adm = r.admission_no ?? r.provisional_no ?? t('students.admissionPending')

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '24px', fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <style>{pageCss(size)}</style>

      {/* toolbar (never printed) */}
      <div className="no-print" style={{ display: 'flex', gap: '10px', marginBottom: '16px' }}>
        <button type="button" onClick={() => window.history.back()} style={{ height: '38px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', cursor: 'pointer' }}>{t('students.back')}</button>
        <button type="button" onClick={() => void printCurrentWindow()} style={{ height: '38px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }}>{t('receipts.print')}</button>
      </div>

      {/* the document */}
      <div style={{ width, background: 'var(--white)', border: '1px solid var(--line)', borderRadius: '6px', padding: '18px 20px', color: 'var(--ink)', boxSizing: 'border-box' }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px', textAlign: 'center', borderBottom: '1px solid var(--track)', paddingBottom: '10px' }}>
          <img src={logoLight} alt={school?.name ?? ''} width={140} height={46} style={{ width: '140px', height: '46px' }} />
          <div style={{ fontWeight: 600, fontSize: '15px' }}>{school?.name ?? ''}</div>
          {school?.address ? <div style={{ fontSize: '11px', color: 'var(--muted)' }}>{school.address}</div> : null}
        </div>

        {duplicate ? <div style={{ textAlign: 'center', fontSize: '11px', fontWeight: 600, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--gold-text)', marginTop: '8px' }}>{t('receipt.doc.duplicate')}</div> : null}
        {r.reversed ? <div style={{ textAlign: 'center', fontSize: '12px', color: 'var(--pill-unpaid-fg)', marginTop: '6px' }}>{t('receipt.doc.reversed')}</div> : null}

        <div style={{ textAlign: 'center', fontFamily: "'Newsreader', Georgia, serif", fontSize: '22px', margin: '10px 0' }}>{t('receipt.doc.title')}</div>

        <Row k={t('receipt.doc.no')} v={r.receipt_no} />
        <Row k={t('receipt.doc.date')} v={r.collected_at} />
        <Row k={t('receipt.doc.student')} v={r.student_name} />
        <Row k={t('receipt.doc.class')} v={r.class_display ?? '—'} />
        <Row k={t('receipt.doc.admission')} v={adm} />

        <div style={{ borderTop: '1px solid var(--track)', margin: '10px 0', paddingTop: '8px' }}>
          {r.lines.map((l, i) => (
            <div key={i} style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0', fontSize: '13px' }}>
              <span>{l.label}</span>
              <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatMoney(l.amount_paise)}</span>
            </div>
          ))}
          {r.advance_credit_paise > 0 ? (
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0', fontSize: '13px', color: 'var(--accent)' }}>
              <span>{t('receipt.doc.advanceCredit')}</span>
              <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.advance_credit_paise)}</span>
            </div>
          ) : null}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', borderTop: '1px solid var(--track)', paddingTop: '8px' }}>
          <span style={{ fontWeight: 600 }}>{t('receipt.doc.total')}</span>
          <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '22px', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.amount_paise)}</span>
        </div>
        <div style={{ fontSize: '11px', color: 'var(--muted)', fontStyle: 'italic', marginTop: '2px' }}>{t('receipt.doc.inWords')}: {words}</div>

        <div style={{ marginTop: '10px' }}>
          <Row k={t('receipt.doc.mode')} v={r.mode.toUpperCase()} />
          {r.reference ? <Row k={t('receipt.doc.reference')} v={r.reference} /> : null}
          <Row k={t('receipt.doc.balanceAfter')} v={formatMoney(r.balance_after_paise)} />
          {r.collected_by_name ? <Row k={t('receipt.doc.collectedBy')} v={r.collected_by_name} /> : null}
        </div>

        {!r.confirmed ? <div style={{ fontSize: '11px', color: 'var(--gold-text)', marginTop: '10px', textAlign: 'center' }}>{t('receipt.doc.waiting')}</div> : null}
        <div style={{ fontSize: '10px', color: 'var(--muted)', marginTop: '10px', textAlign: 'center' }}>{t('receipt.doc.generated')}</div>
      </div>
    </div>
  )
}
