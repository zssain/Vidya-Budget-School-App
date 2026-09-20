import { useMemo, useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { ChipBar } from '../../components/ChipBar.jsx';
import { DataTable } from '../../components/DataTable.jsx';
import { EmptyState } from '../../components/EmptyState.jsx';
import { Pill } from '../../components/Pill.jsx';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useQuery } from '../../core/useCommand.js';
import { useToast } from '../../core/ui.jsx';
import { downloadCommandResult } from '../../core/download.js';
import * as commands from '../../api/commands.js';

export function StudentList() {
  const t = useT();
  const router = useRouter();
  const { toast } = useToast();
  const { user, can } = useCurrentUser();
  const [filter, setFilter] = useState({ q: '', sectionId: 'All', status: 'active' });
  const query = useQuery(() => commands.listStudents(filter), [filter.q, filter.sectionId, filter.status]);
  const sections = useMemo(
    () =>
      user.role === 'teacher'
        ? user.sections
        : [...new Set((query.data || []).map((student) => student.ck).filter(Boolean))],
    [query.data, user],
  );
  const columns = [
    { key: 'roll', label: t('students.roll') },
    {
      key: 'name',
      label: t('students.name'),
      render: (student) => (
        <div>
          <span className="b">{student.name}</span>
          <div className="xs mut num">
            {student.adm}
            {student.rte ? ' | RTE' : ''}
            {student.transport ? ` | ${t('students.bus')}` : ''}
          </div>
        </div>
      ),
    },
    { key: 'ck', label: t('students.class'), render: (student) => <Pill kind="p-blue">{student.ck}</Pill> },
    { key: 'father', label: t('students.father') },
    { key: 'mobile', label: t('students.mobile') },
  ];
  if (can('fees.view'))
    columns.push({
      key: 'balance',
      label: t('nav.fees'),
      render: (student) => (
        <Pill kind={student.feeState === 'paid' ? 'p-green' : 'p-orange'}>{student.balance}</Pill>
      ),
    });
  const exportList = async () => {
    try {
      downloadCommandResult(await commands.exportStudentsXlsx({}));
      toast(t('students.downloaded'), { kind: 'ok' });
    } catch (error) {
      toast(error.message, { kind: 'error' });
    }
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread wrap g12">
          <div>
            <h1>{t('nav.students')}</h1>
            <p className="sm mut">
              {user.role === 'teacher'
                ? t('students.yourClasses')
                : t('students.activeCount', { n: query.data?.length || 0 })}
            </p>
          </div>
          <div className="row g8">
            {user.role !== 'teacher' && (
              <Button kind="outline" onClick={exportList}>
                {t('students.excel')}
              </Button>
            )}
            {can('students.add') && (
              <Button onClick={() => router.go('student-form')}>{t('common.addStudent')}</Button>
            )}
          </div>
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
              ...sections.map((value) => ({ value, label: value })),
            ]}
            value={filter.sectionId}
            onChange={(sectionId) => setFilter((old) => ({ ...old, sectionId }))}
          />
          {user.role !== 'teacher' && (
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
        {query.data?.length ? (
          <div className="card">
            <DataTable
              columns={columns}
              rows={query.data.slice(0, 150)}
              rowKey="adm"
              onRow={(student) => router.go('student', { studentId: student.adm })}
            />
          </div>
        ) : (
          !query.loading && <EmptyState title={t('students.noneFound')} message={t('students.tryAnother')} />
        )}
      </div>
    </section>
  );
}
