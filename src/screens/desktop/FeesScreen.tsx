import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { FeeHeadDto, FeeHeadInput, FeeOverviewRow, Instalment, InstalmentPlan } from '@/lib/api'
import FeesDues from './FeesDues'

const MONTHS_SHORT = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
/** "2026-08-15" → "15 Aug" (chip label). */
export function formatDueShort(d: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(d)
  if (!m) return d
  return `${Number(m[3])} ${MONTHS_SHORT[Number(m[2]) - 1] ?? m[2]}`
}
function parsePlan(json: string | null): InstalmentPlan | null {
  if (!json) return null
  try {
    return JSON.parse(json) as InstalmentPlan
  } catch {
    return null
  }
}

// Fees (prompts/P07 §9): overview by class + fee-structure heads CRUD + Dues (P14).
// Structure edits are Principal-only; a head in use can only be deactivated; an
// amount change previews the affected unpaid dues before applying.

type Tab = 'overview' | 'dues' | 'structure'

function seg(sel: boolean): CSSProperties {
  return { minWidth: '64px', padding: '0 16px', height: '38px', fontSize: '13px', fontWeight: 500, border: 'none', borderRight: '1px solid var(--line-strong)', background: sel ? 'var(--accent)' : 'transparent', color: sel ? 'var(--white)' : 'var(--ink)', cursor: 'pointer' }
}
function field(): CSSProperties {
  return { height: '44px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--white)', color: 'var(--ink)', fontSize: '14px', fontFamily: 'inherit', padding: '0 12px', boxSizing: 'border-box' }
}
function primaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }
}
function secondaryBtn(): CSSProperties {
  return { height: '40px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }
}
const CARD: CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }

export default function FeesScreen() {
  const [tab, setTab] = useState<Tab>('overview')
  const [overview, setOverview] = useState<FeeOverviewRow[]>([])
  const [heads, setHeads] = useState<FeeHeadDto[]>([])
  const [editing, setEditing] = useState<FeeHeadDto | 'new' | null>(null)

  const loadOverview = useCallback(() => {
    api.fees_overview().then(setOverview).catch(() => setOverview([]))
  }, [])
  const loadHeads = useCallback(() => {
    api.list_fee_heads().then(setHeads).catch(() => setHeads([]))
  }, [])
  useEffect(() => {
    loadOverview()
    loadHeads()
  }, [loadOverview, loadHeads])

  const totalOutstanding = overview.reduce((n, r) => n + r.outstanding_paise, 0)

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('fees.eyebrow')}
        title={t('fees.title')}
        sub={t('fees.sub')}
        actions={
          <div style={{ display: 'inline-flex', gap: '10px' }}>
            <button type="button" style={secondaryBtn()} onClick={() => navigate('/principal/daybook')}>{t('fees.daybook')}</button>
            <button type="button" style={secondaryBtn()} onClick={() => navigate('/principal/reports')}>{t('fees.reports')}</button>
          </div>
        }
      />

      <div style={{ display: 'inline-flex', alignSelf: 'flex-start', border: '1px solid var(--line-strong)', borderRadius: '6px', overflow: 'hidden' }}>
        <button type="button" style={seg(tab === 'overview')} onClick={() => setTab('overview')}>{t('fees.tab.overview')}</button>
        <button type="button" style={seg(tab === 'dues')} onClick={() => setTab('dues')}>{t('fees.tab.dues')}</button>
        <button type="button" style={seg(tab === 'structure')} onClick={() => setTab('structure')}>{t('fees.tab.structure')}</button>
      </div>

      {tab === 'dues' ? (
        <FeesDues />
      ) : tab === 'overview' ? (
        <div style={CARD}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: '16px', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
            <span>{t('fees.col.class')}</span>
            <span style={{ textAlign: 'right' }}>{t('fees.col.withDues')}</span>
            <span style={{ textAlign: 'right' }}>{t('fees.col.outstanding')}</span>
          </div>
          {overview.length === 0 ? (
            <div style={{ padding: '48px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('fees.overviewEmpty')}</div>
          ) : (
            overview.map((r, i) => (
              <button key={r.class_id} type="button" onClick={() => navigate('/principal/students')} style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: '16px', width: '100%', padding: '14px 24px', border: 'none', borderTop: i > 0 ? '1px solid var(--track)' : 'none', background: 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '14px', textAlign: 'left' }}>
                <span style={{ fontWeight: 500 }}>{r.class_display}</span>
                <span style={{ textAlign: 'right', color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{r.students_with_dues}</span>
                <span style={{ textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.outstanding_paise)}</span>
              </button>
            ))
          )}
          <div style={{ display: 'flex', justifyContent: 'space-between', padding: '14px 24px', borderTop: '1px solid var(--track)', background: 'var(--panel)', fontSize: '14px' }}>
            <span style={{ fontWeight: 600 }}>{t('fees.totalOutstanding')}</span>
            <span style={{ fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>{formatMoney(totalOutstanding)}</span>
          </div>
        </div>
      ) : (
        <>
          <button type="button" style={{ ...primaryBtn(), alignSelf: 'flex-start' }} onClick={() => setEditing('new')}>{t('fees.addHead')}</button>
          <div style={CARD}>
            <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr 2.6fr 1.4fr', gap: '16px', padding: '12px 24px', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)', borderBottom: '1px solid var(--track)' }}>
              <span>{t('fees.col.head')}</span>
              <span style={{ textAlign: 'right' }}>{t('fees.col.amount')}</span>
              <span>{t('fees.col.instalments')}</span>
              <span style={{ textAlign: 'right' }}>{t('fees.col.status')}</span>
            </div>
            {heads.map((h, i) => (
              <div key={h.id} style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr 2.6fr 1.4fr', gap: '16px', alignItems: 'center', padding: '14px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '14px' }}>
                <span style={{ fontWeight: 500 }}>{h.name}</span>
                <span style={{ textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>{formatMoney(h.amount_paise)}</span>
                <InstalmentChips plan={parsePlan(h.instalments_json)} freq={h.frequency} />
                <span style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end', alignItems: 'center' }}>
                  {h.active ? <Pill variant="paid">{t('fees.active')}</Pill> : <Pill variant="neutral">{t('fees.inactive')}</Pill>}
                  <button type="button" style={secondaryBtn()} onClick={() => setEditing(h)}>{t('fees.edit')}</button>
                </span>
              </div>
            ))}
          </div>
        </>
      )}

      {editing != null ? (
        <HeadEditor
          head={editing === 'new' ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => { setEditing(null); loadHeads(); loadOverview() }}
        />
      ) : null}
    </div>
  )
}

