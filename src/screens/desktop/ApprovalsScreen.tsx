import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { RequestDto } from '@/lib/api'

// Approvals — the Principal's decision queue (prompts/P03 Step 9).
// Segmented tabs filter pending requests by kind; a list of rows on the left,
// a "Sheet"-pattern detail panel on the right with a BEFORE → AFTER table and
// Approve / Return / Reject actions. Self-fetching; degrades gracefully when
// the backend is unavailable. Visible text via t(); tokens only for colour.

// --- Tabs → RequestDto.kind -----------------------------------------------

type Tab = 'all' | 'marks' | 'payment' | 'attendance' | 'details' | 'access'

const TABS: { id: Tab; labelKey: string }[] = [
  { id: 'all', labelKey: 'approvals.tab.all' },
  { id: 'marks', labelKey: 'approvals.tab.marks' },
  { id: 'payment', labelKey: 'approvals.tab.payment' },
  { id: 'attendance', labelKey: 'approvals.tab.attendance' },
  { id: 'details', labelKey: 'approvals.tab.details' },
  { id: 'access', labelKey: 'approvals.tab.access' },
]

function kindForTab(tab: Tab): string | undefined {
  switch (tab) {
    case 'marks':
      return 'marks_correction'
    case 'payment':
      return 'payment_reversal'
    case 'attendance':
      return 'attendance_correction'
    case 'details':
      return 'student_details'
    case 'access':
      return 'access_change'
    case 'all':
    default:
      return undefined
  }
}

// --- Presentation helpers --------------------------------------------------

// Compact "2h ago" / "1d ago". Falls back to formatRelative when parseable;
// this local variant is a pure function used for the terse list-row age.
function ageFrom(iso: string): string {
  const then = new Date(iso).getTime()
  if (Number.isNaN(then)) return ''
  const secs = Math.max(0, Math.floor((Date.now() - then) / 1000))
  if (secs < 60) return `${secs}s ago`
  const mins = Math.floor(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  return `${days}d ago`
}

interface Badge {
  bg: string
  fg: string
}

function badgeFor(kind: string): Badge {
  switch (kind) {
    case 'marks_correction':
      return { bg: '#E7E3F1', fg: '#4A3B78' }
    case 'payment_reversal':
      return { bg: '#F6E4E2', fg: '#8E2F2A' }
    case 'student_details':
      return { bg: '#F4ECDC', fg: '#6B5220' }
    case 'attendance_correction':
      return { bg: 'var(--accent-12)', fg: 'var(--accent)' }
    default:
      return { bg: 'var(--panel)', fg: 'var(--muted)' }
  }
}

// Parse a JSON string into a flat record of string values; empty on failure.
function parseJson(raw: string | null): Record<string, string> {
  if (raw == null || raw === '') return {}
  try {
    const parsed: unknown = JSON.parse(raw)
    if (parsed == null || typeof parsed !== 'object') return {}
    const out: Record<string, string> = {}
    for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
      out[k] = typeof v === 'string' ? v : JSON.stringify(v)
    }
    return out
  } catch {
    return {}
  }
}

// The list-row title: an after_json.summary when present, else the reason.
function rowTitle(req: RequestDto): string {
  const after = parseJson(req.after_json)
  const summary = after['summary']
  if (summary != null && summary !== '') return summary
  return req.reason
}

// --- Static styles ---------------------------------------------------------

const EYEBROW: CSSProperties = {
  fontSize: '10px',
  textTransform: 'uppercase',
  letterSpacing: '.16em',
  color: 'var(--muted)',
}

const SERIF: CSSProperties = {
  fontFamily: "'Newsreader', Georgia, serif",
  color: 'var(--ink)',
}

function segItemStyle(selected: boolean): CSSProperties {
  return {
    minWidth: '44px',
    padding: '8px 14px',
    fontSize: '13px',
    fontWeight: 500,
    border: 'none',
    background: selected ? 'var(--accent)' : 'transparent',
    color: selected ? '#FFFFFF' : 'var(--ink)',
    cursor: 'pointer',
    borderRight: '1px solid var(--line-strong)',
  }
}

function primaryBtnStyle(): CSSProperties {
  return {
    padding: '9px 18px',
    borderRadius: '8px',
    border: '1px solid var(--accent)',
    background: 'var(--accent)',
    color: '#FFFFFF',
    fontWeight: 500,
    fontSize: '13px',
    cursor: 'pointer',
  }
}

function secondaryBtnStyle(): CSSProperties {
  return {
    padding: '9px 18px',
    borderRadius: '8px',
    border: '1px solid var(--line-strong)',
    background: 'var(--surface)',
    color: 'var(--ink)',
    fontWeight: 500,
    fontSize: '13px',
    cursor: 'pointer',
  }
}

