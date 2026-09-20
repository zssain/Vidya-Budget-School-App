import { Button } from '../../components/Button.jsx';
import { useT } from '../../core/i18n.jsx';
import * as commands from '../../api/commands.js';
export function Welcome({ onReady, onSetup }) {
  const t = useT();
  const sample = async () => {
    await commands.loadSampleSchool();
    onReady();
  };
  return (
    <main className="welcome">
      <section className="welcome-card">
        <div className="logo-big">V</div>
        <h1>Vidya</h1>
        <p>{t('welcome.tagline')}</p>
        <div className="col g12">
          <Button onClick={onSetup}>{t('welcome.setup')}</Button>
          <Button kind="outline" onClick={sample}>
            {t('welcome.loadSample')}
          </Button>
        </div>
        <small>{t('welcome.testNote')}</small>
      </section>
    </main>
  );
}
