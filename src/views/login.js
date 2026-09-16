// Sign-in screen.
import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

export async function renderLogin(root, ctx, message) {
  let status;
  try {
    status = await commands.appStatus();
  } catch {
    status = {};
  }
  const code = status.schoolCode || '';
  render(
    root,
    html`<div class="lcard">
      <div class="logo">वि</div>
      <div class="lname">${status.schoolName || 'Vidya'}</div>
      <div class="lsub">${t('login.subtitle')}</div>
      <div class="lform">
        <label class="lbl" for="lgUser">${t('login.usernameLabel')}</label>
        <div class="sfx">
          <input
            class="inp"
            id="lgUser"
            data-action="focuspass"
            data-action-key="Enter"
            autocomplete="username"
            autocapitalize="none"
            spellcheck="false"
            placeholder="${t('login.usernamePlaceholder')}"
          /><span>@${code}</span>
        </div>
        <label class="lbl" for="lgPass">${t('login.passwordLabel')}</label>
        <div class="sfx">
          <input
            class="inp"
            id="lgPass"
            type="password"
            data-action="submit"
            data-action-key="Enter"
            autocomplete="current-password"
          />
          <span
            ><button type="button" class="link" style="text-decoration:none" data-action="toggle">
              ${t('login.show')}
            </button></span
          >
        </div>
        <div class="err" id="lgErr">${message || ''}</div>
        <button class="btn b-pri b-lg" id="lgBtn" style="width:100%" data-action="submit">
          ${t('common.signIn')}
        </button>
      </div>
      <p class="xs mut" style="margin:14px 0 0">${t('login.forgot')}</p>
      <div class="offbadge">● ${t('login.worksOffline')}</div>
    </div>`,
  );

  const err = (m) => (root.querySelector('#lgErr').textContent = m || '');
  const btn = root.querySelector('#lgBtn');

  async function doLogin() {
    const username = root.querySelector('#lgUser').value.trim();
    const password = root.querySelector('#lgPass').value;
    err('');
    if (!username || !password) return err(t('login.enterBoth'));
    btn.disabled = true;
    btn.textContent = t('login.checking');
    try {
      const r = await commands.signIn({ username, password });
      if (r.status === 'must_change_password') {
        btn.disabled = false;
        btn.textContent = t('common.signIn');
        ctx.showPasswordFor(r.pendingToken);
        return;
      }
      await ctx.enterApp(r.user);
    } catch (e) {
      btn.disabled = false;
      btn.textContent = t('common.signIn');
      err(toAppError(e).message);
    }
  }

  delegate(root, {
    submit: doLogin,
    focuspass: () => root.querySelector('#lgPass').focus(),
    toggle: (e, el) => {
      const p = root.querySelector('#lgPass');
      p.type = p.type === 'password' ? 'text' : 'password';
      el.textContent = p.type === 'password' ? t('login.show') : t('login.hide');
    },
  });

  setTimeout(() => root.querySelector('#lgUser') && root.querySelector('#lgUser').focus(), 30);
}
