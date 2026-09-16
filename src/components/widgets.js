// Small reusable, presentational components. They receive already-formatted
// text (money/dates formatted by the caller) and use t() for fixed labels.
import { html } from '../core/html.js';
import { t } from '../core/i18n.js';
import { formatRupees } from '../core/format.js';

export function pill(text, kind = 'grey') {
  return html`<span class="pill p-${kind}">${text}</span>`;
}

export function statCard(label, value, note, color) {
  const style = color ? `color:${color}` : '';
  return html`<div class="stat">
    <div class="k">${label}</div>
    <div class="v num" style="${style}">${value}</div>
    <div class="n">${note}</div>
  </div>`;
}

export function emptyState(icon, title, sub) {
  return html`<div class="empty">
    <div class="e">${icon}</div>
    <b>${title}</b>
    ${sub ? html`<div class="sm">${sub}</div>` : ''}
  </div>`;
}

export function bar(pct, color) {
  const width = Math.max(0, Math.min(100, pct));
  return html`<div class="bar"><i style="width:${width}%;background:${color}"></i></div>`;
}

/** A fee-status pill. `state` is 'rte'|'paid'|'part'|'due'; balance is a number. */
export function feePill(state, balance) {
  if (state === 'rte')
    return html`<span
      class="pill"
      style="background:var(--purple-l);color:var(--purple);border:1px solid #DCCBF2"
      >${t('fees.pill.rteFree')}</span
    >`;
  if (state === 'paid') return html`<span class="pill p-green">✓ ${t('fees.pill.paid')}</span>`;
  return html`<span class="pill ${state === 'part' ? 'p-orange' : 'p-red'}"
    >${formatRupees(balance)} ${t('fees.pill.due')}</span
  >`;
}

/** A row in the change/activity log. */
export function activityRow(c, agoText) {
  const col =
    {
      fee: 'var(--orange)',
      att: 'var(--green)',
      mark: 'var(--blue)',
      stu: 'var(--purple)',
      user: 'var(--purple)',
      auth: 'var(--ink4)',
      setup: 'var(--blue)',
      settings: 'var(--ink3)',
      backup: 'var(--green)',
    }[c.kind] || 'var(--ink4)';
  return html`<div class="act">
    <span class="dot" style="background:${col}"></span>
    <div class="grow">
      <div class="sm">${c.text}</div>
      <div class="xs mut">${c.who} | ${agoText} | ${t('activity.device')} ${c.device}</div>
    </div>
  </div>`;
}
