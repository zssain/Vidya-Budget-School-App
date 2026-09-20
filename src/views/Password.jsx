import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { useT } from '../core/i18n.jsx';
import { useMutation } from '../core/useCommand.js';
import * as commands from '../api/commands.js';
import { useCurrentUser } from '../core/session.jsx';
export function Password({ pendingToken, onDone }) {
  const t = useT();
  const { refresh } = useCurrentUser();
  const [newPassword, setNewPassword] = useState('');
  const [again, setAgain] = useState('');
  const mutation = useMutation(commands.setFirstPassword);
  const submit = async (e) => {
    e.preventDefault();
    if (newPassword !== again) return;
    await mutation.run({ pendingToken, newPassword });
    await refresh();
    onDone?.();
  };
  return (
    <main className="login-screen">
      <form className="login-card" onSubmit={submit}>
        <h1>{t('password.setTitle')}</h1>
        <Field
          label={t('password.newLabel')}
          type="password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          error={mutation.fieldError('newPassword')}
        />
        <Field
          label={t('password.again')}
          type="password"
          value={again}
          onChange={(e) => setAgain(e.target.value)}
          error={newPassword !== again && again ? t('password.mismatch') : undefined}
        />
        <Button type="submit">{t('password.saveButton')}</Button>
      </form>
    </main>
  );
}
