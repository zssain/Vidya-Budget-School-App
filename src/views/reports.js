import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { formatRupees } from '../core/format.js';
import { toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';
import { bar, statCard } from '../components/widgets.js';
import { downloadCsv } from '../core/download.js';

export async function view(root) {
  const r = await commands.reportsSummary();
  render(
    root,
    html`<div class="inner stack">
      <div class="spread wrap g12">
        <div>
          <h1 style="font-size:21px;font-weight:750">${t('nav.reports')}</h1>
          <p class="sm mut" style="margin:2px 0 0">${t('reports.subtitle')}</p>
        </div>
        <button class="btn b-out" data-action="export">⬇ ${t('reports.export')}</button>
      </div>
      <div class="stats">
        ${statCard(t('reports.enrolment'), r.enrolment, t('reports.genderCount', { boys: r.boys, girls: r.girls }))}
        ${statCard(t('reports.rte'), r.rte, t('reports.studentsLabel'))}
        ${statCard(t('reports.aadhaar'), r.aadhaarPct == null ? '—' : `${r.aadhaarPct}%`, t('reports.pendingCount', { n: r.aadhaarPending }))}
        ${statCard(t('reports.attendance30'), r.attendance30 == null ? '—' : `${r.attendance30}%`, t('reports.marksRecorded', { n: r.attendanceMarks }), 'var(--blue)')}
      </div>
      <div class="card scroll-x">
        <div class="ch"><h3>${t('reports.classSummary')}</h3></div>
        <table class="tbl">
          <thead>
            <tr>
              <th>${t('students.class')}</th>
              <th>${t('reports.students')}</th>
              <th>${t('reports.boys')}</th>
              <th>${t('reports.girls')}</th>
              <th>${t('reports.rte')}</th>
              <th>${t('reports.feeDue')}</th>
              <th>${t('reports.collected')}</th>
            </tr>
          </thead>
          <tbody>
            ${r.byClass.map(
              (x) =>
                html`<tr>
                  <td><b>${x.c}</b></td>
                  <td>${x.n}</td>
                  <td>${x.boys}</td>
                  <td>${x.girls}</td>
                  <td>${x.rte}</td>
                  <td>${formatRupees(x.due)}</td>
                  <td>
                    ${formatRupees(x.col)}${bar(x.due ? Math.round((x.col / x.due) * 100) : 100, 'var(--green)')}
                  </td>
                </tr>`,
            )}
          </tbody>
        </table>
      </div>
      <div class="pgrid2" style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card">
          <div class="ch"><h3>${t('reports.category')}</h3></div>
          <div class="cb">
            ${r.cats.map((x) => html`<div class="pl"><span class="k">${x.c}</span><span class="v">${x.n}</span></div>`)}
          </div>
        </div>
        <div class="card">
          <div class="ch"><h3>${t('reports.feeStatus')}</h3></div>
          <div class="cb">
            ${r.feeStatus.map((x) => html`<div class="pl"><span class="k">${t('reports.fee.' + x.k)}</span><span class="v">${x.n}</span></div>`)}
          </div>
        </div>
      </div>
    </div>`,
  );
  delegate(root, {
    export: async () => {
      try {
        const out = await commands.exportClassSummaryXlsx({});
        downloadCsv(out.filename, out.rows);
        toast(t('reports.downloaded'), { kind: 'ok' });
      } catch (e) {
        toast(toAppError(e).message, { kind: 'error' });
      }
    },
  });
}
