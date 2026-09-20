import { Button } from '../components/Button.jsx';
import { DataTable } from '../components/DataTable.jsx';
import { StatCard } from '../components/StatCard.jsx';
import { formatRupees } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import { downloadCommandResult } from '../core/download.js';
import * as commands from '../api/commands.js';
export function Reports() {
  const t = useT();
  const { toast } = useToast();
  const query = useQuery(commands.reportsSummary, []);
  const report = query.data;
  const columns = [
    { key: 'c', label: t('students.class') },
    { key: 'n', label: t('reports.students') },
    { key: 'boys', label: t('reports.boys') },
    { key: 'girls', label: t('reports.girls') },
    { key: 'rte', label: t('reports.rte') },
    { key: 'due', label: t('reports.feeDue'), render: (row) => formatRupees(row.due) },
    { key: 'col', label: t('reports.collected'), render: (row) => formatRupees(row.col) },
  ];
  const exportReport = async () => {
    try {
      downloadCommandResult(await commands.exportClassSummaryXlsx({}));
      toast(t('reports.downloaded'), { kind: 'ok' });
    } catch (error) {
      toast(error.message, { kind: 'error' });
    }
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t('nav.reports')}</h1>
            <p className="mut">{t('reports.subtitle')}</p>
          </div>
          <Button kind="outline" onClick={exportReport}>
            {t('reports.export')}
          </Button>
        </div>
        {report && (
          <>
            <div className="stats">
              <StatCard
                label={t('reports.enrolment')}
                value={report.enrolment}
                note={t('reports.genderCount', { boys: report.boys, girls: report.girls })}
              />
              <StatCard label={t('reports.rte')} value={report.rte} note={t('reports.studentsLabel')} />
              <StatCard
                label={t('reports.aadhaar')}
                value={report.aadhaarPct == null ? '—' : `${report.aadhaarPct}%`}
                note={t('reports.pendingCount', { n: report.aadhaarPending })}
              />
              <StatCard
                label={t('reports.attendance30')}
                value={report.attendance30 == null ? '—' : `${report.attendance30}%`}
                note={t('reports.marksRecorded', { n: report.attendanceMarks })}
              />
            </div>
            <div className="card">
              <h2>{t('reports.classSummary')}</h2>
              <DataTable columns={columns} rows={report.byClass} rowKey="c" />
            </div>
            <div className="pgrid2">
              <div className="card cb">
                <h3>{t('reports.category')}</h3>
                {report.cats.map((item) => (
                  <div className="pl" key={item.c}>
                    <span>{item.c}</span>
                    <b>{item.n}</b>
                  </div>
                ))}
              </div>
              <div className="card cb">
                <h3>{t('reports.feeStatus')}</h3>
                {report.feeStatus.map((item) => (
                  <div className="pl" key={item.k}>
                    <span>{t(`reports.fee.${item.k}`)}</span>
                    <b>{item.n}</b>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
    </section>
  );
}