/** The "Instalments and due dates" cell in the Fee-structure table (prototype
 *  feesadmin state 1): teal chips per instalment, or a sentence for a monthly
 *  rule, or a muted "One due" for a plan-less head. */
function InstalmentChips({ plan, freq }: { plan: InstalmentPlan | null; freq: string }) {
  if (plan?.type === 'list') {
    return (
      <span style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }} data-hl="instalments">
        {plan.instalments.map((it) => (
          <span key={it.no} style={{ display: 'inline-flex', gap: '6px', alignItems: 'baseline', padding: '3px 9px', borderRadius: '999px', background: 'var(--accent-12)', color: 'var(--accent)', fontSize: '12px', fontWeight: 500, fontVariantNumeric: 'tabular-nums' }}>
            <span>{formatDueShort(it.due_date)}</span>
            <span>{formatMoney(it.amount_paise)}</span>
          </span>
        ))}
      </span>
    )
  }
  if (plan?.type === 'monthly') {
    return <span style={{ color: 'var(--muted)' }}>{t('fees.inst.monthlyOn', { day: String(plan.day_of_month) })}</span>
  }
  return <span style={{ color: 'var(--muted)' }}>{t(`fees.freq.${freq}`)}</span>
}

type ListItem = { rupees: string; due: string }
type PlanMode = 'none' | 'list' | 'monthly'

function initialPlan(head: FeeHeadDto | null): { mode: PlanMode; items: ListItem[]; monthlyRupees: string; monthlyDay: string } {
  const plan = parsePlan(head?.instalments_json ?? null)
  if (plan?.type === 'list') {
    return { mode: 'list', items: plan.instalments.map((it) => ({ rupees: String(it.amount_paise / 100), due: it.due_date })), monthlyRupees: '', monthlyDay: '10' }
  }
  if (plan?.type === 'monthly') {
    return { mode: 'monthly', items: [], monthlyRupees: String(plan.monthly_amount_paise / 100), monthlyDay: String(plan.day_of_month) }
  }
  return { mode: 'none', items: [{ rupees: '', due: '' }], monthlyRupees: '', monthlyDay: '10' }
}

