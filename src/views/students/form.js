// Add / edit student modal. Ported from the prototype.
import { html } from '../../core/html.js';
import { t } from '../../core/i18n.js';
import { openModal, toast, confirmDialog } from '../../core/ui.js';
import { go, rerender } from '../../core/router.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';

const opt = (arr, cur) => arr.map((x) => html`<option ${x === cur ? 'selected' : ''}>${x}</option>`);

export async function openStudentForm(adm) {
  const me = await commands.currentUser();
  const classes = me.schoolClasses || [];
  const s = adm ? await commands.getStudent({ studentId: adm }) : null;
  const e = s || { gender: 'Male', cat: 'General', cls: (classes[0] || {}).name, sec: 'A', status: 'active' };
  const isPrincipal = me.role === 'principal';
  const secsOf = (cls) => (classes.find((c) => c.name === cls) || classes[0] || { sections: ['A'] }).sections;

  const { element, close } = openModal({
    wide: true,
    title: s ? t('students.editTitle') : t('students.newAdmission'),
    subtitle: s ? s.adm : t('students.admAuto'),
    body: html`<div class="fgrid">
        <div>
          <label class="lbl">${t('students.nameLabel')}</label
          ><input class="inp" id="fName" value="${e.name || ''}" autofocus />
        </div>
        <div>
          <label class="lbl">${t('students.genderLabel')}</label
          ><select class="inp" id="fGender">
            ${opt(['Male', 'Female', 'Other'], e.gender)}
          </select>
        </div>
        <div>
          <label class="lbl">${t('students.classLabel')}</label
          ><select class="inp" id="fClass">
            ${opt(
              classes.map((c) => c.name),
              e.cls,
            )}
          </select>
        </div>
        <div>
          <label class="lbl">${t('students.sectionLabel')}</label
          ><select class="inp" id="fSec">
            ${opt(secsOf(e.cls), e.sec)}
          </select>
        </div>
        <div>
          <label class="lbl">${t('students.fatherLabel')}</label
          ><input class="inp" id="fFather" value="${e.father || ''}" />
        </div>
        <div>
          <label class="lbl">${t('students.motherLabel')}</label
          ><input class="inp" id="fMother" value="${e.mother || ''}" />
        </div>
        <div>
          <label class="lbl">${t('students.mobileLabel')}</label
          ><input class="inp num" id="fMob" inputmode="numeric" maxlength="10" value="${e.mobile || ''}" />
        </div>
        <div>
          <label class="lbl">${t('students.dobLabel')}</label
          ><input class="inp" type="date" id="fDob" value="${e.dob || ''}" />
        </div>
        <div>
          <label class="lbl">${t('students.categoryLabel')}</label
          ><select class="inp" id="fCat">
            ${opt(['General', 'OBC', 'SC', 'ST'], e.cat)}
          </select>
        </div>
        <div>
          <label class="lbl">${t('students.localityLabel')}</label
          ><input class="inp" id="fVil" value="${e.village || ''}" />
        </div>
        ${isPrincipal ? html`<div><label class="lbl">${t('students.concessionLabel')}</label><input class="inp num" id="fCon" inputmode="numeric" value="${e.concession || 0}" /></div>` : ''}
        ${
          s
            ? html`<div>
                <label class="lbl">${t('students.statusLabel')}</label
                ><select class="inp" id="fStatus">
                  <option value="active" ${e.status === 'active' ? 'selected' : ''}>
                    ${t('students.studying')}
                  </option>
                  <option value="left" ${e.status === 'left' ? 'selected' : ''}>${t('students.left')}</option>
                </select>
              </div>`
            : ''
        }
      </div>
      ${
        s
          ? html`<div id="leftBox" class="fgrid ${e.status === 'left' ? '' : 'hide'}" style="margin-top:12px">
              <div>
                <label class="lbl">${t('students.leftOnLabel')}</label
                ><input class="inp" type="date" id="fLeftOn" value="${e.leftOn || ''}" />
              </div>
              <div>
                <label class="lbl">${t('students.reasonLabel')}</label
                ><input class="inp" id="fLeftWhy" value="${e.leftReason || ''}" />
              </div>
            </div>`
          : ''
      }
      <div class="row g16 wrap" style="margin-top:14px">
        ${[
          ['fRte', t('students.rteSeat'), e.rte],
          ['fBus', t('students.usesBus'), e.transport],
          ['fAad', t('students.aadhaarCollected'), e.aadhaar],
          ['fApaar', t('students.apaarCreated'), e.apaar],
        ].map(
          ([id, label, checked]) =>
            html`<label class="row g8" style="cursor:pointer"
              ><input
                type="checkbox"
                id="${id}"
                ${checked ? 'checked' : ''}
                style="width:20px;height:20px"
              /><span class="sm">${label}</span></label
            >`,
        )}
      </div>
      ${!isPrincipal ? html`<div class="note n-blue" style="margin-top:12px">${t('students.concessionNote')}</div>` : ''}
      <div class="err" id="fErr"></div>`,
    footer: html`<button class="btn b-quiet" data-ui="cancel">${t('common.cancel')}</button>
      <button class="btn b-pri" data-ui="save">${t('common.save')}</button>`,
  });

  const q = (sel) => element.querySelector(sel);
  const err = (m) => (q('#fErr').textContent = m || '');
  q('#fClass').addEventListener('change', () => {
    const sel = q('#fSec');
    sel.replaceChildren();
    secsOf(q('#fClass').value).forEach((x) => {
      const o = document.createElement('option');
      o.textContent = x;
      sel.appendChild(o);
    });
  });
  if (q('#fStatus'))
    q('#fStatus').addEventListener('change', () =>
      q('#leftBox').classList.toggle('hide', q('#fStatus').value !== 'left'),
    );

  element.addEventListener('click', async (ev) => {
    const b = ev.target.closest && ev.target.closest('[data-ui]');
    if (!b) return;
    if (b.getAttribute('data-ui') === 'cancel') return close();
    if (b.getAttribute('data-ui') !== 'save') return;
    const input = {
      name: q('#fName').value,
      gender: q('#fGender').value,
      cls: q('#fClass').value,
      sec: q('#fSec').value,
      father: q('#fFather').value,
      mother: q('#fMother').value,
      mobile: q('#fMob').value,
      dob: q('#fDob').value,
      cat: q('#fCat').value,
      village: q('#fVil').value,
      rte: q('#fRte').checked,
      transport: q('#fBus').checked,
      aadhaar: q('#fAad').checked,
      apaar: q('#fApaar').checked,
    };
    if (q('#fCon')) input.concession = q('#fCon').value;
    if (s) {
      input.studentId = s.adm;
      if (q('#fStatus')) {
        input.status = q('#fStatus').value;
        input.leftOn = q('#fLeftOn') ? q('#fLeftOn').value : '';
        input.leftReason = q('#fLeftWhy') ? q('#fLeftWhy').value : '';
      }
    }
    b.disabled = true;
    err('');
    try {
      if (s) {
        await commands.updateStudent(input);
        close();
        go('student', { adm: s.adm });
        toast(t('students.saved'), { kind: 'ok' });
      } else {
        await commands.addStudent(input);
        close();
        toast(t('students.added'), { kind: 'ok' });
        rerender();
      }
    } catch (ex) {
      b.disabled = false;
      const appErr = toAppError(ex);
      if (appErr.kind === 'conflict') {
        const ok = await confirmDialog({
          title: t('students.possibleDuplicate'),
          message: appErr.message,
          confirmLabel: t('students.addAnyway'),
        });
        if (!ok) return;
        try {
          const created = await commands.addStudent({ ...input, confirmDuplicate: true });
          close();
          go('student', { adm: created.adm });
          toast(t('students.added'), { kind: 'ok' });
        } catch (ex2) {
          err(toAppError(ex2).message);
        }
        return;
      }
      err(appErr.message);
    }
  });
}
