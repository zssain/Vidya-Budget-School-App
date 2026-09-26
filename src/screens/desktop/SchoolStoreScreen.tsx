// School store (P15 Step 6, prototype `store`, optional module `store`). Items
// grid with stock + low-stock warning, a sale sheet (student optional, mode),
// and — for the Principal — add item + stock purchase/adjust. Hidden from the
// sidebar when the module is off; a direct visit shows a module-off notice.

import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { StoreItemDto } from '@/lib/api'
import type { Role } from '@/lib/nav'

function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', padding: '0 12px', boxSizing: 'border-box' }
}

export default function SchoolStoreScreen({ role }: { role: Role }) {
  const [items, setItems] = useState<StoreItemDto[] | null>(null)
  const [off, setOff] = useState(false)
  const [cart, setCart] = useState<Record<string, number>>({})
  const [editItem, setEditItem] = useState<StoreItemDto | 'new' | null>(null)
  const [adjust, setAdjust] = useState<StoreItemDto | null>(null)
  const [flash, setFlash] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_store_items(role === 'principal')
      .then((rows) => { setItems(rows); setOff(false) })
      .catch(() => { setItems([]); setOff(true) })
  }, [role])
  useEffect(load, [load])

  const add = (id: string) => {
    const item = items?.find((i) => i.id === id)
    const inCart = cart[id] ?? 0
    if (item && inCart < item.stock) setCart({ ...cart, [id]: inCart + 1 })
  }

  if (off) {
    return (
      <div style={{ padding: '32px 40px' }}>
        <PageTitle eyebrow={t('store.eyebrow')} title={t('store.title')} sub={t('store.sub')} />
        <div style={{ marginTop: '24px', padding: '24px', background: 'var(--panel)', borderRadius: '12px', color: 'var(--muted)' }}>{t('store.moduleOff')}</div>
      </div>
    )
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('store.eyebrow')}
        title={t('store.title')}
        sub={t('store.sub')}
        actions={role === 'principal' ? <button type="button" style={primaryBtn()} onClick={() => setEditItem('new')}>{t('store.addItem')}</button> : undefined}
      />

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '14px' }} data-hl="items">
        {(items ?? []).map((it) => (
          <div key={it.id} style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '14px', padding: '16px', display: 'flex', flexDirection: 'column', gap: '8px', opacity: it.active ? 1 : 0.5 }}>
            <div style={{ fontSize: '15px', fontWeight: 500 }}>{it.name}</div>
            <div style={{ fontFamily: 'var(--font-serif)', fontSize: '24px' }}>{formatMoney(it.price_paise)}</div>
            <div style={{ fontSize: '12px', color: it.low_stock ? 'var(--danger)' : 'var(--muted)', fontWeight: it.low_stock ? 600 : 400 }}>
              {it.low_stock ? t('store.lowStock', { n: String(it.stock) }) : t('store.inStock', { n: String(it.stock) })}
            </div>
            <div style={{ display: 'flex', gap: '8px', marginTop: '4px' }}>
              <button type="button" disabled={it.stock <= (cart[it.id] ?? 0)} style={{ ...secondaryBtn(), height: '34px', flex: 1, opacity: it.stock <= (cart[it.id] ?? 0) ? 0.5 : 1 }} onClick={() => add(it.id)}>{t('store.add')}</button>
              {role === 'principal' ? <button type="button" style={{ ...secondaryBtn(), height: '34px' }} onClick={() => setAdjust(it)}>{t('store.stock')}</button> : null}
              {role === 'principal' ? <button type="button" style={{ ...secondaryBtn(), height: '34px' }} onClick={() => setEditItem(it)}>{t('store.edit')}</button> : null}
            </div>
          </div>
        ))}
        {items && items.length === 0 ? <div style={{ gridColumn: '1 / -1', padding: '48px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('store.noItems')}</div> : null}
      </div>

      {flash ? <div style={{ padding: '14px 18px', background: 'var(--accent-12)', color: 'var(--accent)', borderRadius: '12px', fontSize: '14px' }}>{flash}</div> : null}

      {Object.keys(cart).length > 0 ? (
        <SaleSheet items={items ?? []} cart={cart} onClose={() => setCart({})} onSold={(receipt) => { setCart({}); setFlash(t('store.sold', { receipt })); load() }} />
      ) : null}
      {editItem != null ? <ItemEditor item={editItem === 'new' ? null : editItem} onClose={() => setEditItem(null)} onSaved={() => { setEditItem(null); load() }} /> : null}
      {adjust ? <StockSheet item={adjust} onClose={() => setAdjust(null)} onSaved={() => { setAdjust(null); load() }} /> : null}
    </div>
  )
}

function SaleSheet({ items, cart, onClose, onSold }: { items: StoreItemDto[]; cart: Record<string, number>; onClose: () => void; onSold: (receipt: string) => void }) {
  const [mode, setMode] = useState('cash')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const lines = Object.entries(cart).map(([id, qty]) => ({ item: items.find((i) => i.id === id)!, qty })).filter((l) => l.item)
  const total = lines.reduce((n, l) => n + l.item.price_paise * l.qty, 0)
  const record = async () => {
    setErr(null)
    setBusy(true)
    try {
      const sale = await api.record_store_sale({ items: lines.map((l) => ({ item_id: l.item.id, qty: l.qty })), mode })
      onSold(sale.receipt_no)
    } catch {
      setErr(t('store.saleError'))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ position: 'absolute', top: 0, right: 0, width: '440px', margin: '16px', marginLeft: 0, height: 'calc(100vh - 32px)', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden', animation: 'vSheet 280ms cubic-bezier(.2,.8,.2,1) both' }}>
        <div style={{ padding: '24px 24px 8px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'var(--accent)' }}>{t('store.saleEyebrow')}</span>
          <span style={{ fontFamily: 'var(--font-serif)', fontSize: '28px' }}>{t('store.saleTitle')}</span>
        </div>
        <div style={{ padding: '16px 24px', flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '14px' }} data-hl="cart">
          <div>
            {lines.map((l) => (
              <div key={l.item.id} style={{ display: 'grid', gridTemplateColumns: '1fr 50px 90px', padding: '10px 0', borderBottom: '1px solid var(--track)', fontVariantNumeric: 'tabular-nums' }}>
                <span>{l.item.name}</span>
                <span style={{ color: 'var(--muted)' }}>× {l.qty}</span>
                <span style={{ textAlign: 'right' }}>{formatMoney(l.item.price_paise * l.qty)}</span>
              </div>
            ))}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', paddingTop: '12px' }}>
              <span style={{ fontWeight: 500 }}>{t('store.total')}</span>
              <span style={{ fontFamily: 'var(--font-serif)', fontSize: '30px' }}>{formatMoney(total)}</span>
            </div>
          </div>
          <div>
            <div style={{ fontSize: '13px', fontWeight: 500, marginBottom: '8px' }}>{t('store.mode')}</div>
            <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
              {['cash', 'upi', 'cheque'].map((m) => (
                <button key={m} type="button" style={{ flex: 1, height: '40px', border: 'none', borderRight: '1px solid var(--line-strong)', background: mode === m ? 'var(--accent)' : 'var(--surface)', color: mode === m ? 'var(--white)' : 'var(--ink)', fontSize: '13px', cursor: 'pointer' }} onClick={() => setMode(m)}>{t(`store.mode.${m}`)}</button>
              ))}
            </div>
          </div>
          <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('store.saleNote')}</div>
          {err ? <div style={{ fontSize: '13px', color: 'var(--danger)' }}>{err}</div> : null}
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px' }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('store.cancel')}</button>
          <button type="button" disabled={busy || total <= 0} style={{ flex: 1, height: '52px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '15px', fontWeight: 500, cursor: 'pointer', opacity: busy || total <= 0 ? 0.5 : 1 }} onClick={record} data-hl="record">
            {t('store.record', { amount: formatMoney(total) })}
          </button>
        </div>
      </div>
    </div>
  )
}

function ItemEditor({ item, onClose, onSaved }: { item: StoreItemDto | null; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(item?.name ?? '')
  const [price, setPrice] = useState(String((item?.price_paise ?? 0) / 100))
  const [low, setLow] = useState(String(item?.low_stock_at ?? 5))
  const [active, setActive] = useState(item?.active ?? true)
  const [busy, setBusy] = useState(false)
  const save = async () => {
    setBusy(true)
    try {
      await api.save_store_item({ id: item?.id ?? null, name, price_paise: Math.round(Number(price || '0') * 100), low_stock_at: Number(low || '0'), active })
      onSaved()
    } finally {
      setBusy(false)
    }
  }
  const label: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '400px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '24px' }}>{item ? t('store.editItem') : t('store.addItem')}</div>
        <div style={{ padding: '8px 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
          <label style={label}>{t('store.itemName')}<input value={name} onChange={(e) => setName(e.target.value)} style={field()} /></label>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
            <label style={label}>{t('store.price')}<input inputMode="decimal" value={price} onChange={(e) => setPrice(e.target.value.replace(/[^0-9.]/g, ''))} style={field()} /></label>
            <label style={label}>{t('store.lowAt')}<input inputMode="numeric" value={low} onChange={(e) => setLow(e.target.value.replace(/[^0-9]/g, ''))} style={field()} /></label>
          </div>
          {item ? <label style={{ display: 'flex', gap: '8px', alignItems: 'center', fontSize: '14px' }}><input type="checkbox" checked={active} onChange={(e) => setActive(e.target.checked)} />{t('store.active')}</label> : null}
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('store.cancel')}</button>
          <button type="button" disabled={busy || name.trim() === ''} style={{ ...primaryBtn(), opacity: busy || name.trim() === '' ? 0.5 : 1 }} onClick={save}>{t('store.save')}</button>
        </div>
      </div>
    </div>
  )
}

function StockSheet({ item, onClose, onSaved }: { item: StoreItemDto; onClose: () => void; onSaved: () => void }) {
  const [delta, setDelta] = useState('')
  const [reason, setReason] = useState('purchase')
  const [busy, setBusy] = useState(false)
  const save = async () => {
    setBusy(true)
    try {
      const d = Number(delta || '0')
      await api.stock_adjust({ item_id: item.id, delta_qty: reason === 'adjust' ? d : Math.abs(d), reason })
      onSaved()
    } finally {
      setBusy(false)
    }
  }
  const label: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '380px', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden' }}>
        <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '22px' }}>{item.name} · {t('store.stock')}</div>
        <div style={{ padding: '8px 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
          <div style={{ fontSize: '13px', color: 'var(--muted)' }}>{t('store.currentStock', { n: String(item.stock) })}</div>
          <div style={{ display: 'flex', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
            {['purchase', 'adjust'].map((r) => (
              <button key={r} type="button" style={{ flex: 1, height: '40px', border: 'none', borderRight: '1px solid var(--line-strong)', background: reason === r ? 'var(--accent)' : 'var(--surface)', color: reason === r ? 'var(--white)' : 'var(--ink)', fontSize: '13px', cursor: 'pointer' }} onClick={() => setReason(r)}>{t(`store.reason.${r}`)}</button>
            ))}
          </div>
          <label style={label}>{reason === 'adjust' ? t('store.deltaAdjust') : t('store.deltaPurchase')}<input inputMode="numeric" value={delta} onChange={(e) => setDelta(e.target.value.replace(/[^0-9-]/g, ''))} style={field()} /></label>
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
          <button type="button" style={secondaryBtn()} onClick={onClose}>{t('store.cancel')}</button>
          <button type="button" disabled={busy || delta === ''} style={{ ...primaryBtn(), opacity: busy || delta === '' ? 0.5 : 1 }} onClick={save}>{t('store.save')}</button>
        </div>
      </div>
    </div>
  )
}
