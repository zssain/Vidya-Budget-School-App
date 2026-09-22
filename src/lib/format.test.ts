import { describe, it, expect } from 'vitest';
import {
  formatMoney,
  formatDateLong,
  formatSession,
  formatRelative,
  greeting,
  parseDigits,
} from './format';

describe('formatMoney', () => {
  it('formats whole rupees with lakh grouping and no decimals', () => {
    expect(formatMoney(68420000)).toBe('₹6,84,200');
  });

  it('formats zero as ₹0', () => {
    expect(formatMoney(0)).toBe('₹0');
  });

  it('formats a small whole amount', () => {
    expect(formatMoney(310000)).toBe('₹3,100');
  });

  it('shows 2 decimals when there is a paise remainder', () => {
    expect(formatMoney(310050)).toBe('₹3,100.50');
  });

  it('pads a single-digit paise remainder to two places', () => {
    // ₹3,100 and 5 paise → "₹3,100.05"
    expect(formatMoney(310005)).toBe('₹3,100.05');
  });

  it('formats a single rupee', () => {
    expect(formatMoney(100)).toBe('₹1');
  });

  it('formats a sub-rupee (paise-only) amount', () => {
    expect(formatMoney(50)).toBe('₹0.50');
  });

  it('applies lakh grouping for large whole amounts', () => {
    // 1,00,00,000 paise = ₹1,00,000
    expect(formatMoney(10000000)).toBe('₹1,00,000');
    // 1,00,00,00,000 paise = ₹1,00,00,000 (one crore)
    expect(formatMoney(1000000000)).toBe('₹1,00,00,000');
  });

  it('places the minus before the ₹ for negatives', () => {
    expect(formatMoney(-10000)).toBe('-₹100');
    expect(formatMoney(-310050)).toBe('-₹3,100.50');
  });
});

describe('formatDateLong', () => {
  it('renders "Weekday, DD Month" with no year', () => {
    // 23 Sep 2026 is a Wednesday (month is 0-indexed → 8).
    const date = new Date(2026, 8, 23, 9, 0, 0);
    expect(formatDateLong(date)).toBe('Wednesday, 23 September');
  });

  it('includes a comma after the weekday', () => {
    const date = new Date(2026, 8, 23, 9, 0, 0);
    expect(formatDateLong(date)).toContain(', ');
  });

  it('omits the year for another date', () => {
    // 1 Jan 2026 is a Thursday.
    const date = new Date(2026, 0, 1, 12, 0, 0);
    expect(formatDateLong(date)).toBe('Thursday, 1 January');
    expect(formatDateLong(date)).not.toContain('2026');
  });
});

describe('formatSession', () => {
  it('formats an April–March session with an EN DASH and 2-digit end year', () => {
    expect(formatSession(2026)).toBe('2026–27');
    expect(formatSession(2026)).toBe('2026–27');
  });

  it('pads the end year to two digits across a century boundary', () => {
    // 2099 → next year 2100, last two digits "00".
    expect(formatSession(2099)).toBe('2099–00');
  });

  it('uses an EN DASH (U+2013), not a hyphen', () => {
    const result = formatSession(2024);
    expect(result).toBe('2024–25');
    expect(result).toContain('–');
    expect(result).not.toContain('-'); // ASCII hyphen-minus
  });
});

describe('formatRelative', () => {
  const base = new Date(2026, 8, 23, 12, 0, 0);
  const secondsAgo = (n: number): Date => new Date(base.getTime() - n * 1000);

  it('formats seconds', () => {
    expect(formatRelative(secondsAgo(12), base)).toBe('12 s ago');
  });

  it('formats minutes', () => {
    expect(formatRelative(secondsAgo(120), base)).toBe('2 min ago');
  });

  it('formats hours (plural)', () => {
    expect(formatRelative(secondsAgo(5 * 3600), base)).toBe('5 hours ago');
  });

  it('formats a single hour (singular)', () => {
    expect(formatRelative(secondsAgo(3600), base)).toBe('1 hour ago');
  });

  it('formats a single day (singular)', () => {
    expect(formatRelative(secondsAgo(24 * 3600), base)).toBe('1 day ago');
  });

  it('formats multiple days (plural)', () => {
    expect(formatRelative(secondsAgo(2 * 24 * 3600), base)).toBe('2 days ago');
  });

  it('uses seconds at the boundary just under a minute', () => {
    expect(formatRelative(secondsAgo(59), base)).toBe('59 s ago');
  });

  it('switches to minutes at exactly 60 seconds', () => {
    expect(formatRelative(secondsAgo(60), base)).toBe('1 min ago');
  });

  it('switches to hours at exactly 60 minutes', () => {
    expect(formatRelative(secondsAgo(3600), base)).toBe('1 hour ago');
  });

  it('clamps future timestamps to "0 s ago"', () => {
    const future = new Date(base.getTime() + 5000);
    expect(formatRelative(future, base)).toBe('0 s ago');
  });

  it('reports "0 s ago" for the same instant', () => {
    expect(formatRelative(base, base)).toBe('0 s ago');
  });
});

describe('greeting', () => {
  it('returns morning before noon', () => {
    expect(greeting(new Date(2026, 8, 23, 0, 0, 0))).toBe('morning');
    expect(greeting(new Date(2026, 8, 23, 9, 0, 0))).toBe('morning');
    expect(greeting(new Date(2026, 8, 23, 11, 59, 59))).toBe('morning');
  });

  it('returns afternoon from noon until 5pm', () => {
    expect(greeting(new Date(2026, 8, 23, 12, 0, 0))).toBe('afternoon');
    expect(greeting(new Date(2026, 8, 23, 16, 59, 59))).toBe('afternoon');
  });

  it('returns evening from 5pm onward', () => {
    expect(greeting(new Date(2026, 8, 23, 17, 0, 0))).toBe('evening');
    expect(greeting(new Date(2026, 8, 23, 23, 59, 59))).toBe('evening');
  });
});

describe('parseDigits', () => {
  it('keeps digits only then slices to maxLen', () => {
    expect(parseDigits('4a2b6', 3)).toBe('426');
  });

  it('truncates a long digit string to maxLen', () => {
    expect(parseDigits('12345678', 7)).toBe('1234567');
  });

  it('returns an empty string when there are no digits', () => {
    expect(parseDigits('abc-def', 5)).toBe('');
  });

  it('strips spaces, plus signs and punctuation from a mobile number', () => {
    expect(parseDigits('+91 98765-43210', 10)).toBe('9198765432');
  });

  it('returns everything when shorter than maxLen', () => {
    expect(parseDigits('42', 10)).toBe('42');
  });
});
