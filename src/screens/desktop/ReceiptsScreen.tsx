import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { shareWhatsApp } from '@/lib/files'
import * as api from '@/lib/api'
import type { ReceiptDto, ReceiptSummaryDto } from '@/lib/api'

// Receipts (prompts/P07 §10): search by receipt no./student/date, open, reprint
// (Duplicate copy), request reversal (Accountant) / reverse (Principal). Print
// opens the hidden print route; WhatsApp shares receipt text (not a file).

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px', boxSizing: 'border-box', width: '100%' }
}

export default function ReceiptsScreen({ role }: { role: string }) {
  const isPrincipal = role === 'principal'
  const [query, setQuery] = useState('')
  const [debounced, setDebounced] = useState('')
  const [rows, setRows] = useState<ReceiptSummaryDto[]>([])
  const [selected, setSelected] = useState<ReceiptDto | null>(null)
  const [reverseOpen, setReverseOpen] = useState(false)
  const [reason, setReason] = useState('')
  const [toast, setToast] = useState<string | null>(null)

  const flash = (m: string) => {
    setToast(m)
    window.setTimeout(() => setToast(null), 2600)
  }

  useEffect(() => {
    const h = window.setTimeout(() => setDebounced(query.trim()), 250)
    return () => window.clearTimeout(h)
  }, [query])

  const search = useCallback(() => {
    api.search_receipts(debounced).then(setRows).catch(() => setRows([]))
  }, [debounced])
  useEffect(search, [search])

  const openReceipt = (id: string) => {
    api.get_receipt(id).then(setSelected).catch(() => setSelected(null))
  }

  const doWhatsApp = (r: ReceiptDto) => {
    const cls = r.class_display ?? ''
    const school = '' // school name is fetched in the doc; keep the message concise
    const msg = `${school ? school + ' — ' : ''}${t('receipt.doc.title')} ${r.receipt_no} · ${formatMoney(r.amount_paise)} · ${r.student_name}${cls ? ` (${cls})` : ''}. ${t('receipt.doc.balanceAfter')}: ${formatMoney(r.balance_after_paise)}.`
    void shareWhatsApp(r.guardian_mobile, msg)
  }

  const doReverse = async () => {
    if (!selected) return
    try {
      if (isPrincipal) {
        await api.reverse_payment(selected.id, reason)
        flash(t('receipts.reversedDone'))
      } else {
        await api.create_request({ kind: 'payment_reversal', target_table: 'payment', target_id: selected.id, base_version: 0, reason, after_json: JSON.stringify({ summary: `Reverse ${selected.receipt_no}` }) })
        flash(t('receipts.reversalRequested'))
      }
      setReverseOpen(false)
      setReason('')
      openReceipt(selected.id)
      search()
    } catch (e) {
      const ce = e as api.CmdError
      flash(t(ce.message_key, ce.vars as Record<string, string | number>))
    }
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle eyebrow={t('receipts.eyebrow')} title={t('receipts.title')} sub={t('receipts.sub')} />

      <div style={{ position: 'relative', maxWidth: '480px' }}>
        <Icon name="search" size={16} strokeWidth={1.75} color="var(--muted)" style={{ position: 'absolute', left: '12px', top: '14px' }} />
        <input type="search" aria-label={t('receipts.search')} placeholder={t('receipts.search')} value={query} onChange={(e) => setQuery(e.target.value)} style={{ ...field(), paddingLeft: '36px' }} />
      </div>

      <div style={{ display: 'flex', gap: '20px', alignItems: 'flex-start' }}>
        {/* List */}
        <div style={{ flex: '1 1 0', minWidth: 0, background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1.6fr 1fr 0.9fr', gap: '12px', padding: '12px 20px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
            <span>{t('receipts.col.receipt')}</span>
            <span>{t('receipts.col.student')}</span>
            <span style={{ textAlign: 'right' }}>{t('receipts.col.amount')}</span>
            <span style={{ textAlign: 'right' }}>{t('receipts.col.mode')}</span>
          </div>
          {rows.length === 0 ? (
            <div style={{ padding: '40px 20px', textAlign: 'center', color: 'var(--muted)' }}>{t('receipts.none')}</div>
          ) : (
            rows.map((r, i) => (
              <button key={r.id} type="button" onClick={() => openReceipt(r.id)} style={{ display: 'grid', gridTemplateColumns: '1.2fr 1.6fr 1fr 0.9fr', gap: '12px', width: '100%', alignItems: 'center', textAlign: 'left', padding: '12px 20px', border: 'none', borderTop: i > 0 ? '1px solid var(--track)' : 'none', background: selected?.id === r.id ? 'var(--accent-12)' : 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '13px' }}>
                <span style={{ fontWeight: 500 }}>{r.receipt_no}{r.reversed ? ' ⟲' : ''}</span>
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{r.student_name}</span>
                <span style={{ textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.amount_paise)}</span>
                <span style={{ textAlign: 'right', color: 'var(--muted)' }}>{r.mode.toUpperCase()}</span>
              </button>
            ))
          )}
        </div>

        {/* Detail */}
        <div style={{ flex: '1 1 0', minWidth: 0 }}>
          {selected ? (
            <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', padding: '24px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ fontFamily: 'var(--font-serif)', fontSize: '24px' }}>{selected.receipt_no}</div>
                {selected.reversed ? <Pill variant="reversal">{t('receipts.reversed')}</Pill> : selected.confirmed ? <Pill variant="paid">✓</Pill> : <Pill variant="partpaid">…</Pill>}
              </div>
              <div style={{ fontSize: '14px' }}>{selected.student_name} · {selected.class_display ?? '—'}</div>
              <div style={{ fontFamily: 'var(--font-serif)', fontSize: '34px', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(selected.amount_paise)}</div>
              <div style={{ fontSize: '12px', color: 'var(--muted)' }}>{selected.mode.toUpperCase()}{selected.reference ? ` · ${selected.reference}` : ''} · {selected.collected_at}</div>

              <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap', marginTop: '4px' }}>
                <button type="button" style={primaryBtn()} onClick={() => navigate(`/print/receipt/${selected.id}?auto=1`)}>{t('receipts.print')}</button>
                <button type="button" style={secondaryBtn()} onClick={() => navigate(`/print/receipt/${selected.id}?duplicate=1&auto=1`)}>{t('receipts.reprint')}</button>
                <button type="button" style={secondaryBtn()} onClick={() => doWhatsApp(selected)}>{t('receipts.whatsapp')}</button>
                {!selected.reversed ? (
                  <button type="button" style={secondaryBtn()} onClick={() => setReverseOpen(true)}>{isPrincipal ? t('receipts.reverse') : t('receipts.requestReversal')}</button>
                ) : null}
              </div>
              <div style={{ fontSize: '11px', color: 'var(--muted)' }}>{t('receipts.whatsappHelp')}</div>
            </div>
          ) : (
            <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)', border: '1px solid var(--line)', borderRadius: '16px', background: 'var(--surface)' }}>{t('receipts.open')}</div>
          )}
        </div>
      </div>

      {reverseOpen && selected ? (
        <div onClick={() => setReverseOpen(false)} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
          <div onClick={(e) => e.stopPropagation()} style={{ width: '420px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
            <div style={{ fontFamily: 'var(--font-serif)', fontSize: '22px' }}>{isPrincipal ? t('receipts.reverse') : t('receipts.requestReversal')}</div>
            <label style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }}>
              {t('receipts.reverseReason')}
              <input value={reason} onChange={(e) => setReason(e.target.value)} style={field()} />
            </label>
            <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              <button type="button" style={secondaryBtn()} onClick={() => setReverseOpen(false)}>{t('receipts.cancel')}</button>
              <button type="button" style={{ ...primaryBtn(), opacity: reason.trim() ? 1 : 0.5 }} disabled={!reason.trim()} onClick={doReverse}>{t('receipts.confirm')}</button>
            </div>
          </div>
        </div>
      ) : null}

      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}
