// Record expense sheet (P15 Step 3/4, prototype `accounts` state 2). Category
// chips from the expense ledger accounts, amount, paid by (Cash/UPI/Bank),
// details, vendor and the honest "Voucher V-… is given when you save" note. The
// bill photo is wired to the encrypted attachment store in Step 2.

import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import { compressImageToJpegBase64 } from '@/lib/image'
import * as api from '@/lib/api'
import type { ExpenseAccountDto } from '@/lib/api'

const PAID_BY: { key: string; label: string }[] = [
  { key: 'cash', label: 'accounts.paid.cash' },
  { key: 'upi', label: 'accounts.paid.upi' },
  { key: 'bank', label: 'accounts.paid.bank' },
]

function chip(on: boolean): CSSProperties {
  return { padding: '7px 14px', borderRadius: '999px', border: `1px solid ${on ? 'var(--accent)' : 'var(--line-strong)'}`, background: on ? 'var(--accent-12)' : 'var(--surface)', color: on ? 'var(--accent)' : 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function seg(on: boolean): CSSProperties {
  return { flex: 1, height: '40px', border: 'none', borderRight: '1px solid var(--line-strong)', background: on ? 'var(--accent)' : 'var(--surface)', color: on ? 'var(--white)' : 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', padding: '0 12px', boxSizing: 'border-box' }
}

export default function RecordExpenseSheet({ onClose, onSaved }: { onClose: () => void; onSaved: () => void }) {
  const [cats, setCats] = useState<ExpenseAccountDto[]>([])
  const [category, setCategory] = useState('')
  const [rupees, setRupees] = useState('')
  const [paidVia, setPaidVia] = useState('cash')
  const [details, setDetails] = useState('')
  const [vendor, setVendor] = useState('')
  const [billHash, setBillHash] = useState<string | null>(null)
  const [billName, setBillName] = useState<string | null>(null)
  const [billBusy, setBillBusy] = useState(false)
  const [busy, setBusy] = useState(false)
  const [warn, setWarn] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const onBill = async (file: File | undefined) => {
    if (!file) return
    setBillBusy(true)
    setErr(null)
    try {
      const b64 = await compressImageToJpegBase64(file)
      const hash = await api.save_attachment(b64)
      setBillHash(hash)
      setBillName(file.name)
    } catch {
      setErr(t('accounts.exp.billError'))
    } finally {
      setBillBusy(false)
    }
  }

  useEffect(() => {
    api.list_expense_accounts().then((c) => { setCats(c); if (c[0]) setCategory(c[0].id) }).catch(() => setCats([]))
  }, [])

  const amountPaise = Math.round(Number(rupees || '0') * 100)
  const save = async () => {
    setErr(null)
    setWarn(null)
    setBusy(true)
    try {
      const dto = await api.record_expense({ category_account_id: category, amount_paise: amountPaise, paid_via: paidVia, details: details || null, vendor: vendor || null, bill_attachment: billHash, spent_on: null })
      if (dto.cash_warning) {
        // Honest: the expense saved, but cash in hand went negative.
        setWarn(t('accounts.exp.cashWarn'))
        setTimeout(onSaved, 1400)
      } else {
        onSaved()
      }
    } catch {
      setErr(t('accounts.exp.saveError'))
    } finally {
      setBusy(false)
    }
  }

  const label: CSSProperties = { fontSize: '13px', fontWeight: 500, color: 'var(--ink)' }

  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ position: 'absolute', top: 0, right: 0, width: '460px', margin: '16px', marginLeft: 0, height: 'calc(100vh - 32px)', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden', animation: 'vSheet 280ms cubic-bezier(.2,.8,.2,1) both' }}>
        <div style={{ padding: '24px 24px 0', display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>{t('accounts.exp.eyebrow')}</span>
          <span style={{ fontFamily: 'var(--font-serif)', fontSize: '30px' }}>{cats.find((c) => c.id === category)?.name ?? t('accounts.exp.title')}</span>
          <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('accounts.exp.voucherNote')}</span>
        </div>

        <div style={{ padding: '20px 24px', display: 'flex', flexDirection: 'column', gap: '16px', overflowY: 'auto', flex: 1 }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={label}>{t('accounts.exp.category')}</span>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
              {cats.map((c) => (
                <button key={c.id} type="button" style={chip(category === c.id)} onClick={() => setCategory(c.id)}>{c.name}</button>
              ))}
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={label}>{t('accounts.exp.amountPaid')}</span>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: '4px', height: '62px', border: '1.5px solid var(--accent)', borderRadius: '8px', padding: '0 16px', boxShadow: '0 0 0 4px var(--accent-12)' }}>
              <span style={{ fontFamily: 'var(--font-serif)', fontSize: '28px', color: 'var(--muted)' }}>₹</span>
              <input inputMode="decimal" value={rupees} onChange={(e) => setRupees(e.target.value.replace(/[^0-9.]/g, ''))} placeholder="0" style={{ border: 'none', outline: 'none', background: 'transparent', fontFamily: 'var(--font-serif)', fontSize: '30px', color: 'var(--ink)', width: '100%' }} />
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={label}>{t('accounts.exp.paidBy')}</span>
            <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
              {PAID_BY.map((p) => (
                <button key={p.key} type="button" style={seg(paidVia === p.key)} onClick={() => setPaidVia(p.key)}>{t(p.label)}</button>
              ))}
            </div>
          </div>

          <label style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={label}>{t('accounts.exp.detailsField')}</span>
            <input value={details} onChange={(e) => setDetails(e.target.value)} style={field()} />
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={label}>{t('accounts.exp.vendor')}</span>
            <input value={vendor} onChange={(e) => setVendor(e.target.value)} style={field()} />
          </label>

          {/* Bill photo — compressed on device, stored encrypted (P15 Step 2). */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }} data-hl="bill">
            <span style={label}>{t('accounts.exp.billPhoto')}</span>
            <label style={{ display: 'flex', gap: '12px', alignItems: 'center', padding: '12px 14px', border: '1px dashed var(--line-strong)', borderRadius: '8px', cursor: 'pointer', fontSize: '13px', color: 'var(--muted)' }}>
              <input type="file" accept="image/*" style={{ display: 'none' }} onChange={(e) => onBill(e.target.files?.[0])} />
              <span>{billBusy ? t('accounts.exp.billCompressing') : billName ? `✓ ${billName}` : t('accounts.exp.billChoose')}</span>
            </label>
          </div>

          {warn ? <div style={{ fontSize: '13px', color: 'var(--gold-text)', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', borderRadius: '8px', padding: '10px 12px' }}>{warn}</div> : null}
          {err ? <div style={{ fontSize: '13px', color: 'var(--danger)' }}>{err}</div> : null}
        </div>

        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" style={{ height: '52px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '14px', cursor: 'pointer' }} onClick={onClose}>{t('accounts.cancel')}</button>
          <button type="button" disabled={busy || amountPaise <= 0 || !category} style={{ flex: 1, height: '52px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '15px', fontWeight: 500, cursor: 'pointer', opacity: busy || amountPaise <= 0 || !category ? 0.5 : 1 }} onClick={save}>
            {t('accounts.exp.save', { amount: amountPaise > 0 ? formatMoney(amountPaise) : '' })}
          </button>
        </div>
      </div>
    </div>
  )
}
