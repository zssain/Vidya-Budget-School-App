import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { formatDateTime } from '../core/format.js';
import { confirmDialog, openModal, toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

function credentialSlip(slip) {
  openModal({
    title: t('users.credentials'),
    locked: true,
    body: html`<div class="note n-orange">${t('users.once')}</div>
      <div class="slip">
        <h3>${slip.name}</h3>
        <div class="pl">
          <span class="k">${t('account.username')}</span><b class="mono">${slip.username}</b>
        </div>
        <div class="pl">
          <span class="k">${t('users.tempPassword')}</span><b class="mono">${slip.tempPassword}</b>
        </div>
      </div>`,
    footer: html`<button class="btn b-pri" data-action="done">${t('common.done')}</button>`,
    onAction: (name, { close }) => {
      if (name === 'done') close();
    },
  });
}

function userForm(user, refresh) {
  const editing = !!user;
  openModal({
    title: editing ? t('users.edit') : t('users.add'),
    body: html` <label class="lbl">${t('users.name')}</label
      ><input class="inp" id="usrName" value="${user?.name || ''}" />
      <label class="lbl" style="margin-top:10px">${t('users.mobile')}</label
      ><input class="inp" id="usrMobile" value="${user?.mobile || ''}" />
      ${
        editing
          ? ''
          : html`<label class="lbl" style="margin-top:10px">${t('users.role')}</label
              ><select class="inp" id="usrRole">
                <option value="teacher">${t('roles.teacher')}</option>
                <option value="accountant">${t('roles.accountant')}</option>
              </select>`
      }
      <label class="lbl" style="margin-top:10px">${t('users.classes')}</label
      ><input
        class="inp"
        id="usrClasses"
        value="${(user?.classes || []).join(', ')}"
        placeholder="1-A, 2-A"
      />
      <div class="err" id="usrErr"></div>`,
    footer: html`<button class="btn b-quiet" data-action="cancel">${t('common.cancel')}</button
      ><button class="btn b-pri" data-action="save">${t('common.save')}</button>`,
    onAction: async (name, { element, close }) => {
      if (name === 'cancel') return close();
      if (name !== 'save') return;
      const modal = element.closest('.modal');
      const input = {
        name: modal.querySelector('#usrName').value,
        mobile: modal.querySelector('#usrMobile').value,
        sectionIds: modal
          .querySelector('#usrClasses')
          .value.split(',')
          .map((x) => x.trim())
          .filter(Boolean),
      };
      try {
        if (editing) await commands.updateUser({ userId: user.id, ...input });
        else {
          const slip = await commands.createUser({ ...input, role: modal.querySelector('#usrRole').value });
          close();
          credentialSlip(slip);
          refresh();
          return;
        }
        close();
        toast(t('users.saved'), { kind: 'ok' });
        refresh();
      } catch (e) {
        modal.querySelector('#usrErr').textContent = toAppError(e).message;
      }
    },
  });
}

export async function view(root) {
  const users = await commands.listUsers();
  const refresh = () => view(root);
  render(
    root,
    html`<div class="inner stack">
      <div class="spread wrap g12">
        <div>
          <h1 style="font-size:21px;font-weight:750">${t('nav.users')}</h1>
          <p class="sm mut" style="margin:2px 0 0">${t('users.subtitle')}</p>
        </div>
        <button class="btn b-pri" data-action="add">+ ${t('users.add')}</button>
      </div>
      <div class="card">
        ${users.map(
          (u) =>
            html`<div class="userrow">
              <div class="av">
                ${u.name
                  .split(/\s+/)
                  .map((x) => x[0])
                  .slice(0, 2)
                  .join('')}
              </div>
              <div class="grow">
                <b>${u.name}</b>
                <div class="xs mut">
                  ${u.username} ·
                  ${t('roles.' + u.role)}${u.classes.length ? ' · ' + u.classes.join(', ') : ''}
                </div>
                <div class="xs mut">
                  ${u.lastLogin ? t('users.lastLogin', { at: formatDateTime(u.lastLogin) }) : t('users.neverSignedIn')}
                </div>
              </div>
              <span
                class="pill ${u.status === 'active' ? 'p-green' : u.status === 'locked' ? 'p-red' : 'p-orange'}"
                >${t('users.status.' + u.status)}</span
              >
              <button class="btn b-out b-sm" data-action="edit" data-id="${u.id}">${t('common.edit')}</button
              >${u.role !== 'principal' ? html`<button class="btn b-quiet b-sm" data-action="reset" data-id="${u.id}">${t('users.reset')}</button><button class="btn b-quiet b-sm" data-action="toggle" data-id="${u.id}" data-active="${u.active}">${u.active ? t('users.switchOff') : t('users.switchOn')}</button>` : ''}${u.locked ? html`<button class="btn b-out b-sm" data-action="unlock" data-id="${u.id}">${t('users.unlock')}</button>` : ''}
            </div>`,
        )}
      </div>
    </div>`,
  );
  delegate(root, {
    add: () => userForm(null, refresh),
    edit: (e, el) =>
      userForm(
        users.find((u) => u.id === el.dataset.id),
        refresh,
      ),
    reset: async (e, el) => {
      try {
        credentialSlip(await commands.resetUserPassword({ userId: el.dataset.id }));
        refresh();
      } catch (x) {
        toast(toAppError(x).message, { kind: 'error' });
      }
    },
    unlock: async (e, el) => {
      try {
        await commands.unlockUser({ userId: el.dataset.id });
        toast(t('users.unlocked'), { kind: 'ok' });
        refresh();
      } catch (x) {
        toast(toAppError(x).message, { kind: 'error' });
      }
    },
    toggle: async (e, el) => {
      const active = el.dataset.active !== 'true';
      if (
        !active &&
        !(await confirmDialog({
          title: t('users.switchOffTitle'),
          message: t('users.switchOffMessage'),
          confirmLabel: t('users.switchOff'),
          danger: true,
        }))
      )
        return;
      try {
        await commands.setUserActive({ userId: el.dataset.id, active });
        refresh();
      } catch (x) {
        toast(toAppError(x).message, { kind: 'error' });
      }
    },
  });
}
