import { useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { ChipBar } from '../../components/ChipBar.jsx';
import { DataTable } from '../../components/DataTable.jsx';
import { EmptyState } from '../../components/EmptyState.jsx';
import { Pill } from '../../components/Pill.jsx';
import { StatCard } from '../../components/StatCard.jsx';
import { formatRupees } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useQuery } from '../../core/useCommand.js';
import * as commands from '../../api/commands.js';
export function FeeRegister() {
  const t = useT();
  const router = useRouter();
  const [filter, setFilter] = useState({ q: '', sectionId: 'All', state: 'due' });
  const query = useQuery(() => commands.feeRegister(filter), [filter.q, filter.sectionId, filter.state]);
  const data = query.data;
  const columns = [
    {
      key: 'name',
      label: t('fees.student'),
      render: (row) => (
        <div>
          <span className="b">{row.name}</span>
          <div className="xs mut">
            {row.adm} · {row.father} · {row.mobile}
          </div>
        </div>
      ),
    },
    { key: 'ck', label: t('students.class'), render: (row) => <Pill kind="p-blue">{row.ck}</Pill> },
    { key: 'due', label: t('fees.due'), render: (row) => formatRupees(row.due) },
    { key: 'paid', label: t('fees.paid'), render: (row) => formatRupees(row.paid) },
    {
      key: 'balance',
      label: t('fees.balance'),
      render: (row) => <Pill kind={row.balance ? 'p-orange' : 'p-green'}>{formatRupees(row.balance)}</Pill>,
    },
    {
      key: 'action',
      label: '',
      render: (row) =>
        !row.rte && row.balance > 0 ? (
          <Button
            kind="small"
            onClick={(event) => {
              event.stopPropagation();
              router.go('collect-fee', { studentId: row.adm });
            }}
          >
            {t('home.collect')}
          </Button>
        ) : null,
    },
  ];
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread wrap g12">
          <div>
            <h1>{t('nav.fees')}</h1>
            {data && (
              <p className="sm mut">
                {t('fees.sessionInfo', {
                  session: data.session,
                  terms: data.terms,
                  prefix: data.devicePrefix,
                })}
              </p>
            )}
          </div>
          <div className="row g8">
            <Button kind="outline" onClick={() => router.go('daybook')}>
              {t('fees.dayBook')}
            </Button>
            <Button kind="outline">{t('fees.duesList')}</Button>
            <Button onClick={() => router.go('collect-fee')}>{t('common.collectFee')}</Button>
          </div>
        </div>
        {data && (
          <div className="stats">
            <StatCard
              label={t('fees.totalDueSession')}
              value={formatRupees(data.totals.due)}
              note={t('fees.nPaying', { n: data.totals.payingCount })}
            />
            <StatCard
              label={t('fees.collected')}
              value={formatRupees(data.totals.paid)}
              note={t('fees.pctCollected', {
                pct: data.totals.pctCollected,
              })}
            />
            <StatCard
              label={t('fees.pending')}
              value={formatRupees(data.totals.pending)}
              note={t('fees.nStudents', { n: data.totals.unpaid + data.totals.part })}
            />
            <StatCard
              label={t('home.collectedToday')}
              value={formatRupees(data.collectedToday)}
              note={t('fees.nReceipts', { n: data.receiptsToday })}
            />
          </div>
        )}
        <div className="card cb">
          <input
            className="inp"
            aria-label={t('fees.searchPlaceholder')}
            placeholder={t('fees.searchPlaceholder')}
            value={filter.q}
            onChange={(event) => setFilter((old) => ({ ...old, q: event.target.value }))}
          />
          <ChipBar
            items={['due', 'paid', 'rte', 'all'].map((value) => ({
              value,
              label: t(`fees.filter.${value === 'due' ? 'dues' : value === 'paid' ? 'fullyPaid' : value}`),
            }))}
            value={filter.state}
            onChange={(state) => setFilter((old) => ({ ...old, state }))}
          />
        </div>
        {query.error && (
          <div className="note n-red" role="alert">
            {query.error.message}
          </div>
        )}
        {data?.list.length ? (
          <div className="card">
            <DataTable
              columns={columns}
              rows={data.list.slice(0, 200)}
              rowKey="adm"
              onRow={(row) => router.go('student', { studentId: row.adm })}
            />
          </div>
        ) : (
          data && <EmptyState title={t('fees.noMatch')} />
        )}
      </div>
    </section>
  );
}
