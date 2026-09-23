import type { CSSProperties, ReactNode } from 'react'

// Pill (docs/01-MOCK-SPEC.md §7): auto width, 24px tall, radius 4, 12px/500,
// padding 0 10. Variants map to the mock's badge colours (tokens only).
export type PillVariant =
  | 'paid'
  | 'partpaid'
  | 'unpaid'
  | 'marks'
  | 'attendance'
  | 'details'
  | 'reversal'
  | 'neutral'

const VARIANTS: Record<PillVariant, { bg: string; fg: string }> = {
  paid: { bg: 'var(--accent-12)', fg: 'var(--accent)' },
  attendance: { bg: 'var(--accent-12)', fg: 'var(--accent)' },
  partpaid: { bg: 'var(--pill-partpaid-bg)', fg: 'var(--pill-partpaid-fg)' },
  details: { bg: 'var(--pill-partpaid-bg)', fg: 'var(--pill-partpaid-fg)' },
  unpaid: { bg: 'var(--pill-unpaid-bg)', fg: 'var(--pill-unpaid-fg)' },
  reversal: { bg: 'var(--pill-unpaid-bg)', fg: 'var(--pill-unpaid-fg)' },
  marks: { bg: 'var(--pill-marks-bg)', fg: 'var(--pill-marks-fg)' },
  neutral: { bg: 'var(--panel)', fg: 'var(--muted)' },
}

export function Pill({ variant, children, style }: { variant: PillVariant; children: ReactNode; style?: CSSProperties }) {
  const v = VARIANTS[variant]
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        height: '24px',
        padding: '0 10px',
        borderRadius: '4px',
        fontSize: '12px',
        fontWeight: 500,
        whiteSpace: 'nowrap',
        background: v.bg,
        color: v.fg,
        ...style,
      }}
    >
      {children}
    </span>
  )
}

export default Pill
