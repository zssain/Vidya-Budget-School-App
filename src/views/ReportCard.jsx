import { Button } from '../components/Button.jsx';
import { ReportCardSheet } from '../components/ReportCardSheet.jsx';
import { useT } from '../core/i18n.jsx';
import { usePrint } from '../core/print.jsx';
import { useRouter } from '../core/router.jsx';
import { useQuery } from '../core/useCommand.js';
import * as commands from '../api/commands.js';
export function ReportCard({ studentId }) {
  const t = useT();
  const router = useRouter();
  const { printElement } = usePrint();
  const query = useQuery(
    () => (studentId ? commands.getReportCard({ studentId }) : Promise.resolve(null)),
    [studentId],
  );
  const printClass = async () => {
    const cards = await commands.getClassReportCards({ sectionId: query.data.student.sectionId });
    await printElement(
      <div>
        {cards.map((card) => (
          <ReportCardSheet key={card.student.adm} report={card} />
        ))}
      </div>,
    );
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <h1>{t('students.reportCard')}</h1>
          <Button kind="quiet" onClick={router.back}>
            {t('common.back')}
          </Button>
        </div>
        {query.error && <div className="note n-red">{query.error.message}</div>}
        {query.data && (
          <>
            <ReportCardSheet report={query.data} />
            <div className="row g8 end">
              <Button kind="outline" onClick={printClass}>
                {t('students.class')} · {t('common.print')}
              </Button>
              <Button onClick={() => printElement(<ReportCardSheet report={query.data} />)}>
                {t('common.print')}
              </Button>
            </div>
          </>
        )}
        {!studentId && <div className="empty">{t('students.noneFound')}</div>}
      </div>
    </section>
  );
}
