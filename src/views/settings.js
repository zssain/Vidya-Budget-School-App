import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { formatRupees } from '../core/format.js';
import { toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

export async function view(root, params) {
  const s = await commands.getSettings();
  const refresh = () => view(root, params);
  render(
    root,
    html`<div class="inner stack">
      <h1 style="font-size:21px;font-weight:750">${t('nav.settings')}</h1>
      <div class="card">
        <div class="ch"><h3>${t('settings.school')}</h3></div>
        <div class="cb">
          <div class="fgrid">
            <div style="grid-column:1/-1">
              <label class="lbl">${t('settings.schoolName')}</label
              ><input class="inp" id="sName" value="${s.school.name}" />
            </div>
            <div style="grid-column:1/-1">
              <label class="lbl">${t('settings.address')}</label
              ><input class="inp" id="sAddr" value="${s.school.addr}" />
            </div>
            <div>
              <label class="lbl">${t('settings.udise')}</label
              ><input class="inp" id="sUdise" value="${s.school.udise}" />
            </div>
            <div>
              <label class="lbl">${t('settings.board')}</label
              ><input class="inp" id="sBoard" value="${s.school.board}" />
            </div>
            <div>
              <label class="lbl">${t('settings.session')}</label
              ><input class="inp" id="sSession" value="${s.school.session}" />
            </div>
            <div>
              <label class="lbl">${t('settings.phone')}</label
              ><input class="inp" id="sPhone" value="${s.school.phone}" />
            </div>
          </div>
          <div class="err" id="schoolErr"></div>
          <button class="btn b-pri" data-action="save-school">${t('settings.saveSchool')}</button>
        </div>
      </div>
      <div class="card">
        <div class="ch">
          <div>
            <h3>${t('settings.classesFees')}</h3>
            <p>${t('settings.perTerm')}</p>
          </div>
        </div>
        <div class="cb scroll-x">
          <table class="tbl">
            <thead>
              <tr>
                <th>${t('students.class')}</th>
                <th>${t('settings.sections')}</th>
                <th>${t('settings.tuition')}</th>
                <th>${t('settings.exam')}</th>
                <th>${t('settings.other')}</th>
                <th>${t('settings.termTotal')}</th>
                <th>${t('settings.subjects')}</th>
              </tr>
            </thead>
            <tbody>
              ${s.classes.map(
                (c, i) =>
                  html`<tr data-class-row data-index="${i}">
                    <td><input class="inp" data-field="name" value="${c.name}" /></td>
                    <td><input class="inp num-in" data-field="sections" value="${c.sections}" /></td>
                    <td><input class="inp num-in" data-field="tuition" value="${c.tuition}" /></td>
                    <td><input class="inp num-in" data-field="exam" value="${c.exam}" /></td>
                    <td><input class="inp num-in" data-field="other" value="${c.other}" /></td>
                    <td>${formatRupees(c.termFee)}</td>
                    <td><input class="inp" data-field="subjects" value="${c.subjects.join(', ')}" /></td>
                  </tr>`,
              )}
            </tbody>
          </table>
          <div class="fgrid" style="margin-top:12px">
            <div>
              <label class="lbl">${t('settings.busFee')}</label
              ><input class="inp" id="sBus" value="${s.transportFee}" />
            </div>
            <div>
              <label class="lbl">${t('settings.terms')}</label
              ><input class="inp" id="sTerms" value="${s.terms}" />
            </div>
          </div>
          <div class="err" id="classErr"></div>
          <button class="btn b-pri" data-action="save-classes">${t('settings.saveClasses')}</button>
        </div>
      </div>
      <div class="card">
        <div class="ch"><h3>${t('settings.exams')}</h3></div>
        <div class="cb">
          ${s.exams.map((ex) => html`<div class="row g10" data-exam-row data-id="${ex.id}" style="margin-bottom:8px"><input class="inp" data-field="name" value="${ex.name}" /><input class="inp num-in" data-field="max" value="${ex.max}" /></div>`)}
          <div class="err" id="examErr"></div>
          <button class="btn b-pri" data-action="save-exams">${t('settings.saveExams')}</button>
        </div>
      </div>
      <div class="card">
        <div class="ch"><h3>${t('settings.deviceLicense')}</h3></div>
        <div class="cb">
          <div class="pl">
            <span class="k">${t('settings.schoolCode')}</span
            ><span class="v mono">@${s.license.schoolCode}</span>
          </div>
          <div class="pl">
            <span class="k">${t('settings.deviceId')}</span><span class="v mono">${s.deviceId}</span>
          </div>
          <div class="pl">
            <span class="k">${t('settings.activation')}</span
            ><span class="v">${s.license.activationCode}</span>
          </div>
          <div class="pl">
            <span class="k">${t('settings.loginLimit')}</span><span class="v">${s.license.maxUsers}</span>
          </div>
          <label class="lbl" style="margin-top:12px">${t('settings.receiptPrefix')}</label>
          <div class="row g10">
            <input class="inp mono" id="sDev" value="${s.deviceCode}" style="width:120px" /><button
              class="btn b-out"
              data-action="save-device"
            >
              ${t('common.save')}
            </button>
          </div>
          <div class="err" id="deviceErr"></div>
        </div>
      </div>
    </div>`,
  );
  const fail = (id, e) => {
    root.querySelector(id).textContent = toAppError(e).message;
  };
  delegate(root, {
    'save-school': async () => {
      try {
        await commands.saveSchool({
          name: root.querySelector('#sName').value,
          addr: root.querySelector('#sAddr').value,
          udise: root.querySelector('#sUdise').value,
          board: root.querySelector('#sBoard').value,
          session: root.querySelector('#sSession').value,
          phone: root.querySelector('#sPhone').value,
        });
        toast(t('settings.schoolSaved'), { kind: 'ok' });
        params.refreshChrome?.();
        refresh();
      } catch (e) {
        fail('#schoolErr', e);
      }
    },
    'save-classes': async () => {
      const classes = [...root.querySelectorAll('[data-class-row]')].map((row) =>
        Object.fromEntries(
          [...row.querySelectorAll('[data-field]')].map((el) => [el.dataset.field, el.value]),
        ),
      );
      try {
        await commands.saveClasses({
          classes,
          transportFee: root.querySelector('#sBus').value,
          terms: root.querySelector('#sTerms').value,
        });
        toast(t('settings.classesSaved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        fail('#classErr', e);
      }
    },
    'save-exams': async () => {
      const exams = [...root.querySelectorAll('[data-exam-row]')].map((row) => ({
        id: row.dataset.id,
        ...Object.fromEntries(
          [...row.querySelectorAll('[data-field]')].map((el) => [el.dataset.field, el.value]),
        ),
      }));
      try {
        await commands.saveExams({ exams });
        toast(t('settings.examsSaved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        fail('#examErr', e);
      }
    },
    'save-device': async () => {
      try {
        await commands.saveDeviceCode({ code: root.querySelector('#sDev').value });
        toast(t('settings.deviceSaved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        fail('#deviceErr', e);
      }
    },
  });
}
