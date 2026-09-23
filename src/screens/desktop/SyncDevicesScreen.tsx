import { useCallback, useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import { formatMoney } from '@/lib/format'
import * as api from '@/lib/api'
import type { ServerStatusDto, DeviceDto, SyncStatusDto } from '@/lib/api'

// Sync & devices — the school-server view (prompts/P04 Step 9).
// Top: a "server" card with status, LAN addresses, port and fingerprint.
// Middle: a devices table (owner, platform, series, last-seen) with per-row
// Revoke. Bottom: a footer band showing the client's pending sync state plus a
// "Sync now" button. Self-fetching; visible text via t(); tokens only for colour.

// --- Presentation helpers --------------------------------------------------

// Compact "2h ago" / "—". Pure and local — never touches the network.
function ago(iso: string | null): string {
  if (iso == null || iso === '') return '—'
  const then = new Date(iso).getTime()
  if (Number.isNaN(then)) return '—'
  const secs = Math.max(0, Math.floor((Date.now() - then) / 1000))
  if (secs < 60) return `${secs}s ago`
  const mins = Math.floor(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  return `${days}d ago`
}

// --- Static styles ---------------------------------------------------------

const SERIF: CSSProperties = {
  fontFamily: "'Newsreader', Georgia, serif",
  color: 'var(--ink)',
}

const EYEBROW: CSSProperties = {
  fontSize: '10px',
  textTransform: 'uppercase',
  letterSpacing: '.16em',
  color: 'var(--muted)',
}

const CELL: CSSProperties = {
  padding: '14px 24px',
  fontSize: '13px',
  color: 'var(--ink)',
  textAlign: 'left',
}

function fieldRow(_label: string, _value: string, first: boolean): CSSProperties {
  return {
    display: 'flex',
    justifyContent: 'space-between',
    gap: '16px',
    padding: '12px 0',
    fontSize: '13px',
    ...(first ? {} : { borderTop: '1px solid var(--track)' }),
  }
}

// --- Sub-components ---------------------------------------------------------

function ServerCard({ server }: { server: ServerStatusDto }) {
  const fields: { label: string; value: string }[] = [
    { label: t('sd.status'), value: t('sd.online') },
    { label: t('sd.addresses'), value: server.lan_addrs.join(', ') || '—' },
    { label: t('sd.port'), value: String(server.port) },
    { label: t('sd.fingerprint'), value: server.fingerprint.slice(0, 16) },
  ]
  return (
    <div
      style={{
        background: 'var(--surface)',
        border: '1px solid var(--line)',
        borderRadius: '16px',
        padding: '20px 24px',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
        <Icon name="cloudSync" size={18} color="var(--navy)" />
        <span style={{ ...SERIF, fontSize: '18px' }}>{server.school_name}</span>
        <span style={{ ...EYEBROW, marginLeft: 'auto' }}>{t('sd.server')}</span>
      </div>
      {fields.map((f, i) => (
        <div key={f.label} style={fieldRow(f.label, f.value, i === 0)}>
          <span style={{ color: 'var(--muted)' }}>{f.label}</span>
          <span style={{ fontWeight: 500, textAlign: 'right', color: 'var(--ink)' }}>{f.value}</span>
        </div>
      ))}
    </div>
  )
}

function DeviceRow({
  device,
  first,
  onRevoke,
}: {
  device: DeviceDto
  first: boolean
  onRevoke: (id: string) => void
}) {
  return (
    <tr style={first ? {} : { borderTop: '1px solid var(--track)' }}>
      <td style={CELL}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '8px' }}>
          {device.revoked ? (
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 500,
                background: 'var(--danger)',
                color: '#FFFFFF',
              }}
            >
              {t('sd.revoked')}
            </span>
          ) : (
            <span
              aria-label={t('sd.online')}
              style={{
                display: 'inline-block',
                width: '8px',
                height: '8px',
                borderRadius: '50%',
                background: 'var(--online)',
              }}
            />
          )}
          <span style={{ fontWeight: 500 }}>{device.owner_name}</span>
        </span>
      </td>
      <td style={CELL}>{device.platform}</td>
      <td style={{ ...CELL, color: 'var(--muted)' }}>{device.series ?? '—'}</td>
      <td style={{ ...CELL, color: 'var(--muted)' }}>{ago(device.last_seen_at)}</td>
      <td style={{ ...CELL, textAlign: 'right' }}>
        <button
          type="button"
          disabled={device.revoked}
          onClick={() => onRevoke(device.id)}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '6px',
            padding: '6px 12px',
            borderRadius: '8px',
            border: '1px solid var(--line-strong)',
            background: 'var(--surface)',
            color: device.revoked ? 'var(--muted)' : 'var(--danger)',
            fontSize: '12px',
            fontWeight: 500,
            cursor: device.revoked ? 'default' : 'pointer',
            opacity: device.revoked ? 0.5 : 1,
          }}
        >
          <Icon name="close" size={14} />
          {t('sd.revoke')}
        </button>
      </td>
    </tr>
  )
}

