import { useT } from '../../core/i18n.jsx';
import { Button } from '../../components/Button.jsx';
import { Slip } from '../../components/Slip.jsx';
import { usePrint } from '../../core/print.jsx';
export function Credentials({ result, onSignIn }) {
  const t = useT();
  const { printElement } = usePrint();
  return (
    <main className="wizard">
      <section className="wizard-card">
        <h1>{t('credentials.title')}</h1>
        <div className="note orange">{t('credentials.warning')}</div>
        <div className="card cb">
          <span className="mut">{t('credentials.yourUsername')}</span>
          <strong>{result.principalUsername}</strong>
        </div>
        <div className="stack">
          {result.credentials.length ? (
            result.credentials.map((credential) => <Slip key={credential.username} credential={credential} />)
          ) : (
            <p>{t('credentials.none')}</p>
          )}
        </div>
        <div className="row g8">
          <Button
            kind="outline"
            onClick={() =>
              printElement(
                <div>
                  {result.credentials.map((credential) => (
                    <Slip key={credential.username} credential={credential} />
                  ))}
                </div>,
              )
            }
          >
            {t('credentials.print')}
          </Button>
          <Button onClick={onSignIn}>{t('credentials.signIn')}</Button>
        </div>
      </section>
    </main>
  );
}
