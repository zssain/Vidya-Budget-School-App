// Receipt modal, print (2 copies) and cancellation. Ported from the prototype.
import { html } from '../../core/html.js';
import { t } from '../../core/i18n.js';
import { formatDateTime, formatRupees } from '../../core/format.js';
import { openModal, printHtml, toast } from '../../core/ui.js';
import { rerender } from '../../core/router.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { receiptCopy } from '../../components/documents.js';

function printReceipt(r) {
  printHtml(
    html`${receiptCopy(r, t('fees.parentCopy'))}
      <div class="p-cut">${t('fees.cutHere')}</div>
      ${receiptCopy(r, t('fees.schoolCopy'))}`,
  );
}

export async function showReceipt(receiptId, opts = {}) {
  let r;
  try {
    r = await commands.getReceipt({ receiptId });
  } catch (e) {
    return toast(toAppError(e).message, { kind: 'error' });
  }
  const canCancel = opts.me && opts.me.role === 'principal' && !r.cancelled;
  const { element, close } = openModal({
    title: opts.fresh ? t('fees.paymentSaved') : t('fees.receiptTitle', { no: r.no }),
    subtitle: opts.fresh ? t('fees.receiptNo', { no: r.no }) : formatDateTime(r.at),
    body: html`${
        r.cancelled
          ? html`<div class="note n-red" style="margin-bottom:12px">
              ${t('fees.cancelledBy', { name: r.cancelled.byName, at: formatDateTime(r.cancelled.at), reason: r.cancelled.reason })}
            </div>`
          : ''
      }
      <div class="receipt">
        <div class="rline">
          <span class="mut">${t('fees.student')}</span><b>${r.studentName} (${r.ck})</b>
        </div>
        <div class="rline">
          <span class="mut">${t('fees.paidBy')}</span><span>${r.mode}${r.ref ? ' | ' + r.ref : ''}</span>
        </div>
        <div class="rline">
          <span class="mut">${t('fees.receivedBy')}</span><span>${r.byName} · ${r.device}</span>
        </div>
        <div class="rline">
          <span class="mut">${t('fees.balanceAfter')}</span
          ><span class="num">${formatRupees(r.balanceAfter)}</span>
        </div>
        <div class="rtot">
          <span>${t('fees.amount')}</span><span class="num">${formatRupees(r.amount)}</span>
        </div>
        <div class="xs mut" style="margin-top:4px">${t('fees.rupeesWords', { words: r.amountWords })}</div>
      </div>`,
    footer: html`${
        canCancel
          ? html`<button
              class="btn b-quiet"
              style="color:var(--red);margin-right:auto"
              data-ui="cancel-receipt"
            >
              ${t('fees.cancelReceipt')}
            </button>`
          : ''
      }
      <button class="btn b-quiet" data-ui="close2">${t('common.close')}</button>
      <button class="btn b-pri" data-ui="print">🖨 ${t('fees.print2')}</button>`,
  });
  element.addEventListener('click', (e) => {
    const b = e.target.closest && e.target.closest('[data-ui]');
    if (!b) return;
    const act = b.getAttribute('data-ui');
    if (act === 'close2') close();
    else if (act === 'print') printReceipt(r);
    else if (act === 'cancel-receipt') {
      close();
      cancelReceiptFlow(r, opts);
    }
  });
}

function cancelReceiptFlow(r, opts) {
  const { element, close } = openModal({
    title: t('fees.cancelTitle', { no: r.no }),
    subtitle: t('fees.cancelSubtitle'),
    body: html`<label class="lbl">${t('fees.reasonLabel')}</label>
      <input class="inp" id="cxWhy" maxlength="120" placeholder="${t('fees.reasonPlaceholder')}" />
      <div class="err" id="cxErr"></div>`,
    footer: html`<button class="btn b-quiet" data-ui="back">${t('common.back')}</button>
      <button class="btn" style="background:var(--red);color:#fff" data-ui="do-cancel">
        ${t('fees.cancelReceipt')}
      </button>`,
  });
  element.addEventListener('click', async (e) => {
    const b = e.target.closest && e.target.closest('[data-ui]');
    if (!b) return;
    if (b.getAttribute('data-ui') === 'back') {
      close();
      return showReceipt(r.no, opts);
    }
    if (b.getAttribute('data-ui') !== 'do-cancel') return;
    const reason = element.querySelector('#cxWhy').value;
    try {
      await commands.cancelReceipt({ receiptId: r.no, reason });
      toast(t('fees.receiptCancelled'), { kind: 'ok' });
      close();
      rerender();
    } catch (ex) {
      element.querySelector('#cxErr').textContent = toAppError(ex).message;
    }
  });
}
