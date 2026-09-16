// Password modal — forced (first sign-in) or normal (change own password).
import { html } from '../core/html.js';
import { t } from '../core/i18n.js';
import { openModal, toast } from '../core/ui.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

export function renderPasswordModal({ forced = false, pendingToken, ctx } = {}) {
  const { element, close } = openModal({
    title: forced ? t('password.setTitle') : t('password.changeTitle'),
    subtitle: forced ? t('password.setSubtitle') : undefined,
    locked: forced,
    body: html`${
        forced
          ? ''
          : html`<label class="lbl">${t('password.currentLabel')}</label
              ><input class="inp" type="password" id="pwOld" autocomplete="current-password" />`
      }
      <label class="lbl" style="margin-top:10px">${t('password.newLabel')}</label>
      <input class="inp" type="password" id="pwNew" autocomplete="new-password" />
      <label class="lbl" style="margin-top:10px">${t('password.again')}</label>
      <input class="inp" type="password" id="pwNew2" autocomplete="new-password" />
      <div class="err" id="pwErr"></div>`,
    footer: html`${
        forced
          ? html`<button class="btn b-quiet" data-ui="signout">${t('common.signOut')}</button>`
          : html`<button class="btn b-quiet" data-ui="cancel">${t('common.cancel')}</button>`
      } <button class="btn b-pri" data-ui="save">${t('password.saveButton')}</button>`,
  });

  const err = (m) => (element.querySelector('#pwErr').textContent = m || '');

  element.addEventListener('click', async (e) => {
    const b = e.target.closest && e.target.closest('[data-ui]');
    if (!b) return;
    const act = b.getAttribute('data-ui');
    if (act === 'cancel') return close();
    if (act === 'signout') {
      close();
      await commands.signOut();
      if (ctx) ctx.showLogin();
      return;
    }
    if (act !== 'save') return;
    const nw = element.querySelector('#pwNew').value;
    const nw2 = element.querySelector('#pwNew2').value;
    if (nw !== nw2) return err(t('password.mismatch'));
    b.disabled = true;
    try {
      if (forced) {
        const r = await commands.setFirstPassword({ pendingToken, newPassword: nw });
        toast(t('password.saved'), { kind: 'ok' });
        close();
        if (ctx) await ctx.enterApp(r.user);
      } else {
        const old = element.querySelector('#pwOld').value;
        await commands.changePassword({ currentPassword: old, newPassword: nw });
        toast(t('password.saved'), { kind: 'ok' });
        close();
      }
    } catch (ex) {
      b.disabled = false;
      err(toAppError(ex).message);
    }
  });
}
