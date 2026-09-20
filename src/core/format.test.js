import { describe, expect, it } from 'vitest';
import { formatDate, formatRupees, timeAgo } from './format.js';
import { telHref } from './links.js';
describe('formatting', () => {
  it.each([
    [0, '₹0'],
    [999, '₹999'],
    [1000, '₹1,000'],
    [100000, '₹1,00,000'],
    [12345678, '₹1,23,45,678'],
  ])('formats rupees', (value, expected) => expect(formatRupees(value)).toBe(expected));
  it('formats dates and relative time', () => {
    expect(formatDate('2026-09-15')).toBe('15 Sep 2026');
    expect(timeAgo('2026-09-16T00:00:00Z', new Date('2026-09-17T00:00:00Z'))).toBe('1 day ago');
  });
  it('only makes safe telephone links', () => {
    expect(telHref('9876543210')).toBe('tel:+919876543210');
    for (const bad of ['javascript:alert(1)', '98765 43210', 'abcdefghij']) expect(telHref(bad)).toBeNull();
  });
});
