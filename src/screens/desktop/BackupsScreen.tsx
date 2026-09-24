// Backups (docs §12, prompts/P08 Part B/C, wired P10). Turn on backups (recovery
// key → derives + caches the backup key), "Back up now", and real history. Once
// enabled, the backend scheduler also backs up automatically whenever the data
// changes. Backups are encrypted with the recovery-derived key, so restore on a
// new PC needs only the recovery key.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { BackupRunDto, BackupStatusDto, CmdError } from '@/lib/api'
import { formatRelative } from '@/lib/format'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const SERIF = "'Newsreader', Georgia, serif"
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 12px' }
const TICK_SECS = 20

function Bullet({ children }: { children: React.ReactNode }) {
  return (
    <li style={{ fontSize: 14, color: 'var(--ink)', marginBottom: 8, listStyle: 'none', display: 'flex', gap: 10 }}>
      <span style={{ width: 6, height: 6, borderRadius: 3, background: 'var(--accent)', marginTop: 7, flexShrink: 0 }} />
      <span>{children}</span>
    </li>
  )
}

function statusLabel(run: BackupRunDto): string {
  if (run.status === 'verified') return t('backups.status.verified')
  if (run.status === 'failed') return t('backups.status.failed')
  return t('backups.status.partial')
}

export default function BackupsScreen() {
  const [status, setStatus] = useState<BackupStatusDto | null>(null)
  const [recoveryKey, setRecoveryKey] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState(false)

  const refresh = () => api.backup_status().then(setStatus).catch(() => setStatus({ enabled: false, runs: [] }))
  useEffect(() => {
    refresh()
  }, [])

  const runBackup = (key?: string) => {
    setBusy(true)
    setError(null)
    setDone(false)
    api
      .backup_now(key)
      .then((s) => {
        setStatus(s)
        setRecoveryKey('')
        setDone(true)
      })
      .catch((e) => setError(t((e as CmdError).message_key, (e as CmdError).vars as Record<string, string | number>)))
      .finally(() => setBusy(false))
  }

  const enabled = status?.enabled ?? false
  const runs = status?.runs ?? []
  const btn = (bg: string, fg: string): React.CSSProperties => ({ height: 42, padding: '0 20px', borderRadius: 8, border: bg === 'transparent' ? '1px solid var(--line-strong)' : 'none', background: bg, color: fg, fontSize: 14, fontWeight: 600, cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.6 : 1 })
  const input: React.CSSProperties = { height: 42, borderRadius: 8, border: '1px solid var(--line-strong)', padding: '0 14px', background: 'var(--white)', color: 'var(--ink)', fontSize: 14, width: '100%', boxSizing: 'border-box', fontFamily: 'ui-monospace, Menlo, monospace', letterSpacing: '0.04em' }

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--ink)', padding: 40, fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', marginBottom: 24, gap: 16 }}>
        <div>
          <h1 style={{ fontFamily: SERIF, fontSize: 40, letterSpacing: '-0.025em', margin: 0 }}>{t('backups.title')}</h1>
          <p style={{ color: 'var(--muted)', fontSize: 14, margin: '6px 0 0' }}>{t('backups.subtitle')}</p>
        </div>
        {enabled && (
          <button onClick={() => runBackup()} disabled={busy} style={btn('var(--accent)', '#fff')}>
            {busy ? t('backups.running') : t('backups.now')}
          </button>
        )}
      </div>

      {error && <div style={{ ...CARD, padding: 14, marginBottom: 16, borderColor: 'var(--danger)', color: 'var(--danger)', fontSize: 14 }}>{error}</div>}
      {done && !error && <div style={{ ...CARD, padding: 14, marginBottom: 16, borderColor: 'var(--accent)', color: 'var(--accent)', fontSize: 14 }}>{t('backups.done')}</div>}

      {/* Enable card (only when backups are off) */}
      {!enabled && (
        <div style={{ ...CARD, marginBottom: 20, maxWidth: 640, borderColor: 'var(--gold)' }}>
          <h3 style={H3}>{t('backups.enable.title')}</h3>
          <p style={{ fontSize: 13, color: 'var(--muted)', margin: '0 0 14px', lineHeight: 1.5 }}>{t('backups.enable.body')}</p>
          <input
            style={input}
            value={recoveryKey}
            onChange={(e) => setRecoveryKey(e.target.value)}
            placeholder={t('backups.enable.placeholder')}
            aria-label={t('backups.enable.title')}
          />
          <div style={{ marginTop: 14 }}>
            <button onClick={() => runBackup(recoveryKey)} disabled={busy || recoveryKey.trim().length < 20} style={btn('var(--accent)', '#fff')}>
              {busy ? t('backups.running') : t('backups.enable.button')}
            </button>
          </div>
        </div>
      )}

      {enabled && (
        <div style={{ ...CARD, marginBottom: 20, maxWidth: 640, display: 'flex', alignItems: 'center', gap: 12 }}>
          <span style={{ width: 8, height: 8, borderRadius: 4, background: 'var(--online)', flexShrink: 0 }} />
          <div>
            <div style={{ fontWeight: 600, fontSize: 14 }}>{t('backups.enabledPill')}</div>
            <div style={{ fontSize: 13, color: 'var(--muted)' }}>{t('backups.autoNote', { secs: TICK_SECS })}</div>
          </div>
        </div>
      )}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 20, maxWidth: 900 }}>
        <div style={CARD}>
          <h3 style={H3}>{t('backups.how.title')}</h3>
          <ul style={{ margin: 0, padding: 0 }}>
            <Bullet>{t('backups.how.1')}</Bullet>
            <Bullet>{t('backups.how.2')}</Bullet>
            <Bullet>{t('backups.how.3')}</Bullet>
          </ul>
        </div>

        <div style={CARD}>
          <h3 style={H3}>{t('backups.restore.title')}</h3>
          <p style={{ fontSize: 14, color: 'var(--muted)', margin: 0, lineHeight: 1.5 }}>{t('backups.restore.body')}</p>
        </div>

        <div style={{ ...CARD, gridColumn: '1 / -1' }}>
          <h3 style={H3}>{t('backups.history.title')}</h3>
          {runs.length > 0 ? (
            <div>
              {runs.map((r, i) => (
                <div key={i} style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '10px 0', borderTop: i === 0 ? 'none' : '1px solid var(--track)', fontSize: 14 }}>
                  <span>{r.at ? formatRelative(new Date(r.at)) : '—'}</span>
                  <span style={{ color: r.status === 'failed' ? 'var(--danger)' : 'var(--muted)' }}>{statusLabel(r)}</span>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ fontSize: 14, color: 'var(--muted)', margin: 0 }}>{t('backups.history.empty')}</p>
          )}
        </div>
      </div>
    </div>
  )
}
