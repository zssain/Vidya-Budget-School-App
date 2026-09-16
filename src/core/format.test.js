import { describe, expect, it } from 'vitest';
import { formatDate, formatDateTime, formatRupees, formatTime, timeAgo } from './format.js';

describe('formatRupees', () => {
  it('formats with Indian grouping', () => {
    expect(formatRupees(0)).toBe('₹0');
    expect(formatRupees(999)).toBe('₹999');
    expect(formatRupees(1000)).toBe('₹1,000');
    expect(formatRupees(100000)).toBe('₹1,00,000');
    expect(formatRupees(12345678)).toBe('₹1,23,45,678');
  });
  it('rounds and coerces', () => {
    expect(formatRupees('500')).toBe('₹500');
    expect(formatRupees(499.6)).toBe('₹500');
    expect(formatRupees(null)).toBe('₹0');
  });
});

describe('formatDate', () => {
  it('formats YYYY-MM-DD', () => {
    expect(formatDate('2026-09-15')).toBe('15 Sep 2026');
    expect(formatDate('2026-01-01')).toBe('1 Jan 2026');
    expect(formatDate('')).toBe('—');
  });
});

describe('formatTime / formatDateTime', () => {
  it('formats local time as h:mm am/pm', () => {
    expect(formatTime(new Date(2026, 8, 15, 14, 30))).toBe('2:30 pm');
    expect(formatTime(new Date(2026, 8, 15, 0, 5))).toBe('12:05 am');
    expect(formatTime(new Date(2026, 8, 15, 12, 0))).toBe('12:00 pm');
  });
  it('combines date and time', () => {
    expect(formatDateTime(new Date(2026, 8, 15, 14, 30))).toBe('15 Sep 2026, 2:30 pm');
  });
});

describe('timeAgo', () => {
  const now = new Date('2026-09-15T12:00:00Z').getTime();
  it('describes relative time with a fixed now', () => {
    expect(timeAgo(null, now)).toBe('never');
    expect(timeAgo(new Date(now - 10_000).toISOString(), now)).toBe('just now');
    expect(timeAgo(new Date(now - 5 * 60_000).toISOString(), now)).toBe('5 min ago');
    expect(timeAgo(new Date(now - 3 * 3600_000).toISOString(), now)).toBe('3 hr ago');
    expect(timeAgo(new Date(now - 24 * 3600_000).toISOString(), now)).toBe('1 day ago');
    expect(timeAgo(new Date(now - 3 * 86400_000).toISOString(), now)).toBe('3 days ago');
  });
});
