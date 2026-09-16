// First-run welcome screen.
import { html, render } from '../../core/html.js';
import { delegate } from '../../core/dom.js';
import { t } from '../../core/i18n.js';
import { toast } from '../../core/ui.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { renderWizard } from './wizard.js';
import { restoreFromFile } from '../backup.js';

export function renderWelcome(root, ctx) {
  render(
    root,
    html`<div class="wz" style="max-width:520px">
      <div class="wz-b" style="text-align:center;padding-top:34px">
        <div class="logo">वि</div>
        <h1 style="font-size:25px;font-weight:750;letter-spacing:-0.5px">Vidya</h1>
        <p class="mut" style="margin:4px 0 22px">${t('welcome.tagline')}</p>
        <div class="stack" style="gap:10px">
          <button class="btn b-pri b-lg" data-action="setup">${t('welcome.setup')}</button>
          <button class="btn b-out" data-action="restore">${t('welcome.restore')}</button>
          <button class="btn b-pri b-lg" data-action="sample">${t('welcome.loadSample')}</button>
        </div>
        <div class="note n-orange" style="text-align:left;margin-top:22px">${t('welcome.testNote')}</div>
      </div>
    </div>`,
  );
  delegate(root, {
    setup: () => renderWizard(root, ctx),
    restore: () => restoreFromFile(() => ctx.showLogin()),
    sample: async (e, el) => {
      el.disabled = true;
      try {
        await commands.loadSampleSchool();
        toast(t('welcome.sampleReady'), { kind: 'ok' });
        ctx.showLogin();
      } catch (err) {
        el.disabled = false;
        toast(toAppError(err).message, { kind: 'error' });
      }
    },
  });
}
