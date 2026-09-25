// Print engine (P13, foundation §8.7). One module for the printable documents:
// the @page CSS for every paper size (A4 / A5 / 80 mm thermal), and the letterhead
// building blocks (logo, school name, address). Each document composes these in
// its own layout so the rendered DOM is unchanged — receipts and report cards move
// onto this without altering their output. Later phases add hall tickets, seating
// charts, salary slips and notices using the same kit.
//
// "Printed" only ever means the print dialog opened (§3 rule 13); nothing here
// claims a page was physically printed.

import type { CSSProperties } from 'react'
import logoLight from '@/assets/vidya-horizontal-on-light.svg'

export type PaperSize = 'a4' | 'a5' | '80mm'

/** The @page + @media-print CSS for a paper size. A4/A5 → 10 mm margins, 80 mm
 * thermal → 4 mm. The `.no-print` toolbar is hidden and the page background is
 * forced white when printing. */
export function pageCss(size: PaperSize): string {
  const page =
    size === '80mm'
      ? '@page { size: 80mm auto; margin: 4mm; }'
      : size === 'a5'
        ? '@page { size: A5; margin: 10mm; }'
        : '@page { size: A4; margin: 10mm; }'
  return `${page}
@media print { .no-print { display: none !important; } body { background: #ffffff; } }`
}

/** The school logo (light lockup, for print on white). English lockup until the
 * owner supplies a Telugu one (OWNER-DECISIONS #6). */
export function PrintLogo({ width, height, alt }: { width: number; height: number; alt?: string }) {
  return <img src={logoLight} alt={alt ?? ''} width={width} height={height} style={{ width: `${width}px`, height: `${height}px` }} />
}

/** The school name line. */
export function PrintSchoolName({ name, size }: { name: string; size: number }) {
  return <div style={{ fontWeight: 600, fontSize: `${size}px` }}>{name}</div>
}

/** The school address line (nothing when there is no address). */
export function PrintAddress({ address }: { address?: string | null }) {
  return address ? <div style={{ fontSize: '11px', color: 'var(--muted)' }}>{address}</div> : null
}

const toolbarBtnGhost: CSSProperties = { height: '38px', padding: '0 16px', borderRadius: '6px', border: '1px solid var(--line-strong)', background: 'var(--surface)', color: 'var(--ink)', fontSize: '13px', cursor: 'pointer' }
const toolbarBtnPrimary: CSSProperties = { height: '38px', padding: '0 18px', borderRadius: '6px', border: '1px solid var(--accent)', background: 'var(--accent)', color: 'var(--white)', fontSize: '13px', fontWeight: 500, cursor: 'pointer' }

/** The non-printed toolbar (Back + Print). `style` lets a document keep its exact
 * wrapper (e.g. the report card's centred max-width). */
export function PrintToolbar({ backLabel, printLabel, onBack, onPrint, style }: {
  backLabel: string
  printLabel: string
  onBack: () => void
  onPrint: () => void
  style?: CSSProperties
}) {
  return (
    <div className="no-print" style={{ display: 'flex', gap: '10px', marginBottom: '16px', ...style }}>
      <button type="button" onClick={onBack} style={toolbarBtnGhost}>{backLabel}</button>
      <button type="button" onClick={onPrint} style={toolbarBtnPrimary}>{printLabel}</button>
    </div>
  )
}
