import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

const state = { sectionId: '', examId: '', sheet: null };

export async function view(root, params) {
  const sections = params.me.sections?.length
    ? params.me.sections
    : (params.me.schoolClasses || []).flatMap((c) => c.sections.map((s) => `${c.name}-${s}`));
  state.sectionId = state.sectionId || params.ck || sections[0] || '';
  if (state.sectionId)
    state.sheet = await commands.getMarksSheet({ examId: state.examId, sectionId: state.sectionId });
  if (state.sheet) state.examId = state.sheet.exam.id;
  const s = state.sheet;
  render(
    root,
    html`<div class="inner stack">
      <div>
        <h1 style="font-size:21px;font-weight:750">${t('nav.marks')}</h1>
        <p class="sm mut" style="margin:2px 0 0">
          ${s ? t('marks.subtitle', { max: s.exam.max }) : t('marks.noClasses')}
        </p>
      </div>
      <div class="row g10 wrap">
        <select class="inp" id="mkSection" style="width:150px">
          ${sections.map((ck) => html`<option value="${ck}" ${ck === state.sectionId ? 'selected' : ''}>${ck}</option>`)}
        </select>
        ${
          s
            ? html`<select class="inp" id="mkExam" style="width:220px">
                ${s.exams.map((ex) => html`<option value="${ex.id}" ${ex.id === state.examId ? 'selected' : ''}>${ex.name}</option>`)}
              </select>`
            : ''
        }
      </div>
      ${
        s
          ? html`<div class="card scroll-x">
                <table class="tbl marks">
                  <thead>
                    <tr>
                      <th>${t('students.roll')}</th>
                      <th>${t('students.name')}</th>
                      ${s.subjects.map((sub) => html`<th>${sub}</th>`)}
                      <th>${t('report.total')}</th>
                      <th>${t('report.grade')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    ${s.students.map(
                      (stu) =>
                        html`<tr>
                          <td>${stu.roll}</td>
                          <td><b>${stu.name}</b></td>
                          ${s.subjects.map((sub) => html`<td><input class="inp num-in" data-mark-input data-adm="${stu.adm}" data-sub="${sub}" value="${stu.marks[sub] ?? ''}" ${s.editable ? '' : 'disabled'} /></td>`)}
                          <td>${stu.total}</td>
                          <td><span class="grade">${stu.grade}</span></td>
                        </tr>`,
                    )}
                  </tbody>
                </table>
              </div>
              <div class="err" id="mkErr"></div>
              ${s.editable ? html`<button class="btn b-pri b-lg" data-action="save">${t('common.saveMarks')}</button>` : html`<div class="note n-orange">${t('marks.readOnly')}</div>`}`
          : html`<div class="card"><div class="empty">${t('marks.noClasses')}</div></div>`
      }
    </div>`,
  );
  const refresh = () => view(root, params).catch((e) => toast(toAppError(e).message, { kind: 'error' }));
  delegate(root, {
    save: async () => {
      const entries = [...root.querySelectorAll('[data-mark-input]')].map((el) => ({
        studentId: el.dataset.adm,
        subjectId: el.dataset.sub,
        value: el.value,
      }));
      try {
        state.sheet = await commands.saveMarks({ examId: state.examId, sectionId: state.sectionId, entries });
        toast(t('marks.saved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        root.querySelector('#mkErr').textContent = toAppError(e).message;
      }
    },
  });
  root.querySelector('#mkSection')?.addEventListener('change', (e) => {
    state.sectionId = e.target.value;
    state.examId = '';
    refresh();
  });
  root.querySelector('#mkExam')?.addEventListener('change', (e) => {
    state.examId = e.target.value;
    refresh();
  });
}
