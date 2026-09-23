import { useCallback, useEffect, useState } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import type { PillVariant } from '@/components/desktop/Pill'
import { useStore } from '@/lib/store'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { RequestDto } from '@/lib/api'

// My requests (prompts/P07 Shared): the caller's own requests with status pills,
// cancel while pending/returned.
function variantFor(status: string): PillVariant {
  switch (status) {
    case 'approved':
      return 'paid'
    case 'rejected':
      return 'unpaid'
    case 'returned':
      return 'partpaid'
    case 'pending':
      return 'attendance'
    default:
      return 'neutral'
  }
}

export default function MyRequestsScreen() {
  const store = useStore()
  const myId = store.app?.state.kind === 'unlocked' ? store.app.state.staff.id : ''
  const [rows, setRows] = useState<RequestDto[]>([])
  const [toast, setToast] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_requests().then((rs) => setRows(rs.filter((r) => r.requested_by === myId))).catch(() => setRows([]))
  }, [myId])
  useEffect(load, [load])

  const cancel = async (id: string) => {
    try {
      await api.cancel_request(id)
      setToast(t('req.cancelled'))
      window.setTimeout(() => setToast(null), 2200)
      load()
    } catch {
      /* ignore */
    }
  }

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle eyebrow={t('req.eyebrow')} title={t('req.title')} sub={t('req.sub')} />
      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        {rows.length === 0 ? (
          <div style={{ padding: '40px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('req.none')}</div>
        ) : (
          rows.map((r, i) => (
            <div key={r.id} style={{ display: 'flex', alignItems: 'center', gap: '16px', padding: '14px 24px', borderTop: i > 0 ? '1px solid var(--track)' : 'none' }}>
              <span style={{ flex: 1, minWidth: 0 }}>
                <span style={{ display: 'block', fontWeight: 500 }}>{r.kind}</span>
                <span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>{r.reason}</span>
              </span>
              <Pill variant={variantFor(r.status)}>{t(`req.status.${r.status}`)}</Pill>
              {r.status === 'pending' || r.status === 'returned' ? (
                <button type="button" onClick={() => cancel(r.id)} style={{ height: '34px', padding: '0 12px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', cursor: 'pointer' }}>{t('req.cancel')}</button>
              ) : null}
            </div>
          ))
        )}
      </div>
      {toast ? <div role="status" style={{ position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)', zIndex: 30, background: 'var(--navy)', color: 'var(--white)', borderRadius: 10, padding: '12px 18px', fontSize: 14 }}>{toast}</div> : null}
    </div>
  )
}
