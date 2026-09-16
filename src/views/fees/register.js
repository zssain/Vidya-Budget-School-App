// Fees register (principal and accountant). Ported from the prototype.
import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import { formatRupees } from '../../core/format.js';
import { go } from '../../core/router.js';
import { toast } from '../../core/ui.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { statCard, feePill } from '../../components/widgets.js';
import { downloadCsv } from '../../core/download.js';
import { openCollectPicker, openCollect } from './collect.js';
import { openDayBook } from './daybook.js';

const filter = { q: '', sectionId: 'All', state: 'due' };

export async function view(root, params) {
  const d = await commands.feeRegister(filter);
  const states = [
    ['due', t('fees.filter.dues')],
    ['paid', t('fees.filter.fullyPaid')],
    ['rte', t('fees.filter.rte')],
    ['all', t('fees.filter.all')],
  ];
  render(
    root,
    html`<div class="inner stack">
      <div class="spread wrap g12">
        <div>
          <h1 style="font-size:21px;font-weight:750">${t('nav.fees')}</h1>
          <p class="sm mut" style="margin:2px 0 0">
            ${t('fees.sessionInfo', { session: d.session, terms: d.terms, prefix: d.devicePrefix })}
          </p>
        </div>
        <div class="row g8 wrap">
          <button class="btn b-out" data-action="daybook">🧾 ${t('fees.dayBook')}</button>
          <button class="btn b-out" data-action="dues">⬇ ${t('fees.duesList')}</button>
          <button class="btn b-pri" data-action="collect">₹ ${t('common.collectFee')}</button>
        </div>
      </div>
      <div class="stats">
        ${statCard(t('fees.totalDueSession'), formatRupees(d.totals.due), t('fees.nPaying', { n: d.totals.payingCount }))}
        ${statCard(t('fees.collected'), formatRupees(d.totals.paid), t('fees.pctCollected', { pct: d.totals.due ? Math.round((d.totals.paid / d.totals.due) * 100) : 0 }), 'var(--green)')}
        ${statCard(t('fees.pending'), formatRupees(d.totals.pending), t('fees.nStudents', { n: d.totals.unpaid + d.totals.part }), 'var(--red)')}
        ${statCard(t('home.collectedToday'), formatRupees(d.collectedToday), t('fees.nReceipts', { n: d.receiptsToday }), 'var(--blue)')}
      </div>
      <div class="card">
        <div class="cb" style="padding:13px">
          <input
            class="inp"
            id="feeQ"
            placeholder="${t('fees.searchPlaceholder')}"
            value="${filter.q}"
            style="margin-bottom:11px"
          />
          <div class="row g10 wrap">
            <div class="chipbar">
              ${states.map(([k, label]) => html`<button class="chip ${filter.state === k ? 'on' : ''}" data-action="state" data-state="${k}">${label}</button>`)}
            </div>
          </div>
        </div>
      </div>
      <div class="card scroll-x">
        ${
          d.list.length
            ? html`<table class="tbl">
                <thead>
                  <tr>
                    <th>${t('fees.student')}</th>
                    <th style="width:80px">${t('students.class')}</th>
                    <th style="width:110px;text-align:right">${t('fees.due')}</th>
                    <th style="width:110px;text-align:right">${t('fees.paid')}</th>
                    <th style="width:120px">${t('fees.balance')}</th>
                    <th style="width:100px"></th>
                  </tr>
                </thead>
                <tbody>
                  ${d.list.slice(0, 200).map(
                    (s) =>
                      html`<tr>
                        <td class="tap" data-action="open" data-adm="${s.adm}">
                          <b>${s.name}</b>
                          <div class="xs mut">${s.adm} | ${s.father} | ${s.mobile}</div>
                        </td>
                        <td><span class="pill p-blue">${s.ck}</span></td>
                        <td class="num" style="text-align:right">${formatRupees(s.due)}</td>
                        <td class="num" style="text-align:right">${formatRupees(s.paid)}</td>
                        <td>${feePill(s.feeState, s.balance)}</td>
                        <td>
                          ${!s.rte && s.balance > 0 ? html`<button class="btn b-pri b-sm" data-action="collect-one" data-adm="${s.adm}">${t('home.collect')}</button>` : ''}
                        </td>
                      </tr>`,
                  )}
                </tbody>
              </table>`
            : html`<div class="empty">
                <div class="e">✅</div>
                <b>${t('fees.noMatch')}</b>
              </div>`
        }
      </div>
    </div>`,
  );

  delegate(root, {
    daybook: () => openDayBook(),
    collect: () => openCollectPicker(),
    'collect-one': (e, el) => openCollect(el.getAttribute('data-adm')),
    open: (e, el) => go('student', { adm: el.getAttribute('data-adm') }),
    state: (e, el) => {
      filter.state = el.getAttribute('data-state');
      view(root, params);
    },
    dues: async () => {
      try {
        const x = await commands.exportDuesXlsx({});
        downloadCsv(x.filename, x.rows);
        toast(t('fees.duesDownloaded'), { kind: 'ok' });
      } catch (e) {
        toast(toAppError(e).message, { kind: 'error' });
      }
    },
  });
  const q = root.querySelector('#feeQ');
  if (q)
    q.addEventListener('input', () => {
      filter.q = q.value;
      clearTimeout(q._t);
      q._t = setTimeout(() => view(root, params), 250);
    });
}
