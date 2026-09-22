/**
 * Formatting helpers for Vidya (Indian locale conventions).
 *
 * Pure TypeScript. Uses only the built-in `Intl` API — no external libraries.
 *
 * Indian formats (docs/00-SYSTEM-CONTEXT.md §10, docs/01-MOCK-SPEC.md):
 * - Money: ₹ with lakh grouping, e.g. ₹6,84,200.
 * - Dates: "Wednesday, 23 September" (no year).
 * - Session: April–March shown "2026–27" (EN DASH).
 * - Mobiles: 10 digits starting 6–9 (digit extraction helper only).
 */

const RUPEE = '₹'; // ₹
const EN_DASH = '–'; // –

// Reusable Intl formatters. Indian grouping ("en-IN") gives lakh/crore commas.
const rupeesWhole = new Intl.NumberFormat('en-IN', {
  minimumFractionDigits: 0,
  maximumFractionDigits: 0,
});

const rupeesWithPaise = new Intl.NumberFormat('en-IN', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const dateLong = new Intl.DateTimeFormat('en-IN', {
  weekday: 'long',
  day: 'numeric',
  month: 'long',
});

/**
 * Format an integer amount of paise as an Indian rupee string.
 *
 * 100 paise = ₹1. Uses lakh grouping and a leading "₹".
 * - Whole rupees show no decimals: 68420000 → "₹6,84,200".
 * - A paise remainder shows exactly 2 decimals: 310050 → "₹3,100.50".
 * - Negatives place the minus before the ₹: -10000 → "-₹100".
 */
export function formatMoney(paise: number): string {
  // Normalise to an integer number of paise to avoid float artefacts.
  const totalPaise = Math.round(paise);
  const negative = totalPaise < 0;
  const absPaise = Math.abs(totalPaise);

  const rupees = Math.trunc(absPaise / 100);
  const remainderPaise = absPaise % 100;

  let body: string;
  if (remainderPaise === 0) {
    body = rupeesWhole.format(rupees);
  } else {
    // Format the exact rupee value (rupees + paise fraction) so grouping
    // applies to the integer part and we get precisely two decimal places.
    body = rupeesWithPaise.format(rupees + remainderPaise / 100);
  }

  return `${negative ? '-' : ''}${RUPEE}${body}`;
}

/**
 * Format a date as "Wednesday, 23 September" — weekday (long) + day (numeric)
 * + month (long), with NO year. A comma follows the weekday.
 */
export function formatDateLong(date: Date): string {
  const parts = dateLong.formatToParts(date);
  const get = (type: Intl.DateTimeFormatPartTypes): string =>
    parts.find((p) => p.type === type)?.value ?? '';

  const weekday = get('weekday');
  const day = get('day');
  const month = get('month');

  // Reassemble explicitly so the shape is always "<weekday>, <day> <month>",
  // regardless of the part order Intl produces on a given platform.
  return `${weekday}, ${day} ${month}`;
}

/**
 * Format an academic session (April–March) from its start year.
 *
 * Uses an EN DASH and a 2-digit end year: 2026 → "2026–27".
 */
export function formatSession(startYear: number): string {
  const endTwoDigits = String((startYear + 1) % 100).padStart(2, '0');
  return `${startYear}${EN_DASH}${endTwoDigits}`;
}

/**
 * Compact relative "ago" string between `from` and `now`.
 *
 * - < 60s   → "<n> s ago"        (e.g. "12 s ago")
 * - < 60min → "<n> min ago"      (e.g. "2 min ago")
 * - < 24h   → "1 hour ago" / "N hours ago"
 * - else    → "1 day ago" / "N days ago"
 *
 * Future timestamps (negative elapsed) clamp to "0 s ago".
 */
export function formatRelative(from: Date, now: Date = new Date()): string {
  const elapsedMs = now.getTime() - from.getTime();
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1000));

  if (totalSeconds < 60) {
    return `${totalSeconds} s ago`;
  }

  const minutes = Math.floor(totalSeconds / 60);
  if (minutes < 60) {
    return `${minutes} min ago`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours} ${hours === 1 ? 'hour' : 'hours'} ago`;
  }

  const days = Math.floor(hours / 24);
  return `${days} ${days === 1 ? 'day' : 'days'} ago`;
}

/**
 * Time-of-day greeting bucket used to render "Good morning/afternoon/evening".
 *
 * - hour < 12 → 'morning'
 * - hour < 17 → 'afternoon'
 * - else      → 'evening'
 */
export function greeting(date: Date): 'morning' | 'afternoon' | 'evening' {
  const hour = date.getHours();
  if (hour < 12) {
    return 'morning';
  }
  if (hour < 17) {
    return 'afternoon';
  }
  return 'evening';
}

/**
 * Keep only the digit characters of `str`, then truncate to `maxLen`.
 *
 * parseDigits('4a2b6', 3)     → "426"
 * parseDigits('12345678', 7)  → "1234567"
 */
export function parseDigits(str: string, maxLen: number): string {
  return str.replace(/\D/g, '').slice(0, maxLen);
}
