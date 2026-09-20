import { useT } from '../core/i18n.jsx';
export function Slip({ credential = {} }) {
  const t = useT();
  return (
    <article className="p-copy p-doc">
      <h3>{credential.name}</h3>
      <p>
        {t('account.username')}: {credential.username}
      </p>
      <p>
        {t('users.tempPassword')}: {credential.tempPassword}
      </p>
    </article>
  );
}
