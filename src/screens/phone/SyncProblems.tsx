// Refused changes on the phone (repair B1). The school PC's ack can refuse an op
// (permission, locked sheet, validation…); the PWA keeps it in the outbox with the
// reason and `list_sync_problems` returns it — the same command the desktop Inbox
// uses. Two views: a card listing each refused change (About screen) and a notice
// on Teacher home that only appears when there is at least one (tap → About).
// "Dismiss" removes a read item (dismiss_sync_problem); nothing is resent — the
// school PC would refuse the same change again, so the user re-enters it if needed.
// All copy via t() (strings/syncProblems.ts); colours via CSS tokens.
import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { SyncProblemDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { navigate } from '@/lib/router'
import { describeSyncProblem } from '@/lib/syncProblems'

function useSyncProblems(): [SyncProblemDto[] | null, () => void] {
  const [problems, setProblems] = useState<SyncProblemDto[] | null>(null)
  const [gen, setGen] = useState(0)
  useEffect(() => {
    api.list_sync_problems().then(setProblems).catch(() => setProblems([]))
  }, [gen])
  return [problems, () => setGen((g) => g + 1)]
}

const CARD: React.CSSProperties = {
  background: 'var(--surface)',
  border: '1px solid var(--line)',
  borderRadius: 16,
  padding: '4px 18px 14px',
}

/** The list of refused changes (or "all accepted"). */
export function SyncProblemsCard() {
  const [problems, reload] = useSyncProblems()
  const [busy, setBusy] = useState<string | null>(null)
  const dismiss = (id: string) => {
    setBusy(id)
    api
      .dismiss_sync_problem(id)
      .catch(() => undefined)
      .finally(() => {
        setBusy(null)
        reload()
      })
  }
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
                <div style={{ display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0, flexGrow: 1 }}>
                  <span style={{ fontSize: 14, fontWeight: 600, color: 'var(--ink)' }}>{what}</span>
                  <span style={{ fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{why}</span>
                </div>
                <button
                  type="button"
                  className="v-active-96"
                  onClick={() => dismiss(p.id)}
                  disabled={busy === p.id}
                  aria-label={t('syncp.dismissAria', { what })}
                  style={{ alignSelf: 'center', flexShrink: 0, minHeight: 36, padding: '0 12px', borderRadius: 10, border: '1px solid var(--line)', background: 'var(--surface)', color: 'var(--ink)', fontSize: 13, fontWeight: 600 }}
                >
                  {t('syncp.dismiss')}
                </button>
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
  const [problems] = useSyncProblems()
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
