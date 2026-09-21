import { useMemo, useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { ChipBar } from '../../components/ChipBar.jsx';
import { DataTable } from '../../components/DataTable.jsx';
import { EmptyState } from '../../components/EmptyState.jsx';
import { Pill } from '../../components/Pill.jsx';
import { formatRupees } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useQuery } from '../../core/useCommand.js';
import * as commands from '../../api/commands.js';

export function StudentList() {
  const t = useT();
  const router = useRouter();
  const { can } = useCurrentUser();
  const [filter, setFilter] = useState({ q: '', sectionId: 'All', status: 'active' });
  const query = useQuery(
    () =>
      commands.listStudents({
        q: filter.q,
        sectionId: filter.sectionId === 'All' ? undefined : filter.sectionId,
        status: filter.status,
      }),
    [filter.q, filter.sectionId, filter.status],
  );
  const items = query.data?.items || [];
  const isOffice = items[0]?.shape === 'office';
  const sections = useMemo(() => {
    const map = new Map();
    for (const s of query.data?.items || []) map.set(s.sectionId, `${s.className}-${s.sectionName}`);
    return [...map.entries()].map(([id, label]) => ({ id, label }));
  }, [query.data]);
  const columns = [
    { key: 'roll', label: t('students.roll') },
    {
      key: 'name',
      label: t('students.name'),
      render: (s) => (
        <div>
          <span className="b">{s.name}</span>
          <div className="xs mut num">
            {s.admNo}
            {isOffice && s.rte ? ' | RTE' : ''}
            {isOffice && s.transport ? ` | ${t('students.bus')}` : ''}
          </div>
        </div>
      ),
    },
    {
      key: 'className',
      label: t('students.class'),
      render: (s) => <Pill kind="p-blue">{`${s.className}-${s.sectionName}`}</Pill>,
    },
    { key: 'father', label: t('students.father') },
    { key: 'mobile', label: t('students.mobile') },
  ];
  if (isOffice)
    columns.push({
      key: 'fee',
      label: t('nav.fees'),
      render: (s) => (
        <Pill kind={s.fee.state === 'paid' || s.fee.state === 'rte' ? 'p-green' : 'p-orange'}>
          {formatRupees(s.fee.balance)}
        </Pill>
      ),
    });
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread wrap g12">
          <div>
            <h1>{t('nav.students')}</h1>
            <p className="sm mut">{t('students.activeCount', { n: items.length })}</p>
          </div>
          {can('students.add') && (
            <Button onClick={() => router.go('student-form')}>{t('common.addStudent')}</Button>
          )}
        </div>
        <div className="card cb">
          <input
            className="inp"
            aria-label={t('students.searchPlaceholder')}
            placeholder={t('students.searchPlaceholder')}
            value={filter.q}
            onChange={(event) => setFilter((old) => ({ ...old, q: event.target.value }))}
          />
          <ChipBar
            items={[
              { value: 'All', label: t('students.all') },
              ...sections.map((s) => ({ value: s.id, label: s.label })),
            ]}
            value={filter.sectionId}
            onChange={(sectionId) => setFilter((old) => ({ ...old, sectionId }))}
          />
          {isOffice && (
            <Button
              kind="quiet"
              onClick={() =>
                setFilter((old) => ({ ...old, status: old.status === 'left' ? 'active' : 'left' }))
              }
            >
              {filter.status === 'left' ? t('students.showingLeft') : t('students.showLeft')}
            </Button>
          )}
        </div>
        {query.error && (
          <div className="note n-red" role="alert">
            {query.error.message}
          </div>
        )}
        {items.length ? (
          <div className="card">
            <DataTable
              columns={columns}
              rows={items}
              rowKey="id"
              onRow={(student) => router.go('student', { studentId: student.id })}
            />
          </div>
        ) : (
          !query.loading && <EmptyState title={t('students.noneFound')} message={t('students.tryAnother')} />
        )}
      </div>
    </section>
  );
}