// --- Sub-components ---------------------------------------------------------

function KeyValueTable({ label, data }: { label: string; data: Record<string, string> }) {
  const entries = Object.entries(data)
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ ...EYEBROW, marginBottom: '8px' }}>{label}</div>
      <div
        style={{
          border: '1px solid var(--line)',
          borderRadius: '10px',
          overflow: 'hidden',
          background: 'var(--surface)',
        }}
      >
        {entries.length === 0 ? (
          <div style={{ padding: '10px 12px', fontSize: '12px', color: 'var(--muted)' }}>—</div>
        ) : (
          entries.map(([k, v], i) => (
            <div
              key={k}
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                gap: '12px',
                padding: '10px 12px',
                fontSize: '13px',
                color: 'var(--ink)',
                ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}),
              }}
            >
              <span style={{ color: 'var(--muted)' }}>{k}</span>
              <span style={{ fontWeight: 500, textAlign: 'right' }}>{v}</span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

function DetailPanel({
  req,
  note,
  onNote,
  onDecide,
}: {
  req: RequestDto
  note: string
  onNote: (v: string) => void
  onDecide: (decision: string) => void
}) {
  const badge = badgeFor(req.kind)
  return (
    <div
      style={{
        background: 'var(--surface)',
        borderRadius: '20px',
        border: '1px solid var(--line)',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <div style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '3px 10px',
              borderRadius: '4px',
              fontSize: '12px',
              fontWeight: 500,
              background: badge.bg,
              color: badge.fg,
            }}
          >
            {req.kind}
          </span>
          <span style={{ fontSize: '12px', color: 'var(--muted)' }}>{ageFrom(req.created_at)}</span>
        </div>

        <div>
          <div style={EYEBROW}>{req.requester_name}</div>
          <div style={{ ...SERIF, fontSize: '20px', marginTop: '4px' }}>{rowTitle(req)}</div>
        </div>

        {req.reason !== '' ? (
          <div style={{ fontSize: '13px', color: 'var(--muted)', lineHeight: 1.5 }}>{req.reason}</div>
        ) : null}

        <div style={{ display: 'flex', gap: '16px', alignItems: 'stretch' }}>
          <KeyValueTable label={t('approvals.before')} data={parseJson(req.before_json)} />
          <div style={{ alignSelf: 'center', color: 'var(--muted)' }}>
            <Icon name="chevronRight" size={18} />
          </div>
          <KeyValueTable label={t('approvals.after')} data={parseJson(req.after_json)} />
        </div>

        <input
          value={note}
          onChange={(e) => onNote(e.target.value)}
          aria-label={t('approvals.title')}
          style={{
            width: '100%',
            boxSizing: 'border-box',
            padding: '9px 12px',
            borderRadius: '8px',
            border: '1px solid var(--line)',
            background: 'var(--bg)',
            color: 'var(--ink)',
            fontSize: '13px',
            fontFamily: 'inherit',
          }}
        />
      </div>

      <div
        style={{
          background: 'var(--panel)',
          padding: '14px 24px',
          display: 'flex',
          gap: '10px',
          justifyContent: 'flex-end',
        }}
      >
        <button type="button" style={secondaryBtnStyle()} onClick={() => onDecide('reject')}>
          {t('approvals.reject')}
        </button>
        <button type="button" style={secondaryBtnStyle()} onClick={() => onDecide('return')}>
          {t('approvals.return')}
        </button>
        <button type="button" style={primaryBtnStyle()} onClick={() => onDecide('approve')}>
          {t('approvals.approve')}
        </button>
      </div>
    </div>
  )
}

// --- Screen -----------------------------------------------------------------

