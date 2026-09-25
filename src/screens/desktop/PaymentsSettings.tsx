// Settings → Payments (UPI) — prototype `settings` state 1. A card in the
// Settings grid (like PrivacySettings / CustomFieldsSettings), keeping the real
// app's single-page card layout (P11 decision) while reproducing the prototype's
// Payments *content*: School UPI ID + display name + three "show the QR on…"
// toggles + a live preview QR for one student. Every control is real
// (get_payment_settings / set_payment_settings). Vidya never handles money.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CmdError, PaymentSettings } from '@/lib/api'
import { formatMoney } from '@/lib/format'
import { t, useLang } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24, gridColumn: '1 / -1' }
const SERIF = "'Newsreader', Georgia, serif"
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 4px' }

const FIELD: React.CSSProperties = { height: 46, borderRadius: 6, border: '1px solid var(--line-strong)', background: 'var(--white)', padding: '0 14px', fontSize: 14, color: 'var(--ink)', width: '100%', boxSizing: 'border-box' }

function ToggleRow({ label, on, onClick }: { label: string; on: boolean; onClick: () => void }) {
  return (
    <span style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 14px', borderRadius: 8, border: '1px solid var(--line)', background: 'var(--white)', fontSize: 14 }}>
      {label}
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-label={label}
        onClick={onClick}
        style={{ width: 46, height: 26, borderRadius: 13, cursor: 'pointer', position: 'relative', border: `1px solid ${on ? 'var(--accent)' : 'var(--line-strong)'}`, background: on ? 'var(--accent)' : 'var(--white)', transition: 'background-color .18s ease' }}
      >
        <span style={{ position: 'absolute', top: 2, left: on ? 22 : 2, width: 20, height: 20, borderRadius: 10, background: 'var(--white)', boxShadow: '0 1px 2px rgba(11,26,51,0.3)', transition: 'left .18s ease' }} />
      </button>
    </span>
  )
}

export default function PaymentsSettings() {
  useLang()
  const [upiId, setUpiId] = useState('')
  const [upiName, setUpiName] = useState('')
  const [onReceipts, setOnReceipts] = useState(true)
  const [onReminders, setOnReminders] = useState(true)
  const [onDues, setOnDues] = useState(false)
  const [preview, setPreview] = useState<PaymentSettings | null>(null)
  const [qr, setQr] = useState<string | null>(null)
  const [status, setStatus] = useState<'idle' | 'saved' | 'invalid'>('idle')

  const load = () => {
    api.get_payment_settings().then((p) => {
      setUpiId(p.upi_id ?? '')
      setUpiName(p.upi_name ?? '')
      setOnReceipts(p.on_receipts)
      setOnReminders(p.on_reminders)
      setOnDues(p.on_dues_list)
      setPreview(p)
    }).catch(() => {})
  }
  useEffect(load, [])

  // Preview QR reflects the last *saved* settings (link built in Rust).
  useEffect(() => {
    if (preview?.preview_link) api.qr_svg(preview.preview_link).then(setQr).catch(() => setQr(null))
    else setQr(null)
  }, [preview?.preview_link])

  const save = async () => {
    setStatus('idle')
    try {
      await api.set_payment_settings({ upi_id: upiId.trim() || null, upi_name: upiName.trim() || null, on_receipts: onReceipts, on_reminders: onReminders, on_dues_list: onDues })
      setStatus('saved')
      load()
    } catch (e) {
      setStatus((e as CmdError).code === 'VALIDATION' ? 'invalid' : 'idle')
    }
  }

  return (
    <div style={CARD}>
      <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('settings.pay.eyebrow')}</div>
      <h3 style={{ ...H3, fontFamily: SERIF, fontSize: 22, fontWeight: 400, margin: '2px 0 16px' }}>{t('settings.pay.title')}</h3>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 300px', gap: 28 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>{t('settings.pay.id')}</span>
            <input value={upiId} onChange={(e) => setUpiId(e.target.value)} placeholder={t('settings.pay.idPlaceholder')} autoComplete="off" style={FIELD} />
            <span style={{ fontSize: 12, color: 'var(--muted)' }}>{t('settings.pay.idHint')}</span>
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>{t('settings.pay.name')}</span>
            <input value={upiName} onChange={(e) => setUpiName(e.target.value)} placeholder={t('settings.pay.namePlaceholder')} autoComplete="off" style={FIELD} />
          </label>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>{t('settings.pay.showOn')}</span>
            <ToggleRow label={t('settings.pay.onReceipts')} on={onReceipts} onClick={() => setOnReceipts((v) => !v)} />
            <ToggleRow label={t('settings.pay.onReminders')} on={onReminders} onClick={() => setOnReminders((v) => !v)} />
            <ToggleRow label={t('settings.pay.onDues')} on={onDues} onClick={() => setOnDues((v) => !v)} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <button type="button" onClick={save} style={{ height: 44, padding: '0 20px', borderRadius: 6, border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: 14, fontWeight: 500, cursor: 'pointer' }}>{t('settings.pay.save')}</button>
            {status === 'saved' ? <span style={{ fontSize: 13, color: 'var(--accent)' }}>{t('settings.pay.saved')}</span> : null}
            {status === 'invalid' ? <span style={{ fontSize: 13, color: 'var(--danger)' }}>{t('settings.pay.invalid')}</span> : null}
          </div>
        </div>

        {/* Preview for one student (navy card, like the prototype). */}
        <div style={{ borderRadius: 14, background: 'var(--navy)', color: 'var(--white)', padding: 22, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, textAlign: 'center' }}>
          <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--on-navy-label)' }}>{t('settings.pay.previewTitle')}</span>
          {qr ? (
            <>
              <span style={{ background: 'var(--white)', padding: 12, borderRadius: 10, width: 160, height: 160, boxSizing: 'border-box' }} dangerouslySetInnerHTML={{ __html: qr }} />
              <span style={{ fontFamily: SERIF, fontSize: 24 }}>{t('settings.pay.previewPay')} <span style={{ fontStyle: 'italic', color: 'var(--gold)' }}>{formatMoney(210000)}</span></span>
              <span style={{ fontSize: 12, color: 'var(--on-navy-muted)' }}>{t('settings.pay.previewNote')}</span>
            </>
          ) : (
            <span style={{ fontSize: 13, color: 'var(--on-navy-muted)', padding: '30px 0' }}>{t('settings.pay.previewEmpty')}</span>
          )}
        </div>
      </div>
    </div>
  )
}
