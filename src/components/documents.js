// Print-document builders (receipts, headers). Rendered via ui.printHtml.
import { html } from '../core/html.js';
import { t } from '../core/i18n.js';
import { formatDate, formatRupees } from '../core/format.js';

export function schoolHeader(s) {
  return html`<div
    style="text-align:center;border-bottom:2px solid #000;padding-bottom:8px;margin-bottom:10px"
  >
    <h2>${s.name}</h2>
    <div>${s.addr || ''}</div>
    <div style="font-size:11px">
      ${s.udise ? html`${t('doc.udise')} ${s.udise} | ` : ''}${s.board || ''} | ${t('doc.session')}
      ${s.session || ''}${s.phone ? ' | ' + s.phone : ''}
    </div>
  </div>`;
}

export function receiptCopy(r, label) {
  return html`<div class="p-copy">
    ${schoolHeader(r.school)}
    <div style="display:flex;justify-content:space-between;align-items:center">
      <b>${t('doc.feeReceipt')}</b><span style="font-size:11px">${label}</span>
    </div>
    ${r.cancelled ? html`<div style="text-align:center;margin:6px 0"><span class="stamp">${t('doc.cancelled')}</span></div>` : ''}
    <table style="margin:8px 0">
      <tr>
        <th>${t('doc.receiptNo')}</th>
        <td class="mono">${r.no}</td>
        <th>${t('doc.date')}</th>
        <td>${formatDate(r.date)}</td>
      </tr>
      <tr>
        <th>${t('doc.student')}</th>
        <td>${r.studentName}</td>
        <th>${t('doc.class')}</th>
        <td>${r.ck} (${t('doc.roll')} ${r.roll})</td>
      </tr>
      <tr>
        <th>${t('doc.father')}</th>
        <td>${r.father}</td>
        <th>${t('doc.admNo')}</th>
        <td>${r.adm}</td>
      </tr>
      <tr>
        <th>${t('doc.paidBy')}</th>
        <td>${r.mode}${r.ref ? ' (' + r.ref + ')' : ''}</td>
        <th>${t('doc.balanceAfter')}</th>
        <td>${formatRupees(r.balanceAfter)}</td>
      </tr>
      ${
        r.note
          ? html`<tr>
              <th>${t('doc.note')}</th>
              <td colspan="3">${r.note}</td>
            </tr>`
          : ''
      }
    </table>
    <div style="font-size:18px;font-weight:800;border-top:2px solid #000;padding-top:6px">
      ${t('doc.amount')}: ${formatRupees(r.amount)}
    </div>
    <div style="font-size:12px">${t('fees.rupeesWords', { words: r.amountWords })}</div>
    <div style="display:flex;justify-content:space-between;margin-top:26px;font-size:12px">
      <span>${t('doc.receivedBy')}: ${r.byName}</span><span>${t('doc.signature')} ____________</span>
    </div>
  </div>`;
}
