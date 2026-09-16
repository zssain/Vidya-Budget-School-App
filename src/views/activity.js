import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { timeAgo } from '../core/format.js';
import * as commands from '../api/commands.js';
import { activityRow } from '../components/widgets.js';

const filter = { kind: 'all' };
const kinds = ['all', 'fee', 'att', 'mark', 'stu', 'user', 'auth', 'settings', 'backup'];

export async function view(root) {
  const list = await commands.listActivity({ kind: filter.kind, limit: 300 });
  const now = Date.now();
  render(
    root,
    html`<div class="inner stack">
      <div>
        <h1 style="font-size:21px;font-weight:750">${t('nav.activity')}</h1>
        <p class="sm mut" style="margin:2px 0 0">${t('activity.subtitle')}</p>
      </div>
      <div class="chipbar">
        ${kinds.map((k) => html`<button class="chip ${filter.kind === k ? 'on' : ''}" data-action="kind" data-kind="${k}">${t('activity.kind.' + k)}</button>`)}
      </div>
      <div class="card">
        ${list.length ? list.map((x) => activityRow(x, timeAgo(x.at, now))) : html`<div class="empty">${t('activity.empty')}</div>`}
      </div>
      ${list.length === 300 ? html`<p class="sm mut">${t('activity.latest300')}</p>` : ''}
    </div>`,
  );
  delegate(root, {
    kind: (e, el) => {
      filter.kind = el.dataset.kind;
      view(root);
    },
  });
}
