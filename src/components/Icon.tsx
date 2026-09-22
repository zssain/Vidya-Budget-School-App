import type { SVGProps } from 'react';
import { icons, type IconName, type IconPart } from '@/lib/icons';

export interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'stroke'> {
  /** Which icon to draw (see docs/01-MOCK-SPEC.md §6). */
  name: IconName;
  /** Rendered box, in px. Icons are square. Defaults to 18 (sidebar/header). */
  size?: number;
  /** Stroke width: 1.6 sidebar/header, 1.75 buttons, 1.3 tiles, 2–2.2 checks. */
  strokeWidth?: number;
  /** Overrides the mono stroke colour. Two-tone parts keep their own stroke. */
  color?: string;
  /** Accessible label. When set, the icon is exposed as an image, not hidden. */
  title?: string;
  'aria-label'?: string;
}

function renderPart(part: IconPart, index: number) {
  const stroke = 'stroke' in part ? part.stroke : undefined;
  if ('rect' in part) {
    const { x, y, width, height, rx } = part.rect;
    return <rect key={index} x={x} y={y} width={width} height={height} rx={rx} stroke={stroke} />;
  }
  if ('circle' in part) {
    const { cx, cy, r } = part.circle;
    return <circle key={index} cx={cx} cy={cy} r={r} stroke={stroke} />;
  }
  return <path key={index} d={part.d} stroke={stroke} />;
}

/**
 * The one icon primitive. Copies the mock's inline-SVG frame exactly; each part
 * with its own `stroke` (two-tone tiles) overrides the shared stroke colour.
 */
export function Icon({
  name,
  size = 18,
  strokeWidth = 1.6,
  color,
  title,
  'aria-label': ariaLabel,
  ...rest
}: IconProps) {
  const label = title ?? ariaLabel;
  const labelled = label != null && label !== '';
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color ?? 'currentColor'}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      role={labelled ? 'img' : undefined}
      aria-label={labelled ? label : undefined}
      aria-hidden={labelled ? undefined : true}
      {...rest}
    >
      {labelled && title != null && title !== '' ? <title>{title}</title> : null}
      {icons[name].map(renderPart)}
    </svg>
  );
}
