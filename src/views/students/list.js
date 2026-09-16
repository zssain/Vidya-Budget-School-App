// Students list. Ported from the prototype.
import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import { go } from '../../core/router.js';
import { toast } from '../../core/ui.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { feePill } from '../../components/widgets.js';
import { downloadCsv } from '../../core/download.js';
import { openStudentForm } from './form.js';

const filter = { q: '', sectionId: 'All', status: 'active' };

export async function view(root, params) {
  const me = params.me;
  const office = me.role !== 'teacher';
  const showFees = me.permissions.includes('fees.view');
  const canAdd = me.permissions.includes('students.add');
  const [universe, list] = await Promise.all([
    commands.listStudents({ status: filter.status }),
    commands.listStudents(filter),
  ]);
  const sections = office ? [...new Set(universe.map((s) => s.ck))] : me.sections;

  render(
    root,
    html`<div class="inner stack">
      <div class="spread wrap g12">
        <div>
          <h1 style="font-size:21px;font-weight:750">${t('nav.students')}</h1>
          <p class="sm mut" style="margin:2px 0 0">
            ${office ? t('students.activeCount', { n: universe.length }) : t('students.yourClasses')}
          </p>
        </div>
        <div class="row g8">
          ${office ? html`<button class="btn b-out" data-action="export">⬇ ${t('students.excel')}</button>` : ''}
          ${canAdd ? html`<button class="btn b-pri" data-action="add">+ ${t('common.addStudent')}</button>` : ''}
        </div>
      </div>
      <div class="card">
        <div class="cb" style="padding:13px">
          <input
            class="inp"
            id="stuQ"
            placeholder="${t('students.searchPlaceholder')}"
            value="${filter.q}"
            style="margin-bottom:11px"
          />
          <div class="chipbar">
            <button class="chip ${filter.sectionId === 'All' ? 'on' : ''}" data-action="sec" data-sec="All">
              ${t('students.all')}
            </button>
            ${sections.map((ck) => html`<button class="chip ${filter.sectionId === ck ? 'on' : ''}" data-action="sec" data-sec="${ck}">${ck}</button>`)}
            ${
              office
                ? html`<button class="chip ${filter.status === 'left' ? 'on' : ''}" data-action="toggle-left">
                    ${filter.status === 'left' ? t('students.showingLeft') : t('students.showLeft')}
                  </button>`
                : ''
            }
          </div>
        </div>
      </div>
      <div class="card scroll-x">
        ${
          list.length
            ? html`<table class="tbl">
                <thead>
                  <tr>
                    <th style="width:56px">${t('students.roll')}</th>
                    <th>${t('students.name')}</th>
                    <th style="width:84px">${t('students.class')}</th>
                    <th>${t('students.father')}</th>
                    <th style="width:120px">${t('students.mobile')}</th>
                    ${showFees ? html`<th style="width:130px">${t('nav.fees')}</th>` : ''}
                  </tr>
                </thead>
                <tbody>
                  ${list.slice(0, 150).map(
                    (s) =>
                      html`<tr class="tap" data-action="open" data-adm="${s.adm}">
                        <td><b class="num">${s.roll}</b></td>
                        <td>
                          <b>${s.name}</b>
                          <div class="xs mut num">
                            ${s.adm}${s.rte ? ' | RTE' : ''}${s.transport ? ' | ' + t('students.bus') : ''}
                          </div>
                        </td>
                        <td><span class="pill p-blue">${s.ck}</span></td>
                        <td class="sm">${s.father}</td>
                        <td class="sm num">${s.mobile}</td>
                        ${showFees ? html`<td>${feePill(s.feeState, s.balance)}</td>` : ''}
                      </tr>`,
                  )}
                </tbody>
              </table>`
            : html`<div class="empty">
                <div class="e">🔍</div>
                <b>${t('students.noneFound')}</b>
                <div class="sm">${t('students.tryAnother')}</div>
              </div>`
        }
      </div>
    </div>`,
  );

  delegate(root, {
    add: () => openStudentForm(null),
    open: (e, el) => go('student', { adm: el.getAttribute('data-adm') }),
    sec: (e, el) => {
      filter.sectionId = el.getAttribute('data-sec');
      view(root, params);
    },
    'toggle-left': () => {
      filter.status = filter.status === 'left' ? 'active' : 'left';
      view(root, params);
    },
    export: async () => {
      try {
        const x = await commands.exportStudentsXlsx({});
        downloadCsv(x.filename, x.rows);
        toast(t('students.downloaded'), { kind: 'ok' });
      } catch (e) {
        toast(toAppError(e).message, { kind: 'error' });
      }
    },
  });
  const q = root.querySelector('#stuQ');
  if (q)
    q.addEventListener('input', () => {
      filter.q = q.value;
      clearTimeout(q._t);
      q._t = setTimeout(() => view(root, params), 200);
    });
}
