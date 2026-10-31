// Refused changes on the phone (repair B1). The school PC's ack can refuse an op
// (permission, locked sheet, validation…); the PWA keeps it in the outbox with the
// reason and `list_sync_problems` returns it — the same command the desktop Inbox
// uses. Two views: a card listing each refused change (About screen) and a notice
// on Teacher home that only appears when there is at least one (tap → About).
// All copy via t() (strings/syncProblems.ts); colours via CSS tokens.
import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { SyncProblemDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { describeSyncProblem } from '@/lib/syncProblems'

function useSyncProblems(): SyncProblemDto[] | null {
  const [problems, setProblems] = useState<SyncProblemDto[] | null>(null)
  useEffect(() => {
    api.list_sync_problems().then(setProblems).catch(() => setProblems([]))
  }, [])
  return problems
}

const CARD: React.CSSProperties = {
  background: 'var(--surface)',
  border: '1px solid var(--line)',
  borderRadius: 16,
  padding: '4px 18px 14px',
}

/** The list of refused changes (or "all accepted"). */
export function SyncProblemsCard() {
  const problems = useSyncProblems()
  if (problems === null) return null
  return (
    <div style={CARD} data-testid="sync-problems">
      <h2 style={{ fontSize: 15, fontWeight: 600, margin: '16px 0 6px' }}>{t('syncp.title')}</h2>
      {problems.length === 0 ? (
        <p style={{ margin: '0 0 4px', fontSize: 13, color: 'var(--muted)' }}>{t('syncp.none')}</p>
      ) : (
        <>
          <p style={{ margin: '0 0 4px', fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{t('syncp.intro')}</p>
          {problems.map((p) => {
            const { what, why } = describeSyncProblem(p, t)
            return (
              <div key={p.id} style={{ display: 'flex', gap: 12, padding: '14px 0', borderTop: '1px solid var(--track)' }}>
                <span style={{ marginTop: 2, color: 'var(--pill-unpaid-fg)', flexShrink: 0 }}>
                  <Icon name="inbox" size={18} strokeWidth={1.8} />
                </span>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}>
                  <span style={{ fontSize: 14, fontWeight: 600, color: 'var(--ink)' }}>{what}</span>
                  <span style={{ fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{why}</span>
                </div>
              </div>
            )
          })}
        </>
      )}
    </div>
  )
}

/** A Teacher-home notice, shown only when something was refused (tap → About). */
export function SyncProblemsBanner() {
  const problems = useSyncProblems()
  if (!problems || problems.length === 0) return null
  return (
    <button
      type="button"
      className="v-active-96"
      onClick={() => navigate('/teacher/about')}
      style={{ display: 'flex', alignItems: 'center', gap: 14, width: '100%', textAlign: 'left', padding: '14px 16px', borderRadius: 16, border: '1px solid var(--pill-unpaid-fg)', background: 'var(--pill-unpaid-bg)', color: 'var(--pill-unpaid-fg)' }}
    >
      <Icon name="inbox" size={20} strokeWidth={1.7} />
      <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0 }}>
        <b style={{ fontWeight: 600, fontSize: 14 }}>{t('syncp.banner', { n: problems.length })}</b>
        <span style={{ fontSize: 12 }}>{t('syncp.bannerSub')}</span>
      </span>
      <Icon name="chevronRight" size={18} strokeWidth={1.6} />
    </button>
  )
}
