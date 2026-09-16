import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { printHtml, toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';
import { schoolHeader } from '../components/documents.js';

const state = { sectionId: '', date: '', sheet: null };

function today() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function sections(params) {
  if (params.me.sections?.length) return params.me.sections;
  return (params.me.schoolClasses || []).flatMap((c) => c.sections.map((s) => `${c.name}-${s}`));
}

async function load(params) {
  const list = sections(params);
  state.sectionId = state.sectionId || params.ck || list[0] || '';
  state.date = state.date || today();
  state.sheet = state.sectionId
    ? await commands.getAttendance({ sectionId: state.sectionId, date: state.date })
    : null;
}

function registerDocument(reg) {
  return html`${schoolHeader(reg.school)}
    <h3>
      ${t('attendance.registerTitle', { section: reg.sectionId, month: reg.monthName, year: reg.year })}
    </h3>
    <table>
      <thead>
        <tr>
          <th>${t('students.roll')}</th>
          <th>${t('students.name')}</th>
          ${Array.from({ length: reg.days }, (_, i) => html`<th>${i + 1}</th>`)}
          <th>${t('attendance.present')}</th>
        </tr>
      </thead>
      <tbody>
        ${reg.rows.map(
          (r) =>
            html`<tr>
              <td>${r.roll}</td>
              <td>${r.name}</td>
              ${r.cells.map((v) => html`<td>${v}</td>`)}
              <td>${r.present}</td>
            </tr>`,
        )}
      </tbody>
    </table>`;
}

export async function view(root, params) {
  await load(params);
  const sheet = state.sheet;
  render(
    root,
    html`<div class="inner stack">
      <div class="spread wrap g12">
        <div>
          <h1 style="font-size:21px;font-weight:750">${t('nav.attendance')}</h1>
          <p class="sm mut" style="margin:2px 0 0">${t('attendance.subtitle')}</p>
        </div>
        <button class="btn b-out" data-action="register">🖨 ${t('attendance.printRegister')}</button>
      </div>
      <div class="row g10 wrap">
        <select class="inp" id="attSection" style="width:160px" aria-label="${t('students.class')}">
          ${sections(params).map((ck) => html`<option value="${ck}" ${ck === state.sectionId ? 'selected' : ''}>${ck}</option>`)}
        </select>
        <input
          class="inp"
          id="attDate"
          type="date"
          value="${state.date}"
          max="${today()}"
          aria-label="${t('doc.date')}"
        />
      </div>
      ${
        !sheet
          ? html`<div class="card"><div class="empty">${t('attendance.noClasses')}</div></div>`
          : html`
              ${sheet.readOnlyReason ? html`<div class="note n-orange">${sheet.readOnlyReason}</div>` : ''}
              <div class="spread wrap g10">
                <div class="sm mut">
                  ${sheet.savedBy ? t('attendance.savedBy', { name: sheet.savedBy }) : t('attendance.notSaved')}
                </div>
                ${!sheet.readOnlyReason ? html`<button class="btn b-out b-sm" data-action="all-present">${t('common.markAll')}</button>` : ''}
              </div>
              <div class="attgrid">
                ${sheet.students.map(
                  (s) =>
                    html`<div class="attcard">
                      <div class="grow">
                        <b>${s.name}</b>
                        <div class="xs mut">${t('students.roll')} ${s.roll} · ${s.adm}</div>
                      </div>
                      <div class="seg">
                        ${['P', 'A', 'L'].map((mark) => html`<button class="${sheet.marks[s.adm] === mark ? 'on' : ''}" data-action="mark" data-adm="${s.adm}" data-mark="${mark}" ${sheet.readOnlyReason ? 'disabled' : ''}>${mark}</button>`)}
                      </div>
                    </div>`,
                )}
              </div>
              <div class="err" id="attErr"></div>
              ${!sheet.readOnlyReason ? html`<button class="btn b-pri b-lg" data-action="save">${t('common.saveAttendance')}</button>` : ''}
            `
      }
    </div>`,
  );

  const refresh = () => view(root, params).catch((e) => toast(toAppError(e).message, { kind: 'error' }));
  delegate(root, {
    mark: (e, el) => {
      state.sheet.marks[el.dataset.adm] = el.dataset.mark;
      refresh();
    },
    'all-present': () => {
      state.sheet.students.forEach((s) => {
        state.sheet.marks[s.adm] = 'P';
      });
      refresh();
    },
    save: async () => {
      try {
        state.sheet = await commands.saveAttendance({
          sectionId: state.sectionId,
          date: state.date,
          marks: state.sheet.marks,
        });
        toast(t('attendance.saved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        root.querySelector('#attErr').textContent = toAppError(e).message;
      }
    },
    register: async () => {
      if (!state.sectionId) return;
      try {
        const reg = await commands.attendanceRegister({
          sectionId: state.sectionId,
          month: state.date.slice(0, 7),
        });
        printHtml(registerDocument(reg));
      } catch (e) {
        toast(toAppError(e).message, { kind: 'error' });
      }
    },
  });
  root.querySelector('#attSection')?.addEventListener('change', (e) => {
    state.sectionId = e.target.value;
    refresh();
  });
  root.querySelector('#attDate')?.addEventListener('change', (e) => {
    state.date = e.target.value;
    refresh();
  });
}
