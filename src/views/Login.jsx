import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { useT } from '../core/i18n.jsx';
import { useCurrentUser } from '../core/session.jsx';
export function Login({ onPassword }) {
  const t = useT();
  const { signIn } = useCurrentUser();
  const [username, setUsername] = useState('sunita@vaani');
  const [password, setPassword] = useState('vidya123');
  const [error, setError] = useState('');
  const submit = async (event) => {
    event.preventDefault();
    setError('');
    try {
      const result = await signIn({ username, password });
      if (result.status === 'must_change_password') onPassword?.(result.pendingToken);
    } catch (caught) {
      setError(caught.message);
    }
  };
  return (
    <main className="login-screen">
      <form className="login-card" onSubmit={submit}>
        <div className="logo-big">V</div>
        <h1>Vidya</h1>
        <p>{t('login.subtitle')}</p>
        {error && (
          <div className="note red" role="alert">
            {error}
          </div>
        )}
        <Field
          label={t('login.usernameLabel')}
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoComplete="username"
        />
        <Field
          label={t('login.passwordLabel')}
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
        />
        <Button type="submit">{t('common.signIn')}</Button>
        <small>{t('login.worksOffline')}</small>
      </form>
    </main>
  );
}
