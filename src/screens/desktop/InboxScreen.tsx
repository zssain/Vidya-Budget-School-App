import { useEffect, useState } from 'react'
import { PageTitle } from '@/components/desktop/PageTitle'
import { Pill } from '@/components/desktop/Pill'
import { navigate } from '@/lib/router'
import { useStore } from '@/lib/store'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'

// Inbox (prompts/P07 Shared). Notifications aren't yet emitted into the
// notification table, so this pragmatically surfaces the caller's decided
// requests + (for the Principal) open review flags & conflicts, with deep links.
// Documented as a follow-up: a dedicated notification feed.
interface Item {
  kind: string
  title: string
  sub: string
  link?: string
}

export default function InboxScreen({ role }: { role: string }) {
  const store = useStore()
  const myId = store.app?.state.kind === 'unlocked' ? store.app.state.staff.id : ''
  const [items, setItems] = useState<Item[]>([])

  useEffect(() => {
    const collected: Item[] = []
    const jobs: Promise<void>[] = []
    jobs.push(
      api.list_requests().then((rs) => {
        for (const r of rs) {
          if (r.requested_by === myId && (r.status === 'approved' || r.status === 'rejected' || r.status === 'returned')) {
            collected.push({ kind: 'decision', title: t('inbox.decision'), sub: `${r.kind} · ${r.status}` })
          }
        }
      }).catch(() => {}),
    )
    if (role === 'principal') {
      jobs.push(api.list_review_flags().then((fs) => { for (const f of fs) collected.push({ kind: 'flag', title: t('inbox.flag'), sub: `${f.kind} · ${f.ref_table}`, link: '/principal/conflicts' }) }).catch(() => {}))
      jobs.push(api.list_conflicts().then((cs) => { for (const c of cs) collected.push({ kind: 'conflict', title: t('inbox.conflict'), sub: `${c.table} · ${c.field}`, link: '/principal/conflicts' }) }).catch(() => {}))
    }
    Promise.all(jobs).then(() => setItems(collected))
  }, [myId, role])

  return (
    <div style={{ padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: '20px', minHeight: '100%', boxSizing: 'border-box' }}>
      <PageTitle eyebrow={t('inbox.eyebrow')} title={t('inbox.title')} sub={t('inbox.sub')} />
      <div style={{ background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: '16px', overflow: 'hidden' }}>
        {items.length === 0 ? (
          <div style={{ padding: '40px 24px', textAlign: 'center', color: 'var(--muted)' }}>{t('inbox.none')}</div>
        ) : (
          items.map((it, i) => (
            <button key={i} type="button" onClick={() => it.link && navigate(it.link)} style={{ display: 'flex', width: '100%', textAlign: 'left', alignItems: 'center', gap: '16px', padding: '14px 24px', border: 'none', borderTop: i > 0 ? '1px solid var(--track)' : 'none', background: 'transparent', color: 'inherit', cursor: it.link ? 'pointer' : 'default' }}>
              <span style={{ flex: 1, minWidth: 0 }}>
                <span style={{ display: 'block', fontWeight: 500 }}>{it.title}</span>
                <span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>{it.sub}</span>
              </span>
              <Pill variant={it.kind === 'flag' || it.kind === 'conflict' ? 'unpaid' : 'neutral'}>{it.kind}</Pill>
            </button>
          ))
        )}
      </div>
    </div>
  )
}
