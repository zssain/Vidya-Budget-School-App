import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { renderCredentials } from './credentials.js';

const names = ['Nursery', 'LKG', 'UKG', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];
const steps = ['activate', 'school', 'principal', 'classes', 'staff', 'backup', 'review'];
let w;

function defaults() {
  const y = new Date().getFullYear();
  return {
    step: 0,
    deviceId: '',
    code: '',
    school: {
      name: '',
      addr: '',
      udise: '',
      board: 'State Board',
      session: `${y}-${String(y + 1).slice(-2)}`,
      phone: '',
    },
    principal: { name: '', mobile: '', pw: '', pw2: '' },
    classes: names.map((name) => ({
      name,
      on: !['IX', 'X'].includes(name),
      sections: name === 'Nursery' ? 1 : 2,
      tuition: 1200,
      exam: 200,
      other: 100,
    })),
    transportFee: 900,
    terms: 3,
    teachers: [{ name: '', mobile: '', classes: [] }],
    accountants: [{ name: '', mobile: '' }],
    backupPw: '',
    backupPw2: '',
    wroteDown: false,
  };
}

function activeSections() {
  return w.classes
    .filter((c) => c.on)
    .flatMap((c) =>
      Array.from({ length: +c.sections }, (_, i) => `${c.name}-${String.fromCharCode(65 + i)}`),
    );
}

function field(label, path, type = 'text') {
  const value = path.split('.').reduce((o, k) => o[k], w);
  return html`<div>
    <label class="lbl">${t(label)}</label
    ><input class="inp" type="${type}" data-path="${path}" value="${value}" />
  </div>`;
}

function body() {
  if (w.step === 0)
    return html`<p>${t('wizard.activateHelp')}</p>
      <label class="lbl">${t('wizard.deviceId')}</label>
      <div class="devid">${w.deviceId}</div>
      ${field('wizard.activationCode', 'code')}
      <div class="note n-blue">${t('wizard.demoNote')}</div>`;
  if (w.step === 1)
    return html`<div class="fgrid">
      ${field('settings.schoolName', 'school.name')}${field('settings.address', 'school.addr')}${field('settings.udise', 'school.udise')}${field('settings.board', 'school.board')}${field('settings.session', 'school.session')}${field('settings.phone', 'school.phone')}
    </div>`;
  if (w.step === 2)
    return html`<p>${t('wizard.principalHelp')}</p>
      <div class="fgrid">
        ${field('wizard.fullName', 'principal.name')}${field('settings.phone', 'principal.mobile')}${field('wizard.password', 'principal.pw', 'password')}${field('password.again', 'principal.pw2', 'password')}
      </div>`;
  if (w.step === 3)
    return html`<p>${t('wizard.classesHelp')}</p>
      <div class="scroll-x">
        <table class="tbl">
          <thead>
            <tr>
              <th></th>
              <th>${t('students.class')}</th>
              <th>${t('settings.sections')}</th>
              <th>${t('settings.tuition')}</th>
              <th>${t('settings.exam')}</th>
              <th>${t('settings.other')}</th>
            </tr>
          </thead>
          <tbody>
            ${w.classes.map(
              (c, i) =>
                html`<tr>
                  <td><input type="checkbox" data-class-on="${i}" ${c.on ? 'checked' : ''} /></td>
                  <td><b>${c.name}</b></td>
                  ${['sections', 'tuition', 'exam', 'other'].map((x) => html`<td><input class="inp num-in" data-class-index="${i}" data-class-field="${x}" value="${c[x]}" /></td>`)}
                </tr>`,
            )}
          </tbody>
        </table>
      </div>
      <div class="fgrid">
        ${field('settings.busFee', 'transportFee')}
        <div>
          <label class="lbl">${t('settings.terms')}</label
          ><input class="inp" data-path="terms" value="${w.terms}" />
        </div>
      </div>`;
  if (w.step === 4)
    return html`<p>${t('wizard.staffHelp')}</p>
      <div class="spread">
        <h3>${t('wizard.teachers')}</h3>
        <label class="row g8 sm"
          >${t('wizard.howMany')}<input
            class="inp num"
            style="width:80px"
            type="number"
            min="0"
            max="80"
            value="${w.teachers.length}"
            data-action="resize"
            data-staff-kind="teachers"
        /></label>
      </div>
      ${w.teachers.map(
        (x, i) =>
          html`<div class="card">
            <div class="cb">
              <div class="fgrid">
                <div>
                  <label class="lbl">${t('users.name')}</label
                  ><input
                    class="inp"
                    data-staff="teachers"
                    data-index="${i}"
                    data-field="name"
                    value="${x.name}"
                  />
                </div>
                <div>
                  <label class="lbl">${t('users.mobile')}</label
                  ><input
                    class="inp"
                    data-staff="teachers"
                    data-index="${i}"
                    data-field="mobile"
                    value="${x.mobile}"
                  />
                </div>
              </div>
              <label class="lbl">${t('users.classes')}</label>
              <div class="chipbar">
                ${activeSections().map((ck) => html`<button type="button" class="chip ${x.classes.includes(ck) ? 'on' : ''}" data-action="class" data-index="${i}" data-ck="${ck}">${ck}</button>`)}
              </div>
            </div>
          </div>`,
      )}
      <div class="spread">
        <h3>${t('wizard.accountants')}</h3>
        <label class="row g8 sm"
          >${t('wizard.howMany')}<input
            class="inp num"
            style="width:80px"
            type="number"
            min="0"
            max="5"
            value="${w.accountants.length}"
            data-action="resize"
            data-staff-kind="accountants"
        /></label>
      </div>
      ${w.accountants.map(
        (x, i) =>
          html`<div class="fgrid">
            <div>
              <label class="lbl">${t('users.name')}</label
              ><input
                class="inp"
                data-staff="accountants"
                data-index="${i}"
                data-field="name"
                value="${x.name}"
              />
            </div>
            <div>
              <label class="lbl">${t('users.mobile')}</label
              ><input
                class="inp"
                data-staff="accountants"
                data-index="${i}"
                data-field="mobile"
                value="${x.mobile}"
              />
            </div>
          </div>`,
      )}`;
  if (w.step === 5)
    return html`<p>${t('wizard.backupHelp')}</p>
      <div class="fgrid">
        ${field('wizard.backupPassword', 'backupPw', 'password')}${field('password.again', 'backupPw2', 'password')}
      </div>
      <div class="note n-orange">${t('wizard.backupNote')}</div>
      <label class="row g8"
        ><input
          type="checkbox"
          id="wroteDown"
          ${w.wroteDown ? 'checked' : ''}
        />${t('wizard.wroteDown')}</label
      >`;
  return html`<div class="pl"><span class="k">${t('wizard.school')}</span><b>${w.school.name}</b></div>
    <div class="pl">
      <span class="k">${t('wizard.classes')}</span><span>${w.classes.filter((c) => c.on).length}</span>
    </div>
    <div class="pl"><span class="k">${t('wizard.teachers')}</span><span>${w.teachers.length}</span></div>
    <div class="pl">
      <span class="k">${t('wizard.accountants')}</span><span>${w.accountants.length}</span>
    </div>
    <div class="note n-blue">${t('wizard.reviewNote')}</div>`;
}

