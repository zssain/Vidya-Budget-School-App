// Day book modal and print. Ported from the prototype.
import { html, render } from '../../core/html.js';
import { t } from '../../core/i18n.js';
import { formatDate, formatRupees, formatTime } from '../../core/format.js';
import { openModal, printHtml, toast } from '../../core/ui.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { statCard } from '../../components/widgets.js';
import { schoolHeader } from '../../components/documents.js';

function printDayBook(book) {
  printHtml(
    html`${schoolHeader(book.school)}
      <h3 style="margin:6px 0">${t('fees.dayBookOn', { date: formatDate(book.date) })}</h3>
      <table>
        <thead>
          <tr>
            <th>${t('doc.receiptNo')}</th>
            <th>${t('doc.time')}</th>
            <th>${t('doc.student')}</th>
            <th>${t('doc.class')}</th>
            <th>${t('fees.mode')}</th>
            <th>${t('fees.reference')}</th>
            <th>${t('doc.by')}</th>
            <th style="text-align:right">${t('doc.amount')}</th>
          </tr>
        </thead>
        <tbody>
          ${book.receipts.map(
            (r) =>
              html`<tr style="${r.cancelled ? 'text-decoration:line-through' : ''}">
                <td>${r.no}</td>
                <td>${formatTime(r.at)}</td>
                <td>${r.studentName}</td>
                <td>${r.ck}</td>
                <td>${r.mode}</td>
                <td>${r.ref}</td>
                <td>${r.byName}</td>
                <td style="text-align:right">${formatRupees(r.amount)}</td>
              </tr>`,
          )}
        </tbody>
      </table>`,
  );
}

export async function openDayBook(date) {
  async function load(d) {
    let book;
    try {
      book = await commands.dayBook(d ? { date: d } : {});
    } catch (e) {
      return toast(toAppError(e).message, { kind: 'error' });
    }
    const body = html`<input
        class="inp"
        type="date"
        id="dbDate"
        value="${book.date}"
        max="${todayKey()}"
        aria-label="${t('doc.date')}"
      />
      <div class="stats" style="grid-template-columns:repeat(2,1fr);margin-top:12px">
        ${book.modes.map((m) => statCard(m.mode, formatRupees(m.total), t('fees.nReceipts', { n: m.count })))}
        ${statCard(t('fees.total'), formatRupees(book.total), t('fees.nReceipts', { n: book.count }), 'var(--green)')}
      </div>
      <div class="card" style="margin-top:12px">
        ${
          book.receipts.length
            ? book.receipts.map(
                (r) =>
                  html`<div class="feerow">
                    <div class="grow ${r.cancelled ? 'cancelled' : ''}">
                      <b class="sm">${r.studentName}</b>
                      <div class="xs mut">${r.no} | ${r.mode} | ${r.byName}</div>
                    </div>
                    ${r.cancelled ? html`<span class="pill p-red">${t('fees.cancelled')}</span>` : ''}
                    <b class="num ${r.cancelled ? 'cancelled' : ''}">${formatRupees(r.amount)}</b>
                  </div>`,
              )
            : html`<div class="empty">${t('fees.noReceiptsToday')}</div>`
        }
      </div>`;
    if (current) {
      render(current.element.querySelector('.mb'), body);
      wire(current, book);
    } else {
      current = openModal({
        title: t('fees.dayBookTitle'),
        subtitle: formatDate(book.date),
        wide: true,
        body,
        footer: html`<button class="btn b-quiet" data-ui="close2">${t('common.close')}</button>
          <button class="btn b-pri" data-ui="print">🖨 ${t('common.print')}</button>`,
      });
      wire(current, book);
    }
  }
  let current = null;
  function wire(modal, book) {
    const dateInput = modal.element.querySelector('#dbDate');
    if (dateInput)
      dateInput.onchange = () => {
        if (dateInput.value) load(dateInput.value);
      };
    modal.element.onclick = (e) => {
      const b = e.target.closest && e.target.closest('[data-ui]');
      if (!b) return;
      if (b.getAttribute('data-ui') === 'close2') modal.close();
      else if (b.getAttribute('data-ui') === 'print') printDayBook(book);
    };
  }
  await load(date);
}

function todayKey() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}
