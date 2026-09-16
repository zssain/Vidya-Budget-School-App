// UI primitives: toast, modal, confirm dialog and print. Built on core/html.js
// so all content is escaped. Modals sit above the login and setup screens
// (see components.css: .mbg z-index).

import { html, raw, render, SafeHtml } from './html.js';
import { t } from './i18n.js';

function ensureHost(id) {
  let el = document.getElementById(id);
  if (!el) {
    el = document.createElement('div');
    el.id = id;
    document.body.appendChild(el);
  }
  return el;
}

/** Show a transient toast. kind: 'info' | 'ok' | 'error'. */
export function toast(message, { kind = 'info' } = {}) {
  const host = ensureHost('tst');
  const el = document.createElement('div');
  el.className = 'toast' + (kind === 'ok' ? ' ok' : kind === 'error' ? ' error' : '');
  el.textContent = message;
  host.appendChild(el);
  setTimeout(() => {
    el.style.transition = '.25s';
    el.style.opacity = '0';
    setTimeout(() => el.remove(), 260);
  }, 3200);
  return el;
}

let activeModal = null;

/**
 * Open a modal. body/footer are SafeHtml. Returns { close, element }.
 * Esc and backdrop close the modal unless `locked`. Focus moves into the modal
 * and returns to the previously focused element on close.
 */
export function openModal({
  title = '',
  subtitle = '',
  body = null,
  footer = null,
  locked = false,
  wide = false,
  onAction,
} = {}) {
  const host = ensureHost('mroot');
  const previouslyFocused = document.activeElement;

  const bg = document.createElement('div');
  bg.className = 'mbg';
  const modalEl = document.createElement('div');
  modalEl.className = 'modal' + (wide ? ' wide' : '');
  modalEl.setAttribute('role', 'dialog');
  modalEl.setAttribute('aria-modal', 'true');
  bg.appendChild(modalEl);

  const closeBtn = locked
    ? new SafeHtml('')
    : html`<button class="xbtn" data-ui="close" aria-label="${t('common.close')}">×</button>`;
  const headHtml = html`<div class="mh">
    <div>
      <h3>${title}</h3>
      ${subtitle ? html`<p>${subtitle}</p>` : ''}
    </div>
    ${closeBtn}
  </div>`;
  const bodyHtml = body ? html`<div class="mb">${raw(body)}</div>` : new SafeHtml('');
  const footHtml = footer ? html`<div class="mf">${raw(footer)}</div>` : new SafeHtml('');
  render(modalEl, html`${headHtml}${bodyHtml}${footHtml}`);

  host.replaceChildren(bg);

  const onKey = (e) => {
    if (e.key === 'Escape' && !locked) close();
  };

  function close() {
    document.removeEventListener('keydown', onKey);
    if (host.contains(bg)) host.removeChild(bg);
    if (activeModal && activeModal.close === close) activeModal = null;
    if (previouslyFocused && typeof previouslyFocused.focus === 'function') previouslyFocused.focus();
  }

  document.addEventListener('keydown', onKey);
  bg.addEventListener('click', (e) => {
    if (e.target === bg && !locked) {
      close();
      return;
    }
    const closer = e.target.closest && e.target.closest('[data-ui="close"]');
    if (closer) {
      close();
      return;
    }
    const act = e.target.closest && e.target.closest('[data-action]');
    if (act && typeof onAction === 'function') {
      onAction(act.getAttribute('data-action'), { event: e, element: act, close });
    }
  });

  const focusEl =
    modalEl.querySelector('[autofocus]') || modalEl.querySelector('button, [href], input, select, textarea');
  if (focusEl) setTimeout(() => focusEl.focus(), 30);

  activeModal = { close, element: modalEl, locked };
  return { close, element: modalEl };
}

export function closeModal() {
  if (activeModal) activeModal.close();
}

/** Promise-based confirm dialog. Resolves true (confirm) or false (cancel). */
export function confirmDialog({ title = '', message = '', confirmLabel, danger = false } = {}) {
  return new Promise((resolve) => {
    const label = confirmLabel || t('common.ok');
    const confirmBtn = danger
      ? html`<button class="btn" style="background:var(--red);color:#fff" data-ui="confirm">${label}</button>`
      : html`<button class="btn b-pri" data-ui="confirm">${label}</button>`;
    const footer = html`<button class="btn b-quiet" data-ui="cancel">${t('common.cancel')}</button
      >${confirmBtn}`;
    const body = html`<p style="margin:0">${message}</p>`;
    const { close, element } = openModal({ title, body, footer, locked: true });
    element.addEventListener('click', (e) => {
      const btn = e.target.closest && e.target.closest('[data-ui]');
      if (!btn) return;
      const confirmed = btn.getAttribute('data-ui') === 'confirm';
      close();
      resolve(confirmed);
    });
  });
}

/** Print a SafeHtml document (wrapped in .p-doc, using print.css). */
export function printHtml(safe) {
  const area = ensureHost('printArea');
  render(area, html`<div class="p-doc">${raw(safe)}</div>`);
  document.body.classList.add('printing');
  const done = () => {
    document.body.classList.remove('printing');
    area.replaceChildren();
    window.removeEventListener('afterprint', done);
  };
  window.addEventListener('afterprint', done);
  setTimeout(() => {
    try {
      window.print();
    } catch {
      toast('Printing is blocked here. Open the file in Chrome or Edge.', { kind: 'error' });
      done();
    }
  }, 60);
}
