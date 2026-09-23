import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties } from 'react'
import { Icon } from '@/components/Icon'
import { t } from '@/lib/i18n'
import * as api from '@/lib/api'
import type { ConflictDto, ReviewFlagDto } from '@/lib/api'

// Conflict review — the Principal's screen (prompts/P04 Step 8).
// A list of open conflicts on the left; the selected one opens a "Sheet"-pattern
// detail card showing the field with current (A) vs incoming (B) side by side,
// plus who/when, and Keep current / Keep incoming / Edit actions. Below, a
// "Needs a check" section lists review flags with a Mark-as-checked action.
// Resolution is always an explicit op — nothing is auto-resolved. Self-fetching;
// degrades gracefully when the backend is unavailable. Visible text via t();
// tokens only for colour.

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

const PRIMARY_BTN: CSSProperties = {
  padding: '9px 18px',
  borderRadius: '6px',
  border: '1px solid var(--accent)',
  background: 'var(--accent)',
  color: '#FFFFFF',
  fontWeight: 500,
  fontSize: '13px',
  cursor: 'pointer',
}

const SECONDARY_BTN: CSSProperties = {
  padding: '9px 18px',
  borderRadius: '6px',
  border: '1px solid var(--line-strong)',
  background: 'transparent',
  color: 'var(--ink)',
  fontWeight: 500,
  fontSize: '13px',
  cursor: 'pointer',
}

const INPUT: CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: '9px 12px',
  borderRadius: '8px',
  border: '1px solid var(--line)',
  background: 'var(--bg)',
  color: 'var(--ink)',
  fontSize: '13px',
  fontFamily: 'inherit',
}

const EMPTY: CSSProperties = {
  padding: '40px 24px',
  textAlign: 'center',
  color: 'var(--muted)',
  fontSize: '14px',
}

// A single "Current" / "Incoming" value column inside the detail card.
function ValueColumn({ label, value }: { label: string; value: string | null }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ ...EYEBROW, marginBottom: '8px' }}>{label}</div>
      <div
        style={{
          border: '1px solid var(--line)',
          borderRadius: '10px',
          padding: '12px',
          background: 'var(--surface)',
          fontSize: '14px',
          color: value == null || value === '' ? 'var(--muted)' : 'var(--ink)',
          wordBreak: 'break-word',
          minHeight: '20px',
        }}
      >
        {value == null || value === '' ? '—' : value}
      </div>
    </div>
  )
}

// --- Screen -----------------------------------------------------------------

