import { describe, expect, it } from 'vitest';
import { escapeHtml, html, raw, render, SafeHtml } from './html.js';

describe('escapeHtml', () => {
  it('escapes all five characters', () => {
    expect(escapeHtml(`& < > " '`)).toBe('&amp; &lt; &gt; &quot; &#39;');
  });
});

describe('html', () => {
  it('makes a <script> value inert', () => {
    const out = html`<div>${'<script>alert(1)</script>'}</div>`.value;
    expect(out).toBe('<div>&lt;script&gt;alert(1)&lt;/script&gt;</div>');
    expect(out).not.toContain('<script>');
  });

  it('renders numbers, including 0', () => {
    expect(html`${0}`.value).toBe('0');
    expect(html`${999}`.value).toBe('999');
  });

  it('renders null, undefined and false as empty', () => {
    expect(html`${null}${undefined}${false}`.value).toBe('');
  });

  it('joins arrays of html results unescaped', () => {
    const items = [html`<b>a</b>`, html`<i>b</i>`];
    expect(html`${items}`.value).toBe('<b>a</b><i>b</i>');
  });

  it('escapes quotes inside attribute values', () => {
    expect(html`<div title="${'a"b'}"></div>`.value).toBe('<div title="a&quot;b"></div>');
  });

  it('returns a SafeHtml', () => {
    expect(html`x`).toBeInstanceOf(SafeHtml);
  });
});

describe('raw', () => {
  it('passes SafeHtml through and wraps strings', () => {
    expect(raw(html`<b>x</b>`).value).toBe('<b>x</b>');
    expect(raw('<b>x</b>')).toBeInstanceOf(SafeHtml);
  });
  it('rejects other values', () => {
    expect(() => raw(42)).toThrow();
  });
});

describe('render', () => {
  it('sets contents from SafeHtml, escaping interpolated values', () => {
    const el = document.createElement('div');
    render(el, html`<span>${'<x>'}</span>`);
    const span = el.querySelector('span');
    expect(span).not.toBeNull();
    // The escaped value becomes literal text, not a real <x> element.
    expect(span.textContent).toBe('<x>');
    expect(el.querySelector('x')).toBeNull();
  });
  it('throws on a plain string', () => {
    const el = document.createElement('div');
    expect(() => render(el, '<b>x</b>')).toThrow();
  });
});
