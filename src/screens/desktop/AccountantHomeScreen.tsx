import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { navigate } from '@/lib/router'
import { useStore } from '@/lib/store'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { AccountantDashboard, ReceiptSummaryDto, RequestDto } from '@/lib/api'

// Accountant home (prompts/P07 Accountant): collected today, receipts today,
// waiting to send, my pending requests, recent receipts. Reuses existing reads.
function statCell(i: number): CSSProperties {
  return { display: 'flex', flexDirection: 'column', gap: '8px', padding: '22px 24px', ...(i > 0 ? { borderLeft: '1px solid var(--line-stat)' } : {}) }
}

export default function AccountantHomeScreen() {
  const store = useStore()
  const myId = store.app?.state.kind === 'unlocked' ? store.app.state.staff.id : ''
  const [dash, setDash] = useState<AccountantDashboard | null>(null)
  const [receipts, setReceipts] = useState<ReceiptSummaryDto[]>([])
  const [pending, setPending] = useState<RequestDto[]>([])

  useEffect(() => {
    api.dashboard_accountant().then(setDash).catch(() => setDash(null))
    api.search_receipts('').then((r) => setReceipts(r.slice(0, 8))).catch(() => setReceipts([]))
    api.list_requests('pending').then((rs) => setPending(rs.filter((r) => r.requested_by === myId))).catch(() => setPending([]))
  }, [myId])

  const stats = dash
    ? [
        { label: t('acc.collectedToday'), value: formatMoney(dash.collected_today_paise) },
        { label: t('acc.receiptsToday'), value: String(dash.receipts_today) },
        { label: t('acc.outstanding'), value: formatMoney(dash.outstanding_paise) },
        { label: t('acc.withDues'), value: String(dash.students_with_dues) },
      ]
    : []

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '24px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle
        eyebrow={t('acc.eyebrow')}
        title={t('acc.title')}
        sub={t('acc.sub')}
        actions={
          <button type="button" onClick={() => navigate('/accountant/collect')} style={{ height: '44px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '14px', fontWeight: 500, cursor: 'pointer' }}>{t('acc.collectFee')}</button>
        }
      />

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', background: 'var(--panel)', borderRadius: '16px' }}>
        {stats.map((s, i) => (
          <div key={s.label} style={statCell(i)}>
            <span style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: '34px', lineHeight: 1, letterSpacing: '-0.02em', color: 'var(--ink)', fontVariantNumeric: 'tabular-nums' }}>{s.value}</span>
            <span style={{ fontSize: '13px', color: 'var(--muted)' }}>{s.label}</span>
          </div>
        ))}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: '20px', alignItems: 'start' }}>
        <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
          <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--track)', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('acc.recentReceipts')}</div>
          {receipts.map((r, i) => (
            <button key={r.id} type="button" onClick={() => navigate('/accountant/receipts')} style={{ display: 'flex', justifyContent: 'space-between', width: '100%', textAlign: 'left', padding: '12px 24px', border: 'none', borderTop: i > 0 ? '1px solid var(--track)' : 'none', background: 'transparent', color: 'inherit', cursor: 'pointer', fontSize: '13px' }}>
              <span>{r.receipt_no} · {r.student_name}</span>
              <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatMoney(r.amount_paise)}</span>
            </button>
          ))}
        </div>
        <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
          <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--track)', fontSize: '10px', fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('acc.myPending')} · {pending.length}</div>
          {pending.length === 0 ? (
            <div style={{ padding: '20px 24px', color: 'var(--muted)', fontSize: '13px' }}>—</div>
          ) : (
            pending.map((r, i) => (
              <div key={r.id} style={{ padding: '12px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none', fontSize: '13px' }}>{r.kind} · {r.reason}</div>
            ))
          )}
        </div>
      </div>
    </div>
  )
}
