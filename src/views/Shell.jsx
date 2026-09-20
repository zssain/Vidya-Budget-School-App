import { useEffect, useState } from 'react';
import { useT, useLanguage } from '../core/i18n.jsx';
import { useRouter } from '../core/router.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useModal, useToast } from '../core/ui.jsx';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { Icon } from '../components/Icon.jsx';
import { useMutation } from '../core/useCommand.js';
import * as commands from '../api/commands.js';
import { desktopViews } from '../app/views-desktop.js';
function AccountForm() {
  const t = useT();
  const { toast } = useToast();
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [again, setAgain] = useState('');
  const mutation = useMutation(commands.changePassword);
  const save = async (event) => {
    event.preventDefault();
    if (newPassword !== again) return;
    try {
      await mutation.run({ currentPassword, newPassword });
      setCurrentPassword('');
      setNewPassword('');
      setAgain('');
      toast(t('password.saved'), { kind: 'ok' });
    } catch {
      /* The hook exposes password errors beside the fields. */
    }
  };
  return (
    <form className="stack account-form" onSubmit={save}>
      <Field
        label={t('password.currentLabel')}
        type="password"
        value={currentPassword}
        onChange={(event) => setCurrentPassword(event.target.value)}
        error={mutation.fieldError('currentPassword')}
      />
      <Field
        label={t('password.newLabel')}
        type="password"
        value={newPassword}
        onChange={(event) => setNewPassword(event.target.value)}
        error={mutation.fieldError('newPassword')}
      />
      <Field
        label={t('password.again')}
        type="password"
        value={again}
        onChange={(event) => setAgain(event.target.value)}
        error={again && again !== newPassword ? t('password.mismatch') : undefined}
      />
      <Button type="submit">{t('password.saveButton')}</Button>
    </form>
  );
}
export function Shell({ views = desktopViews }) {
  const t = useT();
  const { language, setLanguage } = useLanguage();
  const router = useRouter();
  const session = useCurrentUser();
  const { openModal, closeModal } = useModal();
  const { toast } = useToast();
  const accessible = views.filter((view) => !view.permission || session.can(view.permission));
  const visible = accessible.filter((view) => view.nav !== false);
  const selected = accessible.find((view) => view.id === router.viewId) || visible[0];
  useEffect(() => {
    if (!accessible.some((view) => view.id === router.viewId)) {
      toast(t('errors.noAccess'), { kind: 'error' });
      router.go('home');
    }
  }, [router.viewId, accessible, router, t, toast]);
  const Current = selected.Component;
  const account = () =>
    openModal({
      title: t('account.title'),
      body: (
        <div className="stack">
          <div className="account-identity">
            <span className="account-avatar" aria-hidden="true">
              {session.user.name.slice(0, 1)}
            </span>
            <div>
              <strong>{session.user.name}</strong>
              <span>{session.user.username}</span>
            </div>
          </div>
          <h3 className="account-section-title">{t('account.changePassword')}</h3>
          <AccountForm />
        </div>
      ),
      footer: (
        <div className="spread">
          <Button kind="outline" onClick={closeModal}>
            {t('common.close')}
          </Button>
          <Button
            kind="warn"
            onClick={() => {
              closeModal();
              void session.signOut();
            }}
          >
            {t('common.signOut')}
          </Button>
        </div>
      ),
    });
  return (
    <div className="app-shell">
      <aside className="nav">
        <div className="nav-top">
          <div className="logo">V</div>
          <b>Vidya</b>
        </div>
        <nav className="nav-list" aria-label={t('nav.more')}>
          {visible.map((view) => (
            <button
              className={`nav-item ${selected.id === view.id ? 'on' : ''}`}
              key={view.id}
              onClick={() => router.go(view.id)}
            >
              <span className="ico" aria-hidden="true">
                <Icon name={view.navIcon} size={17} />
              </span>
              {t(view.titleKey)}
            </button>
          ))}
        </nav>
        <button className="nav-user" onClick={account}>
          <span className="av" aria-hidden="true">
            {session.user.name.slice(0, 1)}
          </span>
          <span className="nav-user-copy">
            <b>{session.user.name}</b>
            <span>{t(`roles.${session.user.role}`)}</span>
          </span>
        </button>
      </aside>
      <div className="main">
        <header className="top">
          <h2>{t(selected.titleKey)}</h2>
          <div className="grow" />
          {session.can('backup.run') ? (
            <button className="pill p-green" onClick={() => router.go('backup')}>
              {t('nav.backup')}
            </button>
          ) : (
            <span className="pill p-grey">{t('account.thisDevice')}</span>
          )}
          <button className="btn quiet sm" onClick={() => setLanguage(language === 'en' ? 'hi' : 'en')}>
            {language === 'en' ? 'हिन्दी' : 'English'}
          </button>
        </header>
        <Current {...router.params} />
      </div>
      <nav className="tabbar">
        {visible.slice(0, 4).map((view) => (
          <button
            key={view.id}
            className={selected.id === view.id ? 'on' : ''}
            onClick={() => router.go(view.id)}
          >
            <span className="ti">
              <Icon name={view.navIcon} size={20} />
            </span>
            {t(view.titleKey)}
          </button>
        ))}
        <button onClick={account}>
          <span className="ti">•••</span>
          {t('nav.more')}
        </button>
      </nav>
    </div>
  );
}
