import { useState } from 'react';
import { ChipBar } from '../components/ChipBar.jsx';
import { EmptyState } from '../components/EmptyState.jsx';
import { timeAgo } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useQuery } from '../core/useCommand.js';
import * as commands from '../api/commands.js';
const kinds = ['all', 'fee', 'att', 'mark', 'stu', 'user', 'auth', 'settings', 'backup'];
export function Activity() {
  const t = useT();
  const [kind, setKind] = useState('all');
  const [now] = useState(() => Date.now());
  const query = useQuery(() => commands.listActivity({ kind, limit: 300 }), [kind]);
  return (
    <section className="view">
      <div className="inner stack">
        <div>
          <h1>{t('nav.activity')}</h1>
          <p className="mut">{t('activity.subtitle')}</p>
        </div>
        <ChipBar
          items={kinds.map((value) => ({ value, label: t(`activity.kind.${value}`) }))}
          value={kind}
          onChange={setKind}
        />
        <div className="card">
          {query.data?.length
            ? query.data.map((item) => (
                <div className="activity" key={item.seq}>
                  <PillText kind={item.kind} />
                  <span className="grow">
                    {item.text}
                    <small>
                      {item.who} · {item.device}
                    </small>
                  </span>
                  <small>{timeAgo(item.at, now)}</small>
                </div>
              ))
            : !query.loading && <EmptyState title={t('activity.empty')} />}
        </div>
        {query.data?.length === 300 && <p className="mut">{t('activity.latest300')}</p>}
      </div>
    </section>
  );
}
function PillText({ kind }) {
  const t = useT();
  return <span className="pill">{t(`activity.kind.${kind}`)}</span>;
}