export default function ApprovalsScreen() {
  const [tab, setTab] = useState<Tab>('all')
  const [requests, setRequests] = useState<RequestDto[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [note, setNote] = useState<string>('')

  const refetch = useCallback((forTab: Tab) => {
    api
      .list_requests('pending', kindForTab(forTab))
      .then((rows) => {
        setRequests(rows)
        setError(null)
      })
      .catch(() => {
        setRequests([])
        setError(t('approvals.empty'))
      })
  }, [])

  useEffect(() => {
    setSelectedId(null)
    setNote('')
    refetch(tab)
  }, [tab, refetch])

  const selected = useMemo(
    () => requests.find((r) => r.id === selectedId) ?? null,
    [requests, selectedId],
  )

  const decide = useCallback(
    (decision: string) => {
      if (selectedId == null) return
      api
        .decide_request(selectedId, decision, note || undefined)
        .then(() => {
          setSelectedId(null)
          setNote('')
          refetch(tab)
        })
        .catch(() => {
          setError(t('approvals.empty'))
        })
    },
    [selectedId, note, tab, refetch],
  )

  // Keyboard: J/K move the selection, A approves, R returns.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const key = e.key.toLowerCase()
      if (key === 'j' || key === 'k') {
        if (requests.length === 0) return
        const idx = requests.findIndex((r) => r.id === selectedId)
        const next =
          key === 'j'
            ? Math.min(requests.length - 1, idx < 0 ? 0 : idx + 1)
            : Math.max(0, idx < 0 ? 0 : idx - 1)
        setSelectedId(requests[next].id)
      } else if (key === 'a' && selectedId != null) {
        decide('approve')
      } else if (key === 'r' && selectedId != null) {
        decide('return')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [requests, selectedId, decide])

  const pendingCount = requests.length

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
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
        <Icon name="approvals" size={22} color="var(--navy)" />
        <h1 style={{ ...SERIF, fontSize: '26px', margin: 0 }}>{t('approvals.title')}</h1>
        <span
          aria-label={t('approvals.title')}
          data-sidebar-badge={pendingCount}
          style={{
            marginLeft: '4px',
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            minWidth: '22px',
            height: '22px',
            padding: '0 6px',
            borderRadius: '11px',
            background: 'var(--accent-12)',
            color: 'var(--accent)',
            fontSize: '12px',
            fontWeight: 600,
          }}
        >
          {pendingCount}
        </span>
      </div>

      {/* Segmented control */}
      <div
        style={{
          display: 'inline-flex',
          alignSelf: 'flex-start',
          border: '1px solid var(--line-strong)',
          borderRadius: '6px',
          overflow: 'hidden',
        }}
      >
        {TABS.map((tabDef) => (
          <button
            key={tabDef.id}
            type="button"
            aria-pressed={tab === tabDef.id}
            style={segItemStyle(tab === tabDef.id)}
            onClick={() => setTab(tabDef.id)}
          >
            {t(tabDef.labelKey)}
          </button>
        ))}
      </div>

      {/* Body: list + detail */}
      <div style={{ display: 'flex', gap: '20px', alignItems: 'flex-start' }}>
        <div
          style={{
            flex: '1 1 0',
            minWidth: 0,
            background: 'var(--surface)',
            borderRadius: '20px',
            border: '1px solid var(--line)',
            overflow: 'hidden',
          }}
        >
          {requests.length === 0 ? (
            <div
              style={{
                padding: '48px 24px',
                textAlign: 'center',
                color: 'var(--muted)',
                fontSize: '14px',
              }}
            >
              {error ?? t('approvals.empty')}
            </div>
          ) : (
            requests.map((req, i) => {
              const badge = badgeFor(req.kind)
              const isSel = req.id === selectedId
              return (
                <button
                  key={req.id}
                  type="button"
                  onClick={() => {
                    setSelectedId(req.id)
                    setNote('')
                  }}
                  style={{
                    display: 'flex',
                    width: '100%',
                    alignItems: 'center',
                    gap: '12px',
                    padding: '14px 24px',
                    textAlign: 'left',
                    border: 'none',
                    background: isSel ? 'var(--accent-12)' : 'transparent',
                    cursor: 'pointer',
                    color: 'inherit',
                    ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}),
                  }}
                >
                  <span
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      padding: '3px 8px',
                      borderRadius: '4px',
                      fontSize: '11px',
                      fontWeight: 500,
                      whiteSpace: 'nowrap',
                      background: badge.bg,
                      color: badge.fg,
                    }}
                  >
                    {req.kind}
                  </span>
                  <span style={{ flex: 1, minWidth: 0 }}>
                    <span
                      style={{
                        display: 'block',
                        fontWeight: 500,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {rowTitle(req)}
                    </span>
                    <span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>
                      {req.requester_name} · {ageFrom(req.created_at)}
                    </span>
                  </span>
                  <Icon name="chevronRight" size={16} color="var(--muted)" />
                </button>
              )
            })
          )}
        </div>

        <div style={{ flex: '1 1 0', minWidth: 0 }}>
          {selected != null ? (
            <DetailPanel req={selected} note={note} onNote={setNote} onDecide={decide} />
          ) : (
            <div
              style={{
                padding: '48px 24px',
                textAlign: 'center',
                color: 'var(--muted)',
                fontSize: '14px',
                border: '1px solid var(--line)',
                borderRadius: '20px',
                background: 'var(--surface)',
              }}
            >
              {t('approvals.empty')}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