export default function ConflictReviewScreen() {
  const [conflicts, setConflicts] = useState<ConflictDto[]>([])
  const [flags, setFlags] = useState<ReviewFlagDto[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [editValue, setEditValue] = useState<string>('')
  const [error, setError] = useState<string | null>(null)
  const [editing, setEditing] = useState<boolean>(false)

  const refetch = useCallback(() => {
    Promise.all([api.list_conflicts(), api.list_review_flags()])
      .then(([c, f]) => {
        setConflicts(c)
        setFlags(f)
        setError(null)
      })
      .catch(() => {
        setConflicts([])
        setFlags([])
        setError(t('cr.empty'))
      })
  }, [])

  useEffect(() => {
    refetch()
  }, [refetch])

  const selected = useMemo(
    () => conflicts.find((c) => c.id === selectedId) ?? null,
    [conflicts, selectedId],
  )

  const resolve = useCallback(
    (choice: string, value?: string) => {
      if (selectedId == null) return
      api
        .resolve_conflict(selectedId, choice, value)
        .then(() => {
          setSelectedId(null)
          setEditing(false)
          setEditValue('')
          refetch()
        })
        .catch(() => setError(t('cr.empty')))
    },
    [selectedId, refetch],
  )

  const markChecked = useCallback(
    (id: string) => {
      api
        .resolve_review_flag(id)
        .then(() => refetch())
        .catch(() => setError(t('cr.empty')))
    },
    [refetch],
  )

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '24px',
        padding: '28px',
        background: 'var(--bg)',
        color: 'var(--ink)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        minHeight: '100%',
        boxSizing: 'border-box',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
        <Icon name="approvals" size={26} color="var(--navy)" />
        <h1 style={{ ...SERIF, fontSize: '40px', margin: 0 }}>{t('cr.title')}</h1>
      </div>

      {/* Body: conflict list + detail */}
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
          {conflicts.length === 0 ? (
            <div style={EMPTY}>{error ?? t('cr.empty')}</div>
          ) : (
            conflicts.map((c, i) => {
              const isSel = c.id === selectedId
              return (
                <button
                  key={c.id}
                  type="button"
                  onClick={() => {
                    setSelectedId(c.id)
                    setEditing(false)
                    setEditValue(c.value_a ?? '')
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
                      background: 'var(--pill-unpaid-bg)',
                      color: 'var(--pill-unpaid-fg)',
                    }}
                  >
                    {c.table}
                  </span>
                  <span style={{ flex: 1, minWidth: 0 }}>
                    <span style={{ display: 'block', fontWeight: 500 }}>{c.field}</span>
                    <span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>
                      {c.record_id}
                    </span>
                  </span>
                  <Icon name="chevronRight" size={16} color="var(--muted)" />
                </button>
              )
            })
          )}
        </div>

        {/* Detail: Sheet-pattern card */}
        <div style={{ flex: '1 1 0', minWidth: 0 }}>
          {selected != null ? (
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
                <div>
                  <div style={EYEBROW}>{t('cr.field')}</div>
                  <div style={{ ...SERIF, fontSize: '20px', marginTop: '4px' }}>{selected.field}</div>
                </div>

                <div style={{ display: 'flex', gap: '16px', alignItems: 'stretch' }}>
                  <ValueColumn label={t('cr.current')} value={selected.value_a} />
                  <div style={{ alignSelf: 'center', color: 'var(--muted)' }}>
                    <Icon name="chevronRight" size={18} />
                  </div>
                  <ValueColumn label={t('cr.incoming')} value={selected.value_b} />
                </div>

                {selected.staff_b != null || selected.hlc_b != null ? (
                  <div style={{ fontSize: '12px', color: 'var(--muted)' }}>
                    {selected.staff_b ?? ''}
                    {selected.staff_b != null && selected.hlc_b != null ? ' · ' : ''}
                    {selected.hlc_b ?? ''}
                  </div>
                ) : null}

                {editing ? (
                  <input
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    aria-label={t('cr.edit')}
                    style={INPUT}
                  />
                ) : null}
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
                {editing ? (
                  <button type="button" style={PRIMARY_BTN} onClick={() => resolve('edit', editValue)}>
                    {t('cr.resolve')}
                  </button>
                ) : (
                  <>
                    <button type="button" style={SECONDARY_BTN} onClick={() => resolve('keep_a')}>
                      {t('cr.keepA')}
                    </button>
                    <button type="button" style={SECONDARY_BTN} onClick={() => resolve('keep_b')}>
                      {t('cr.keepB')}
                    </button>
                    <button type="button" style={PRIMARY_BTN} onClick={() => setEditing(true)}>
                      {t('cr.edit')}
                    </button>
                  </>
                )}
              </div>
            </div>
          ) : (
            <div
              style={{
                ...EMPTY,
                border: '1px solid var(--line)',
                borderRadius: '20px',
                background: 'var(--surface)',
              }}
            >
              {t('cr.empty')}
            </div>
          )}
        </div>
      </div>

      {/* Review flags: "Needs a check" */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <h2 style={{ ...SERIF, fontSize: '22px', margin: 0 }}>{t('cr.flags')}</h2>
        <div
          style={{
            background: 'var(--surface)',
            borderRadius: '20px',
            border: '1px solid var(--line)',
            overflow: 'hidden',
          }}
        >
          {flags.length === 0 ? (
            <div style={EMPTY}>{t('cr.flagsEmpty')}</div>
          ) : (
            flags.map((f, i) => (
              <div
                key={f.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  padding: '14px 24px',
                  ...(i > 0 ? { borderTop: '1px solid var(--track)' } : {}),
                }}
              >
                <span style={{ color: 'var(--danger)' }}>
                  <Icon name="close" size={16} />
                </span>
                <span style={{ flex: 1, minWidth: 0 }}>
                  <span style={{ display: 'block', fontWeight: 500 }}>{f.kind}</span>
                  <span style={{ display: 'block', fontSize: '12px', color: 'var(--muted)' }}>
                    {f.ref_table} · {f.ref_id}
                  </span>
                </span>
                <button type="button" style={SECONDARY_BTN} onClick={() => markChecked(f.id)}>
                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px' }}>
                    <Icon name="check" size={14} />
                    {t('cr.markChecked')}
                  </span>
                </button>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  )
}
