import { useQuery } from '../../core/useCommand.js';
import { useT } from '../../core/i18n.jsx';
import { EmptyState } from '../../components/EmptyState.jsx';

function Values({ value }) {
  if (value == null) return null;
  if (Array.isArray(value))
    return (
      <div className="list">
        {value.slice(0, 30).map((item, index) => (
          <Values key={item?.id || item?.adm || item?.no || index} value={item} />
        ))}
      </div>
    );
  if (typeof value === 'object')
    return (
      <article className="card">
        <dl>
          {Object.entries(value)
            .slice(0, 12)
            .map(([key, item]) => (
              <div className="spread g12" key={key}>
                <dt className="mut">{key}</dt>
                <dd>{typeof item === 'object' ? <Values value={item} /> : String(item)}</dd>
              </div>
            ))}
        </dl>
      </article>
    );
  return <span>{String(value)}</span>;
}
export function QueryPage({ titleKey, load, actions }) {
  const t = useT();
  const query = useQuery(load, []);
  return (
    <section className="view">
      <div className="page-head spread">
        <div>
          <h1>{t(titleKey)}</h1>
        </div>
        {actions}
      </div>
      {query.loading && !query.data ? (
        <p>{t('login.checking')}</p>
      ) : query.error ? (
        <div className="note red" role="alert">
          {query.error.message}
        </div>
      ) : query.data == null || (Array.isArray(query.data) && !query.data.length) ? (
        <EmptyState title={t('errors.generic')} />
      ) : (
        <Values value={query.data} />
      )}
    </section>
  );
}
