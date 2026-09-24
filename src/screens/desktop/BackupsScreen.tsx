// Backups (docs §12, prompts/P08 Part B/C). Shows how data is protected, restore
// steps, and the real backup history (backup_status). The write path (manual
// "Back up now" + the daily scheduler) is the next step — this screen is honest
// about that rather than faking a backup.

import { useEffect, useState } from 'react'
import * as api from '@/lib/api'
import type { BackupRunDto } from '@/lib/api'
import { formatRelative } from '@/lib/format'
import { t } from '@/lib/i18n'

const CARD: React.CSSProperties = { background: 'var(--surface)', border: '1px solid var(--line)', borderRadius: 16, padding: 24 }
const SERIF = "'Newsreader', Georgia, serif"
const H3: React.CSSProperties = { fontSize: 15, fontWeight: 600, margin: '0 0 12px' }

function Bullet({ children }: { children: React.ReactNode }) {
  return (
    <li style={{ fontSize: 14, color: 'var(--ink)', marginBottom: 8, listStyle: 'none', display: 'flex', gap: 10 }}>
      <span style={{ width: 6, height: 6, borderRadius: 3, background: 'var(--accent)', marginTop: 7, flexShrink: 0 }} />
      <span>{children}</span>
    </li>
  )
}

export default function BackupsScreen() {
  const [runs, setRuns] = useState<BackupRunDto[] | null>(null)

  useEffect(() => {
    api.backup_status().then(setRuns).catch(() => setRuns([]))
  }, [])

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg)', color: 'var(--ink)', padding: 40, fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif" }}>
      <div style={{ marginBottom: 24 }}>
        <h1 style={{ fontFamily: SERIF, fontSize: 40, letterSpacing: '-0.025em', margin: 0 }}>{t('backups.title')}</h1>
        <p style={{ color: 'var(--muted)', fontSize: 14, margin: '6px 0 0' }}>{t('backups.subtitle')}</p>
      </div>

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
          {runs && runs.length > 0 ? (
            <div>
              {runs.map((r, i) => (
                <div key={i} style={{ display: 'flex', justifyContent: 'space-between', gap: 16, padding: '10px 0', borderTop: i === 0 ? 'none' : '1px solid var(--track)', fontSize: 14 }}>
                  <span>{r.at ? formatRelative(new Date(r.at)) : '—'}</span>
                  <span style={{ color: 'var(--muted)' }}>
                    {r.status}
                    {r.destination ? ` · ${r.destination}` : ''}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <>
              <p style={{ fontSize: 14, color: 'var(--muted)', margin: '0 0 8px' }}>{t('backups.history.empty')}</p>
              <p style={{ fontSize: 13, color: 'var(--muted)', margin: 0, lineHeight: 1.5 }}>{t('backups.history.note')}</p>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