function HeadEditor({ head, onClose, onSaved }: { head: FeeHeadDto | null; onClose: () => void; onSaved: () => void }) {
  const init = initialPlan(head)
  const [f, setF] = useState<FeeHeadInput>({
    name: head?.name ?? '',
    name_hi: head?.name_hi ?? '',
    amount_paise: head?.amount_paise ?? 0,
    frequency: head?.frequency ?? 'term',
    applies_to: head?.applies_to ?? 'all',
  })
  const [rupees, setRupees] = useState(String((head?.amount_paise ?? 0) / 100))
  const [mode, setMode] = useState<PlanMode>(init.mode)
  const [items, setItems] = useState<ListItem[]>(init.items)
  const [monthlyRupees, setMonthlyRupees] = useState(init.monthlyRupees)
  const [monthlyDay, setMonthlyDay] = useState(init.monthlyDay)
  const [preview, setPreview] = useState<{ n: number; delta: number } | null>(null)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const toPaise = (r: string) => Math.round(Number(r || '0') * 100)
  const listSum = items.reduce((n, it) => n + toPaise(it.rupees), 0)
  // The head's total amount: sum of instalments (list), the monthly amount
  // (monthly rule), or the manual field (plan-less).
  const amountPaise = mode === 'list' ? listSum : mode === 'monthly' ? toPaise(monthlyRupees) : toPaise(rupees)

  const instalmentsJson = (): string | null => {
    if (mode === 'list') {
      const instalments: Instalment[] = items
        .filter((it) => it.rupees !== '' && it.due !== '')
        .map((it, i) => ({ no: i + 1, amount_paise: toPaise(it.rupees), due_date: it.due }))
      if (instalments.length === 0) return null
      return JSON.stringify({ type: 'list', instalments } satisfies InstalmentPlan)
    }
    if (mode === 'monthly') {
      return JSON.stringify({ type: 'monthly', monthly_amount_paise: toPaise(monthlyRupees), day_of_month: Number(monthlyDay || '1') } satisfies InstalmentPlan)
    }
    return null
  }

  // Preview which unpaid dues change (amount change or plan change) — Step 1.
  const planKey = JSON.stringify({ amountPaise, json: instalmentsJson() })
  useEffect(() => {
    if (!head) {
      setPreview(null)
      return
    }
    const json = instalmentsJson()
    if (json === null && amountPaise === head.amount_paise) {
      setPreview(null)
      return
    }
    let live = true
    api.preview_fee_head_change(head.id, amountPaise, json)
      .then((p) => { if (live) setPreview({ n: p.affected_dues, delta: p.delta_paise }) })
      .catch(() => setPreview(null))
    return () => { live = false }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [head, planKey])

  const save = async () => {
    setErr(null)
    setBusy(true)
    const input: FeeHeadInput = { ...f, amount_paise: amountPaise, instalments_json: instalmentsJson() }
    try {
      if (head) await api.update_fee_head(head.id, input)
      else await api.create_fee_head(input)
      onSaved()
    } catch (e) {
      setErr(t('fees.saveError'))
    } finally {
      setBusy(false)
    }
  }

  const label: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px', color: 'var(--muted)' }
  const smallField: CSSProperties = { ...field(), height: '38px' }

  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(11,26,51,0.45)', display: 'grid', placeItems: 'center', zIndex: 40 }}>
      <div onClick={(e) => e.stopPropagation()} style={{ width: '480px', maxHeight: 'calc(100vh - 48px)', background: 'var(--surface)', borderRadius: '20px', boxShadow: '0 30px 60px rgba(11,26,51,0.3)', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '20px 24px 8px', fontFamily: 'var(--font-serif)', fontSize: '24px', color: 'var(--ink)' }}>{head ? t('fees.editHead') : t('fees.newHead')}</div>
        <div style={{ padding: '8px 24px 20px', display: 'flex', flexDirection: 'column', gap: '14px', overflowY: 'auto' }}>
          <label style={label}>{t('fees.head.name')}<input value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} style={field()} /></label>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
            <label style={label}>{t('fees.head.amount')}
              <input inputMode="decimal" value={mode === 'none' ? rupees : String(amountPaise / 100)} disabled={mode !== 'none'} onChange={(e) => setRupees(e.target.value.replace(/[^0-9.]/g, ''))} style={{ ...field(), opacity: mode === 'none' ? 1 : 0.6 }} />
            </label>
            <label style={label}>{t('fees.head.frequency')}
              <select value={f.frequency} onChange={(e) => setF({ ...f, frequency: e.target.value })} style={field()}>
                <option value="term">{t('fees.freq.term')}</option>
                <option value="month">{t('fees.freq.month')}</option>
                <option value="once">{t('fees.freq.once')}</option>
              </select>
            </label>
          </div>
          <label style={label}>{t('fees.head.appliesTo')}
            <select value={f.applies_to} onChange={(e) => setF({ ...f, applies_to: e.target.value })} style={field()}>
              <option value="all">{t('fees.applies.all')}</option>
              <option value="transport">{t('fees.applies.transport')}</option>
            </select>
          </label>

          {/* Instalment plan (§10.2). */}
          <label style={label}>{t('fees.head.plan')}
            <select value={mode} onChange={(e) => setMode(e.target.value as PlanMode)} style={field()}>
              <option value="none">{t('fees.plan.none')}</option>
              <option value="list">{t('fees.plan.list')}</option>
              <option value="monthly">{t('fees.plan.monthly')}</option>
            </select>
          </label>

          {mode === 'list' ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {items.map((it, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: '28px 1fr 1.2fr 32px', gap: '8px', alignItems: 'center' }}>
                  <span style={{ fontSize: '12px', color: 'var(--muted)' }}>{i + 1}.</span>
                  <input inputMode="decimal" placeholder={t('fees.inst.amount')} value={it.rupees} onChange={(e) => setItems(items.map((x, j) => j === i ? { ...x, rupees: e.target.value.replace(/[^0-9.]/g, '') } : x))} style={smallField} />
                  <input type="date" value={it.due} onChange={(e) => setItems(items.map((x, j) => j === i ? { ...x, due: e.target.value } : x))} style={smallField} />
                  <button type="button" aria-label={t('fees.inst.remove')} style={{ ...secondaryBtn(), height: '38px', padding: '0 10px' }} onClick={() => setItems(items.filter((_, j) => j !== i))}>×</button>
                </div>
              ))}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <button type="button" style={secondaryBtn()} onClick={() => setItems([...items, { rupees: '', due: '' }])}>{t('fees.inst.add')}</button>
                <span style={{ fontSize: '13px', color: 'var(--muted)', fontVariantNumeric: 'tabular-nums' }}>{t('fees.inst.sum', { total: formatMoney(listSum) })}</span>
              </div>
            </div>
          ) : null}

          {mode === 'monthly' ? (
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
              <label style={label}>{t('fees.inst.monthlyAmount')}<input inputMode="decimal" value={monthlyRupees} onChange={(e) => setMonthlyRupees(e.target.value.replace(/[^0-9.]/g, ''))} style={field()} /></label>
              <label style={label}>{t('fees.inst.dayOfMonth')}<input inputMode="numeric" value={monthlyDay} onChange={(e) => setMonthlyDay(e.target.value.replace(/[^0-9]/g, ''))} style={field()} /></label>
            </div>
          ) : null}

          {preview ? (
            <div style={{ fontSize: '13px', color: preview.n > 0 ? 'var(--gold-text)' : 'var(--muted)', background: 'var(--unmarked)', border: '1px solid var(--gold-line)', borderRadius: '8px', padding: '10px 12px' }}>
              {preview.n > 0 ? t('fees.preview', { n: String(preview.n), delta: formatMoney(preview.delta) }) : t('fees.previewNone')}
            </div>
          ) : null}
          {err ? <div style={{ fontSize: '13px', color: 'var(--danger)' }}>{err}</div> : null}
          {head?.has_allocations ? <div style={{ fontSize: '12px', color: 'var(--muted)' }}>{t('fees.hasAllocations')}</div> : null}
        </div>
        <div style={{ background: 'var(--panel)', padding: '14px 24px', display: 'flex', gap: '10px', justifyContent: 'space-between' }}>
          <div>
            {head && head.active ? (
              <button type="button" style={secondaryBtn()} onClick={async () => { await api.deactivate_fee_head(head.id); onSaved() }}>{t('fees.deactivate')}</button>
            ) : null}
          </div>
          <div style={{ display: 'flex', gap: '10px' }}>
            <button type="button" style={secondaryBtn()} onClick={onClose}>{t('fees.cancel')}</button>
            <button type="button" style={{ ...primaryBtn(), opacity: busy || f.name.trim() === '' ? 0.5 : 1 }} disabled={busy || f.name.trim() === ''} onClick={save}>
              {preview && preview.n > 0 ? t('fees.applyChange') : t('fees.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
