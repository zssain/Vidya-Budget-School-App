import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { formatDateTime } from '../core/format.js';
import { openModal, toast } from '../core/ui.js';
import { downloadFile } from '../core/download.js';
import * as commands from '../api/commands.js';
import { toAppError } from '../api/errors.js';

function passwordModal(title, fields, onSave) {
  openModal({
    title,
    body: html`${fields.map((f) => html`<label class="lbl" style="margin-top:10px">${t(f.label)}</label><input class="inp" type="password" id="${f.id}" autocomplete="off" />`)}
      <div class="err" id="bkErr"></div>`,
    footer: html`<button class="btn b-quiet" data-action="cancel">${t('common.cancel')}</button
      ><button class="btn b-pri" data-action="save">${t('common.save')}</button>`,
    onAction: async (name, { element, close }) => {
      if (name === 'cancel') return close();
      if (name !== 'save') return;
      const modal = element.closest('.modal');
      const values = Object.fromEntries(fields.map((f) => [f.id, modal.querySelector('#' + f.id).value]));
      try {
        await onSave(values);
        close();
      } catch (e) {
        modal.querySelector('#bkErr').textContent = toAppError(e).message;
      }
    },
  });
}

export async function restoreFromFile(onRestored) {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.vidyabak,application/json,application/octet-stream';
  input.addEventListener('change', async () => {
    const file = input.files?.[0];
    if (!file) return;
    let parsed;
    try {
      parsed = JSON.parse(await file.text());
    } catch {
      return toast(t('backup.invalidFile'), { kind: 'error' });
    }
    passwordModal(
      t('backup.restore'),
      [{ id: 'backupPassword', label: 'backup.password' }],
      async ({ backupPassword }) => {
        const preview = await commands.restoreInspect({ file: parsed, backupPassword });
        openModal({
          title: t('backup.restore'),
          locked: true,
          body: html`<div class="note n-red">${t('backup.restoreWarning')}</div>
            <div class="pl"><span class="k">${t('backup.school')}</span><b>${preview.school}</b></div>
            <div class="pl">
              <span class="k">${t('backup.saved')}</span><span>${formatDateTime(preview.createdAt)}</span>
            </div>`,
          footer: html`<button class="btn b-quiet" data-action="cancel">${t('common.cancel')}</button
            ><button class="btn" style="background:var(--red);color:#fff" data-action="restore">
              ${t('backup.restore')}
            </button>`,
          onAction: async (name, { close }) => {
            if (name === 'cancel') return close();
            if (name === 'restore') {
              try {
                await commands.restoreCommit({ previewId: preview.previewId });
                close();
                toast(t('backup.restored'), { kind: 'ok' });
                onRestored?.();
              } catch (e) {
                toast(toAppError(e).message, { kind: 'error' });
              }
            }
          },
        });
      },
    );
  });
  input.click();
}

export async function view(root) {
  const status = await commands.backupStatus();
  const refresh = () => view(root);
  render(
    root,
    html`<div class="inner stack">
      <div>
        <h1 style="font-size:21px;font-weight:750">${t('nav.backup')}</h1>
        <p class="sm mut" style="margin:2px 0 0">${t('backup.subtitle')}</p>
      </div>
      <div class="card">
        <div class="ch">
          <div>
            <h3>${t('backup.file')}</h3>
            <p>
              ${status.lastBackupAt ? t('backup.lastSaved', { at: formatDateTime(status.lastBackupAt) }) : t('backup.neverSaved')}
            </p>
          </div>
          <span class="pill ${status.overdue ? 'p-orange' : 'p-green'}"
            >${t('backup.changesSince', { n: status.changesSince })}</span
          >
        </div>
        <div class="cb">
          <p style="margin-top:0">${t('backup.fileHelp')}</p>
          <div class="row g10 wrap">
            <button class="btn b-pri b-lg" data-action="save">💾 ${t('backup.saveFile')}</button
            ><button class="btn b-out" data-action="restore">${t('backup.restoreFile')}</button>
          </div>
        </div>
      </div>
      <div class="pgrid2" style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
        <div class="card">
          <div class="ch">
            <h3>${t('backup.drive')}</h3>
            <span class="pill p-grey">${t('backup.windowsApp')}</span>
          </div>
          <div class="cb sm">${t('backup.driveHelp')}</div>
        </div>
        <div class="card">
          <div class="ch">
            <h3>${t('backup.daily')}</h3>
            <span class="pill p-grey">${t('backup.windowsApp')}</span>
          </div>
          <div class="cb sm">${t('backup.dailyHelp')}</div>
        </div>
      </div>
      <div class="card">
        <div class="ch"><h3>${t('backup.password')}</h3></div>
        <div class="cb">
          <p style="margin-top:0" class="sm">${t('backup.passwordHelp')}</p>
          <button class="btn b-out" data-action="change-password">${t('backup.changePassword')}</button>
        </div>
      </div>
    </div>`,
  );
  delegate(root, {
    save: () =>
      passwordModal(
        t('backup.saveFile'),
        [{ id: 'backupPassword', label: 'backup.password' }],
        async ({ backupPassword }) => {
          const out = await commands.backupSaveFile({ backupPassword });
          downloadFile(out.filename, out.content);
          toast(t('backup.savedFile', { name: out.filename }), { kind: 'ok' });
          refresh();
        },
      ),
    restore: () => restoreFromFile(refresh),
    'change-password': () =>
      passwordModal(
        t('backup.changePassword'),
        [
          { id: 'current', label: 'backup.currentPassword' },
          { id: 'new', label: 'backup.newPassword' },
          { id: 'again', label: 'password.again' },
        ],
        async (v) => {
          if (v.new !== v.again) throw { kind: 'validation', message: t('password.mismatch') };
          await commands.backupChangePassword({ current: v.current, new: v.new });
          toast(t('backup.passwordChanged'), { kind: 'ok' });
        },
      ),
  });
}
