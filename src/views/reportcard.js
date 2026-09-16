// Report-card modal and print. Ported from the prototype.
import { html } from '../core/html.js';
import { t } from '../core/i18n.js';
import { formatDate } from '../core/format.js';
import { openModal, printHtml, toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';
import { schoolHeader } from '../components/documents.js';

function reportCardDoc(d) {
  return html`${schoolHeader(d.school)}
    <h3 style="text-align:center;margin:6px 0 10px">
      ${t('report.progress', { session: d.school.session })}
    </h3>
    <table style="margin-bottom:10px">
      <tr>
        <th>${t('report.name')}</th>
        <td>${d.student.name}</td>
        <th>${t('report.class')}</th>
        <td>${d.student.ck}</td>
        <th>${t('report.roll')}</th>
        <td>${d.student.roll}</td>
      </tr>
      <tr>
        <th>${t('report.father')}</th>
        <td>${d.student.father}</td>
        <th>${t('report.admNo')}</th>
        <td>${d.student.adm}</td>
        <th>${t('report.dob')}</th>
        <td>${formatDate(d.student.dob)}</td>
      </tr>
    </table>
    <table>
      <thead>
        <tr>
          <th>${t('report.subject')}</th>
          ${d.exams.map((ex) => html`<th style="text-align:center">${ex.name}<br /><span style="font-weight:400">${t('report.outOf', { max: ex.max })}</span></th>`)}
        </tr>
      </thead>
      <tbody>
        ${d.rows.map(
          (r) =>
            html`<tr>
              <td>${r.subject}</td>
              ${r.byExam.map((v) => html`<td style="text-align:center">${v}</td>`)}
            </tr>`,
        )}
        <tr>
          <th>${t('report.total')}</th>
          ${d.totals.map((x) => html`<th style="text-align:center">${x.entered ? `${x.got}/${x.max}` : '—'}</th>`)}
        </tr>
        <tr>
          <th>${t('report.percentage')}</th>
          ${d.totals.map((x) => html`<th style="text-align:center">${x.pct == null ? '—' : x.pct + '%'}</th>`)}
        </tr>
        <tr>
          <th>${t('report.grade')}</th>
          ${d.totals.map((x) => html`<th style="text-align:center">${x.entered ? x.grade : '—'}</th>`)}
        </tr>
      </tbody>
    </table>
    <p style="margin:10px 0">
      ${t('report.attendance')}:
      ${d.attendance.t ? t('students.daysPct', { p: d.attendance.p, t: d.attendance.t, pct: d.attendance.pct }) : t('report.notRecorded')}
    </p>
    <p style="margin:10px 0">${t('report.remarks')}: ____________________________________________</p>
    <table style="margin-top:34px;border:none">
      <tr>
        ${[t('report.classTeacher'), t('report.principal'), t('report.parent')].map((x) => html`<td style="border:none;text-align:center;padding-top:26px">____________________<br />${x}</td>`)}
      </tr>
    </table>`;
}

export async function openReportCard(studentId) {
  let d;
  try {
    d = await commands.getReportCard({ studentId });
  } catch (e) {
    return toast(toAppError(e).message, { kind: 'error' });
  }
  const { element, close } = openModal({
    wide: true,
    title: t('students.reportCard'),
    subtitle: `${d.student.name}, ${d.student.ck}`,
    body: html`<div class="rc p-doc">${reportCardDoc(d)}</div>`,
    footer: html`<button class="btn b-quiet" data-ui="close2">${t('common.close')}</button>
      <button class="btn b-pri" data-ui="print">🖨 ${t('common.print')}</button>`,
  });
  element.addEventListener('click', (e) => {
    const b = e.target.closest && e.target.closest('[data-ui]');
    if (!b) return;
    if (b.getAttribute('data-ui') === 'close2') close();
    else if (b.getAttribute('data-ui') === 'print') printHtml(reportCardDoc(d));
  });
}
