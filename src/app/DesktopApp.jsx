import { useCallback, useState } from 'react';
import { useQuery } from '../core/useCommand.js';
import { useCurrentUser } from '../core/session.jsx';
import { useT } from '../core/i18n.jsx';
import * as commands from '../api/commands.js';
import { Welcome } from '../views/setup/Welcome.jsx';
import { Wizard } from '../views/setup/Wizard.jsx';
import { Credentials } from '../views/setup/Credentials.jsx';
import { Login } from '../views/Login.jsx';
import { Password } from '../views/Password.jsx';
import { Shell } from '../views/Shell.jsx';
import { StartupError } from '../views/StartupError.jsx';
export function DesktopApp({ views }) {
  const t = useT();
  const status = useQuery(commands.appStatus, []);
  const { user } = useCurrentUser();
  const [setup, setSetup] = useState(false);
  const [pending, setPending] = useState(null);
  const [credentials, setCredentials] = useState(null);
  const ready = useCallback(() => {
    setSetup(false);
    void status.reload();
  }, [status]);
  // While the database is opening (P2.4), app_status has not answered yet.
  if (status.loading && !status.data)
    return (
      <main className="welcome">
        <section className="welcome-card">
          <div className="logo-big">V</div>
          <p>{t('startup.opening')}</p>
        </section>
      </main>
    );
  // The database could not be opened: show the full-screen error.
  if (status.data?.ready === false && status.data.failure)
    return (
      <StartupError
        kind={status.data.failure}
        platform={status.data.platform}
        version={status.data.version}
      />
    );
  if (!status.data?.hasSchool)
    if (credentials)
      return (
        <Credentials
          result={credentials}
          onSignIn={() => {
            setCredentials(null);
            void status.reload();
          }}
        />
      );
  if (!status.data?.hasSchool)
    return setup ? (
      <Wizard onBack={() => setSetup(false)} onComplete={setCredentials} />
    ) : (
      <Welcome onReady={ready} onSetup={() => setSetup(true)} />
    );
  if (pending) return <Password pendingToken={pending} onDone={() => setPending(null)} />;
  if (!user) return <Login onPassword={setPending} />;
  return <Shell views={views} />;
}
