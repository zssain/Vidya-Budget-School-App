import type { CSSProperties, ReactNode } from 'react'

// PageTitle (docs/01-MOCK-SPEC.md §7): accent eyebrow (11px/600/.14em + 5px dot)
// → Newsreader 46px h1 → optional italic accent sub-line (46px) → actions slot.
// Same lockup as Principal Home and Collect fee. Used by every derived desktop
// screen (docs §6.2).

const EYEBROW: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  fontSize: '11px',
  fontWeight: 600,
  letterSpacing: '0.14em',
  textTransform: 'uppercase',
  color: 'var(--accent)',
}

const DOT: CSSProperties = { width: '5px', height: '5px', borderRadius: '3px', background: 'var(--accent)' }

const H1: CSSProperties = {
  margin: 0,
  fontFamily: 'var(--font-serif)',
  fontWeight: 400,
  fontSize: '46px',
  lineHeight: 1.05,
  letterSpacing: '-0.025em',
  color: 'var(--ink)',
}

const SUB: CSSProperties = {
  fontFamily: 'var(--font-serif)',
  fontStyle: 'italic',
  fontSize: '46px',
  lineHeight: 1.1,
  letterSpacing: '-0.025em',
  color: 'var(--accent)',
}

export function PageTitle({
  eyebrow,
  title,
  sub,
  actions,
}: {
  eyebrow: string
  title: string
  sub?: string
  actions?: ReactNode
}) {
  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: '24px' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <span style={EYEBROW}>
          <span style={DOT} />
          {eyebrow}
        </span>
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <h1 className="v-serif" style={H1}>
            {title}
          </h1>
          {sub != null && sub !== '' ? (
            <span className="v-serif" style={SUB}>
              {sub}
            </span>
          ) : null}
        </div>
      </div>
      {actions != null ? <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>{actions}</div> : null}
    </div>
  )
}

export default PageTitle
