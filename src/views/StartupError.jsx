import { useCallback } from 'react';
import { Button } from '../components/Button.jsx';
import { useT } from '../core/i18n.jsx';
import { useToast } from '../core/ui.jsx';

// Failure kinds that Rust (`StartupFailure`) can report. Anything else falls
// back to the generic "Unknown" message.
const KINDS = ['DataFolder', 'SecureStorage', 'WrongKey', 'Migration', 'Unknown'];

// Full-screen message shown when the app cannot open its database (P2.4).
export function StartupError({ kind, platform, version }) {
  const t = useT();
  const { toast } = useToast();
  const safeKind = KINDS.includes(kind) ? kind : 'Unknown';
  const copyDetails = useCallback(() => {
    // Support details only: no personal data, no key material.
    const details = `Vidya startup error: ${safeKind} (${platform || '-'}, v${version || '-'})`;
    if (navigator.clipboard) void navigator.clipboard.writeText(details);
    toast(t('startup.copied'), { kind: 'ok' });
  }, [safeKind, platform, version, t, toast]);
  return (
    <main className="welcome">
      <section className="welcome-card">
        <div className="logo-big">V</div>
        <h1>{t('startup.errorTitle')}</h1>
        <p>{t(`startup.error.${safeKind}`)}</p>
        <Button kind="outline" onClick={copyDetails}>
          {t('startup.copyDetails')}
        </Button>
      </section>
    </main>
  );
}