// --- Screen -----------------------------------------------------------------

export default function SyncDevicesScreen() {
  const [server, setServer] = useState<ServerStatusDto | null>(null)
  const [devices, setDevices] = useState<DeviceDto[]>([])
  const [sync, setSync] = useState<SyncStatusDto | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    Promise.all([api.server_status(), api.list_devices(), api.sync_status()])
      .then(([s, d, y]) => {
        if (!live) return
        setServer(s)
        setDevices(d)
        setSync(y)
        setError(null)
      })
      .catch(() => {
        if (!live) return
        setError(t('sd.title'))
      })
    return () => {
      live = false
    }
  }, [])

  const revoke = useCallback((id: string) => {
    api
      .revoke_device(id)
      .then(() => api.list_devices())
      .then((d) => setDevices(d))
      .catch(() => setError(t('sd.title')))
  }, [])

  const syncNow = useCallback(() => {
    api
      .sync_now()
      .then((y) => setSync(y))
      .catch(() => setError(t('sd.title')))
  }, [])

  const headers = [
    t('sd.owner'),
    t('sd.platform'),
    t('sd.series'),
    t('sd.lastSeen'),
    '',
  ]

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '20px',
        padding: '28px',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        minHeight: '100%',
        boxSizing: 'border-box',
      }}
    >
      <h1 style={{ ...SERIF, fontSize: '40px', margin: 0 }}>{t('sd.title')}</h1>

      {error != null ? (
        <div style={{ fontSize: '13px', color: 'var(--danger)' }}>{error}</div>
      ) : null}

      {server != null ? <ServerCard server={server} /> : null}

      {/* Devices table */}
      <div
        style={{
          background: 'var(--surface)',
          border: '1px solid var(--line)',
          borderRadius: '16px',
          overflow: 'hidden',
        }}
      >
        <div style={{ padding: '16px 24px', borderBottom: '1px solid var(--track)' }}>
          <span style={{ ...SERIF, fontSize: '18px' }}>{t('sd.devices')}</span>
        </div>
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr>
              {headers.map((h, i) => (
                <th
                  key={i}
                  style={{
                    ...EYEBROW,
                    padding: '12px 24px',
                    textAlign: i === headers.length - 1 ? 'right' : 'left',
                    background: 'var(--panel)',
                  }}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {devices.map((device, i) => (
              <DeviceRow key={device.id} device={device} first={i === 0} onRevoke={revoke} />
            ))}
          </tbody>
        </table>
      </div>

      {/* Client sync footer band */}
      <div
        style={{
          background: 'var(--panel)',
          borderRadius: '16px',
          padding: '14px 24px',
          display: 'flex',
          alignItems: 'center',
          gap: '16px',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          <span style={EYEBROW}>{t('sd.pending')}</span>
          <span style={{ fontSize: '14px', color: 'var(--ink)' }}>
            {sync != null
              ? `${sync.pending_count} · ${formatMoney(sync.pending_paise)} ${t('sd.waiting')}`
              : '—'}
          </span>
        </div>
        <button
          type="button"
          onClick={syncNow}
          style={{
            marginLeft: 'auto',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            padding: '9px 18px',
            borderRadius: '8px',
            border: '1px solid var(--accent)',
            background: 'var(--accent)',
            color: '#FFFFFF',
            fontWeight: 500,
            fontSize: '13px',
            cursor: 'pointer',
          }}
        >
          <Icon name="sync" size={16} color="#FFFFFF" />
          {t('sd.syncNow')}
        </button>
      </div>
    </div>
  )
}
