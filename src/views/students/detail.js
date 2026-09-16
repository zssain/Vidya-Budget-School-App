// Student detail page. Ported from the prototype.
import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import { formatDate, formatRupees } from '../../core/format.js';
import { go } from '../../core/router.js';
import * as commands from '../../api/commands.js';
import { feePill } from '../../components/widgets.js';
import { openStudentForm } from './form.js';
import { openReportCard } from '../reportcard.js';
import { openCollect } from '../fees/collect.js';

function initials(name) {
  return String(name || '?')
    .trim()
    .split(/\s+/)
    .map((w) => w[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();
}
const row = (k, v) => html`<div class="pl"><span class="k">${k}</span><span class="v">${v}</span></div>`;

export async function view(root, params) {
  const me = params.me;
  const s = await commands.getStudent({ studentId: params.adm });
  const perms = me.permissions;
  const canEdit = perms.includes('students.edit');
  const canReport =
    me.role !== 'accountant' && (me.role === 'principal' || (me.sections || []).includes(s.ck));
  const canCollect = perms.includes('fees.collect') && !s.rte && s.balance > 0 && s.status === 'active';
  const showFees = perms.includes('fees.view');
  const showMarks = me.role !== 'accountant';

  render(
    root,
    html`<div class="inner stack">
      <button class="btn b-quiet b-sm" style="align-self:flex-start" data-action="back">
        ← ${t('nav.students')}
      </button>
      <div class="card">
        <div class="cb row g16 wrap">
          <div
            style="width:60px;height:60px;border-radius:16px;background:var(--purple-l);color:var(--purple);display:flex;align-items:center;justify-content:center;font-size:24px;font-weight:750"
          >
            ${initials(s.name)}
          </div>
          <div class="grow" style="min-width:200px">
            <div class="row g8 wrap">
              <h1 style="font-size:21px;font-weight:750">${s.name}</h1>
              ${s.rte ? html`<span class="pill" style="background:var(--purple-l);color:var(--purple)">RTE</span>` : ''}
              ${s.transport ? html`<span class="pill p-grey">${t('students.bus')}</span>` : ''}
              ${s.status === 'left' ? html`<span class="pill p-red">${t('students.leftSchool')}</span>` : ''}
            </div>
            <div class="sm mut" style="margin-top:3px">
              ${t('students.classRoll', { ck: s.ck, roll: s.roll })} | <span class="num">${s.adm}</span>
            </div>
          </div>
          <div class="row g8 wrap">
            ${canEdit ? html`<button class="btn b-out" data-action="edit">✎ ${t('common.edit')}</button>` : ''}
            ${canReport ? html`<button class="btn b-out" data-action="report">📄 ${t('students.reportCard')}</button>` : ''}
            ${canCollect ? html`<button class="btn b-pri" data-action="collect">₹ ${t('common.collectFee')}</button>` : ''}
          </div>
        </div>
      </div>
      <div
        style="display:grid;grid-template-columns:repeat(auto-fit,minmax(290px,1fr));gap:14px;align-items:start"
      >
        <div class="card">
          <div class="ch"><h3>${t('students.details')}</h3></div>
          <div class="cb" style="padding-top:4px">
            ${row(t('students.father'), s.father)}${row(t('students.mother'), s.mother || '—')}${row(t('students.mobile'), html`<span class="num">${s.mobile}</span>`)}
            ${row(t('students.dob'), formatDate(s.dob))}${row(t('students.gender'), s.gender)}${row(t('students.category'), s.cat)}${row(t('students.locality'), s.village || '—')}
            ${row(t('students.admittedOn'), formatDate(s.admittedOn))}${s.status === 'left' ? row(t('students.leftOn'), formatDate(s.leftOn) + (s.leftReason ? ' (' + s.leftReason + ')' : '')) : ''}
          </div>
        </div>
        ${
          showFees
            ? html`<div class="card">
                <div class="ch">
                  <h3>${t('students.feesSession')}</h3>
                  ${feePill(s.feeState, s.balance)}
                </div>
                <div class="cb" style="padding-top:4px">
                  ${
                    s.rte
                      ? html`<div class="note n-blue">${t('students.rteNote')}</div>`
                      : html`${row(t('students.termFeeX', { n: s.terms }), formatRupees(s.termFee * s.terms))}
                        ${s.transport ? row(t('students.busX', { n: s.terms }), formatRupees(s.transportFee * s.terms)) : ''}
                        ${s.concession ? row(t('students.concession'), '− ' + formatRupees(s.concession)) : ''}
                        ${row(html`<b>${t('students.totalDue')}</b>`, html`<b class="num">${formatRupees(s.feeDue)}</b>`)}
                        ${row(t('fees.paid'), html`<span class="num" style="color:var(--green)">${formatRupees(s.paid)}</span>`)}
                        ${row(html`<b>${t('fees.balance')}</b>`, html`<b class="num" style="color:${s.balance ? 'var(--red)' : 'var(--green)'}">${formatRupees(s.balance)}</b>`)}`
                  }
                </div>
              </div>`
            : ''
        }
        ${
          showMarks
            ? html`<div class="card">
                <div class="ch"><h3>${t('students.attendanceMarks')}</h3></div>
                <div class="cb" style="padding-top:4px">
                  ${row(t('students.thisMonth'), s.attendanceMonth.t ? t('students.daysPct', { p: s.attendanceMonth.p, t: s.attendanceMonth.t, pct: s.attendanceMonth.pct }) : t('students.notMarkedYet'))}
                  ${row(t('students.thisSession'), s.attendanceSession.t ? t('students.daysPct', { p: s.attendanceSession.p, t: s.attendanceSession.t, pct: s.attendanceSession.pct }) : t('students.notMarkedYet'))}
                  ${s.exams.map((ex) => row(ex.name, ex.entered ? html`${ex.got}/${ex.max} <span class="grade g-${ex.grade}" style="margin-left:6px">${ex.grade}</span>` : html`<span class="mut">${t('students.noMarksYet')}</span>`))}
                </div>
              </div>`
            : ''
        }
      </div>
    </div>`,
  );

  delegate(root, {
    back: () => go('students'),
    edit: () => openStudentForm(s.adm),
    report: () => openReportCard(s.adm),
    collect: () => openCollect(s.adm),
  });
}
