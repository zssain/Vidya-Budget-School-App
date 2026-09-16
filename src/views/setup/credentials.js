import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import { printHtml } from '../../core/ui.js';

function slip(c) {
  return html`<div class="slip">
    <div class="spread"><b>${c.name}</b><span class="pill p-blue">${t('roles.' + c.role)}</span></div>
    ${c.classes?.length ? html`<div class="xs mut">${c.classes.join(', ')}</div>` : ''}
    <div class="sm">${t('account.username')}</div>
    <div class="cred">${c.username}</div>
    <div class="sm">${t('users.tempPassword')}</div>
    <div class="cred">${c.tempPassword}</div>
    <div class="xs mut">${t('credentials.firstSignIn')}</div>
  </div>`;
}

export function renderCredentials(root, ctx, result) {
  render(
    root,
    html`<div class="wz">
      <div class="wz-h">
        <div class="xs b" style="color:var(--green)">${t('credentials.created')}</div>
        <h1>${t('credentials.title')}</h1>
      </div>
      <div class="wz-b">
        <div class="note n-orange">${t('credentials.warning')}</div>
        <div class="pl">
          <span class="k">${t('credentials.yourUsername')}</span
          ><span class="v mono">${result.principalUsername}</span>
        </div>
        <div class="slips">
          ${result.credentials.length ? result.credentials.map(slip) : html`<div class="mut">${t('credentials.none')}</div>`}
        </div>
      </div>
      <div class="wz-f">
        <button class="btn b-out" data-action="print">🖨 ${t('credentials.print')}</button
        ><button class="btn b-pri" data-action="login">${t('credentials.signIn')}</button>
      </div>
    </div>`,
  );
  delegate(root, {
    print: () => printHtml(html`<div class="slips">${result.credentials.map(slip)}</div>`),
    login: () => ctx.showLogin(),
  });
}
