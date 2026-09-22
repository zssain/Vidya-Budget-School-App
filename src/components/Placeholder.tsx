import { t } from '@/lib/i18n'

// "Coming in a later phase" — every non-mock sidebar link/tile routes here in
// Phase 1 (docs/01-MOCK-SPEC.md §7). Must be gone by the last phase.
export default function Placeholder({ title }: { title?: string }) {
  return (
    <div
      style={{
        minHeight: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg)',
        color: 'var(--muted)',
        fontFamily: "'Geist', 'Noto Sans Devanagari', system-ui, sans-serif",
        fontSize: 14,
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
        {title ? (
          <span
            className="v-serif"
            style={{ fontFamily: "'Newsreader', Georgia, serif", fontSize: 26, color: 'var(--ink)' }}
          >
            {title}
          </span>
        ) : null}
        <span style={{ fontSize: 13, color: 'var(--muted)' }}>{t('placeholder.title')}</span>
      </div>
    </div>
  )
}
