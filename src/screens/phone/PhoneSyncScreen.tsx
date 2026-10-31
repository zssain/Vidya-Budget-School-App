// Sync (repair T1): this phone's real sync state (sync_status — changes still
// waiting to reach the school computer, and when it last synced) and "Sync now"
// (sync_now). Natively sync_now really runs a pass and waits for it (up to ~30 s);
// on the iPhone app it runs one pass of the foreground Google Drive loop. The button
// shows busy until the pass ends and is hidden where sync_now is not offered.
import { useCallback, useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { SyncStatusDto } from '@/lib/api'
import PhoneScreen, { PHONE_CARD, PHONE_PRIMARY, PhoneEmpty } from '@/components/PhoneScreen'
import { t, useLang } from '@/lib/i18n'
import { isWeb } from '@/lib/platform'
import { isAvailableOnWeb } from '@/lib/web/backend'
import { agoText } from '@/lib/teacherHome'

function Row({ label, value, first }: { label: string; value: string; first?: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '14px 16px', borderTop: first ? 0 : '1px solid var(--track)', fontSize: 15 }}>
      <span style={{ color: 'var(--muted)' }}>{label}</span>
      <b style={{ fontWeight: 600, textAlign: 'right' }}>{value}</b>
    </div>
  )
}

export default function PhoneSyncScreen() {
  useLang()
  const [s, setS] = useState<SyncStatusDto | null>(null)
  const [failed, setFailed] = useState(false)
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const canSyncNow = !isWeb || isAvailableOnWeb('sync_now')

  const load = useCallback(() => {
    api.sync_status().then((x) => { setS(x); setFailed(false) }).catch(() => setFailed(true))
  }, [])
  useEffect(load, [load])

  const syncNow = async () => {
    setBusy(true)
    setMsg(null)
    const before = s?.last_confirmed_at ?? null
    try {
      const after = await api.sync_now()
      setS(after)
      // A pass that moved nothing and left "last synced" unchanged with work still
      // pending did not reach the school computer: say so honestly.
      const reached = after.pending_count === 0 || (after.last_confirmed_at !== null && after.last_confirmed_at !== before)
      setMsg(t(reached ? 'tp.sync.done' : 'tp.sync.failed'))
    } catch {
      setMsg(t('tp.sync.failed'))
    } finally {
      setBusy(false)
    }
  }

  const now = new Date()
  const last = s?.last_confirmed_at ? new Date(s.last_confirmed_at) : null

  return (
    <PhoneScreen
      title={t('tp.sync.title')}
      footer={
        canSyncNow ? (
          <button type="button" disabled={busy} aria-busy={busy} onClick={() => void syncNow()} style={{ ...PHONE_PRIMARY, opacity: busy ? 0.7 : 1 }}>
            {busy ? t('tp.sync.busy') : t('tp.sync.now')}
          </button>
        ) : undefined
      }
    >
      {s === null ? (
        <PhoneEmpty>{failed ? t('tp.sync.failed') : t('tp.loading')}</PhoneEmpty>
      ) : (
        <>
          <div style={PHONE_CARD}>
            <Row first label={t('tp.sync.pending')} value={t('tp.sync.pendingVal', { n: s.pending_count })} />
            <Row label={t('tp.sync.last')} value={last && !Number.isNaN(last.getTime()) ? agoText(last, now, t) : t('tp.sync.never')} />
          </div>
          <p style={{ margin: 0, fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{t(s.pending_count === 0 ? 'tp.sync.upToDate' : 'tp.sync.waiting')}</p>
          {isWeb ? <p style={{ margin: 0, fontSize: 13, lineHeight: 1.5, color: 'var(--muted)' }}>{t('tp.sync.webNote')}</p> : null}
        </>
      )}
      {msg ? <span role="status" style={{ fontSize: 13, color: 'var(--ink)' }}>{msg}</span> : null}
    </PhoneScreen>
  )
}
