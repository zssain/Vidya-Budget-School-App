// Inbox (repair T1): the circulars sent to this teacher (list_circulars; opening an
// unread one marks it read — mark_circular_read) and the changes the school
// computer did not accept (list_sync_problems, the same card the iPhone About screen
// shows). The Teacher Home bell + Inbox badge count exactly these.
import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { CircularDto } from '@/lib/api'
import { Icon } from '@/components/Icon'
import PhoneScreen, { PHONE_CARD, PhoneEmpty, PhonePill } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { formatDateShortIn } from '@/lib/format'
import { SyncProblemsCard } from './SyncProblems'


export default function PhoneInboxScreen() {
  const lang = useLang()
  const [rows, setRows] = useState<CircularDto[] | null>(null)
  const [open, setOpen] = useState<string | null>(null)

  const load = useCallback(() => {
    api.list_circulars().then(setRows).catch(() => setRows([]))
  }, [])
  useEffect(load, [load])

  const toggle = (c: CircularDto) => {
    setOpen((cur) => (cur === c.id ? null : c.id))
    if (!c.read_by_me) {
      // Optimistic: the row stops being "New" now; the read receipt syncs.
      setRows((cur) => (cur ?? []).map((x) => (x.id === c.id ? { ...x, read_by_me: true } : x)))
      api.mark_circular_read(c.id).catch(() => load())
    }
  }

  return (
    <PhoneScreen title={t('tp.inbox.title')}>
      <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--muted)' }}>{t('tp.inbox.circulars')}</span>
      {rows === null ? (
        <PhoneEmpty>{t('tp.loading')}</PhoneEmpty>
      ) : rows.length === 0 ? (
        <PhoneEmpty>{t('tp.inbox.noCirculars')}</PhoneEmpty>
      ) : (
        <div style={PHONE_CARD}>
          {rows.map((c, i) => (
            <div key={c.id} style={{ borderTop: i === 0 ? 0 : '1px solid var(--track)' }}>
              <button
                type="button"
                aria-expanded={open === c.id}
                onClick={() => toggle(c)}
                style={{ display: 'flex', alignItems: 'center', gap: 12, width: '100%', padding: '14px 16px', border: 0, background: 'transparent', color: 'inherit', textAlign: 'left' }}
              >
                <span style={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <b style={{ fontWeight: c.read_by_me ? 500 : 700, fontSize: 15 }}>{c.title}</b>
                  <span style={{ fontSize: 12, color: 'var(--muted)' }}>
                    {[c.number, formatDateShortIn(c.sent_at ?? c.created_at, lang)].filter(Boolean).join(' · ')}
                  </span>
                </span>
                {!c.read_by_me ? <PhonePill tone="warn">{t('tp.inbox.new')}</PhonePill> : null}
                <Icon name="chevronRight" size={18} strokeWidth={1.6} />
              </button>
              {open === c.id ? (
                <div style={{ padding: '0 16px 16px', fontSize: 14, lineHeight: 1.55, whiteSpace: 'pre-wrap', color: 'var(--ink)' }}>{c.body}</div>
              ) : null}
            </div>
          ))}
        </div>
      )}
      <SyncProblemsCard />
    </PhoneScreen>
  )
}
