import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { Pill } from '../components/Pill.jsx';
import { formatDateTime } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useCurrentUser } from '../core/session.jsx';
import { useModal, useToast } from '../core/ui.jsx';
import { downloadCommandResult } from '../core/download.js';
import * as commands from '../api/commands.js';
function PasswordForm({ fields, onSave }) {
  const t = useT();
  const [values, setValues] = useState(Object.fromEntries(fields.map((field) => [field.name, ''])));
  const [error, setError] = useState('');
  const submit = async (event) => {
    event.preventDefault();
    setError('');
    try {
      await onSave(values);
    } catch (caught) {
      setError(caught.message);
    }
  };
  return (
    <form className="stack" onSubmit={submit}>
      {fields.map((field) => (
        <Field
          key={field.name}
          label={t(field.label)}
          type="password"
          value={values[field.name]}
          onChange={(event) => setValues((old) => ({ ...old, [field.name]: event.target.value }))}
        />
      ))}
      {error && (
        <div className="err" role="alert">
          {error}
        </div>
      )}
      <Button type="submit">{t('common.save')}</Button>
    </form>
  );
}
export function Backup() {
  const t = useT();
  const query = useQuery(commands.backupStatus, []);
  const { openModal, closeModal } = useModal();
  const { toast } = useToast();
  const { can } = useCurrentUser();
  const restoreMutation = useMutation(commands.restoreCommit);
  const save = () =>
    openModal({
      title: t('backup.saveFile'),
      body: (
        <PasswordForm
          fields={[{ name: 'backupPassword', label: 'backup.password' }]}
          onSave={async (values) => {
            const out = await commands.backupSaveFile(values);
            downloadCommandResult(out);
            closeModal();
            toast(t('backup.savedFile', { name: out.filename || out.path }), { kind: 'ok' });
            await query.reload();
          }}
        />
      ),
    });
  const changePassword = () =>
    openModal({
      title: t('backup.changePassword'),
      body: (
        <PasswordForm
          fields={[
            { name: 'current', label: 'backup.currentPassword' },
            { name: 'new', label: 'backup.newPassword' },
            { name: 'again', label: 'password.again' },
          ]}
          onSave={async (values) => {
            if (values.new !== values.again) throw new Error(t('password.mismatch'));
            await commands.backupChangePassword({ current: values.current, new: values.new });
            closeModal();
            toast(t('backup.passwordChanged'), { kind: 'ok' });
          }}
        />
      ),
    });
  const inspect = async (file, password) => {
    let parsed;
    try {
      parsed = JSON.parse(await file.text());
    } catch {
      throw new Error(t('backup.invalidFile'));
    }
    return commands.restoreInspect({ file: parsed, backupPassword: password });
  };
  const chooseRestore = (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    openModal({
      title: t('backup.restore'),
      body: (
        <PasswordForm
          fields={[{ name: 'backupPassword', label: 'backup.password' }]}
          onSave={async ({ backupPassword }) => {
            const preview = await inspect(file, backupPassword);
            openModal({
              title: t('backup.restore'),
              locked: true,
              body: (
                <div className="stack">
                  <div className="note n-red">{t('backup.restoreWarning')}</div>
                  <div className="pl">
                    <span>{t('backup.school')}</span>
                    <b>{preview.school}</b>
                  </div>
                  <div className="pl">
                    <span>{t('backup.saved')}</span>
                    <span>{formatDateTime(preview.createdAt)}</span>
                  </div>
                </div>
              ),
              footer: (
                <div className="row g8">
                  <Button kind="outline" onClick={closeModal}>
                    {t('common.cancel')}
                  </Button>
                  <Button
                    kind="warn"
                    onClick={async () => {
                      await restoreMutation.run({ previewId: preview.previewId });
                      closeModal();
                      toast(t('backup.restored'), { kind: 'ok' });
                    }}
                  >
                    {t('backup.restore')}
                  </Button>
                </div>
              ),
            });
          }}
        />
      ),
    });
  };
  const status = query.data;
  const runNow = async () => {
    try {
      await commands.runBackupNow();
      toast(t('backup.runNow'), { kind: 'ok' });
      await query.reload();
    } catch (error) {
      toast(error.message, { kind: 'error' });
    }
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div>
          <h1>{t('nav.backup')}</h1>
          <p className="mut">{t('backup.subtitle')}</p>
        </div>
        {status && (
          <section className="card">
            <div className="ch">
              <div>
                <h2>{t('backup.file')}</h2>
                <p>
                  {status.lastBackupAt
                    ? t('backup.lastSaved', { at: formatDateTime(status.lastBackupAt) })
                    : t('backup.neverSaved')}
                </p>
              </div>
              <Pill kind={status.overdue ? 'p-orange' : 'p-green'}>
                {t('backup.changesSince', { n: status.changesSince })}
              </Pill>
            </div>
            <div className="cb stack">
              <p>{t('backup.fileHelp')}</p>
              <div className="row g10 wrap">
                <Button kind="large" onClick={runNow}>
                  {t('backup.runNow')}
                </Button>
                {can('backup.manage') && (
                  <Button kind="outline" onClick={save}>
                    {t('backup.saveFile')}
                  </Button>
                )}
                {can('backup.manage') && (
                  <label className="btn outline">
                    {t('backup.restoreFile')}
                    <input className="hide" type="file" accept=".vidyabak" onChange={chooseRestore} />
                  </label>
                )}
              </div>
            </div>
          </section>
        )}
        {can('backup.manage') && (
          <section className="card cb stack">
            <h2>{t('backup.password')}</h2>
            <p>{t('backup.passwordHelp')}</p>
            <Button kind="outline" onClick={changePassword}>
              {t('backup.changePassword')}
            </Button>
          </section>
        )}
      </div>
    </section>
  );
}