function sync(root) {
  root.querySelectorAll('[data-path]').forEach((el) =>
    el.addEventListener('input', () => {
      const p = el.dataset.path.split('.');
      let o = w;
      while (p.length > 1) o = o[p.shift()];
      o[p[0]] = el.value;
    }),
  );
  root.querySelectorAll('[data-class-on]').forEach((el) =>
    el.addEventListener('change', () => {
      w.classes[+el.dataset.classOn].on = el.checked;
    }),
  );
  root.querySelectorAll('[data-class-index]').forEach((el) =>
    el.addEventListener('input', () => {
      w.classes[+el.dataset.classIndex][el.dataset.classField] = el.value;
    }),
  );
  root.querySelectorAll('[data-staff]').forEach((el) =>
    el.addEventListener('input', () => {
      w[el.dataset.staff][+el.dataset.index][el.dataset.field] = el.value;
    }),
  );
  root.querySelector('#wroteDown')?.addEventListener('change', (e) => {
    w.wroteDown = e.target.checked;
  });
}

async function draw(root, ctx, error = '') {
  render(
    root,
    html`<div class="wz">
      <div class="wz-h">
        <div class="xs mut b">${t('wizard.step', { n: w.step + 1, total: steps.length })}</div>
        <h1>${t('wizard.' + steps[w.step])}</h1>
        <div class="wz-bar">
          ${steps.map((_, i) => html`<i class="${i < w.step ? 'done' : i === w.step ? 'on' : ''}"></i>`)}
        </div>
      </div>
      <div class="wz-b">
        ${body()}
        <div class="err" id="wzErr">${error}</div>
      </div>
      <div class="wz-f">
        <button class="btn b-quiet" data-action="back">
          ${w.step ? t('common.back') : t('common.cancel')}</button
        ><button class="btn b-pri" data-action="next">
          ${w.step === steps.length - 1 ? t('wizard.create') : t('wizard.continue')}
        </button>
      </div>
    </div>`,
  );
  sync(root);
  delegate(root, {
    back: () => {
      if (!w.step) ctx.showSetup();
      else {
        w.step--;
        draw(root, ctx);
      }
    },
    class: (e, el) => {
      const list = w.teachers[+el.dataset.index].classes;
      const at = list.indexOf(el.dataset.ck);
      if (at >= 0) list.splice(at, 1);
      else list.push(el.dataset.ck);
      draw(root, ctx);
    },
    resize: (e, el) => {
      const kind = el.dataset.staffKind;
      const max = kind === 'teachers' ? 80 : 5;
      const count = Math.max(0, Math.min(max, Number.parseInt(el.value, 10) || 0));
      while (w[kind].length < count)
        w[kind].push(kind === 'teachers' ? { name: '', mobile: '', classes: [] } : { name: '', mobile: '' });
      w[kind].length = count;
      draw(root, ctx);
    },
    next: async () => {
      if (w.step < steps.length - 1) {
        w.step++;
        draw(root, ctx);
        return;
      }
      try {
        const result = await commands.wizardCreateSchool(w);
        renderCredentials(root, ctx, result);
      } catch (e) {
        draw(root, ctx, toAppError(e).message);
      }
    },
  });
}

export async function renderWizard(root, ctx) {
  w = defaults();
  try {
    w.deviceId = (await commands.getDeviceId()).deviceId;
  } catch (e) {
    w.deviceId = toAppError(e).message;
  }
  draw(root, ctx);
}
