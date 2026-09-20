import { formatDate } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
export function ReportCardSheet({ report }) {
  const t = useT();
  return (
    <article className="rc p-doc">
      <h2>{report.school.name}</h2>
      <h3>{t('report.progress', { session: report.school.session })}</h3>
      <table>
        <tbody>
          <tr>
            <th>{t('report.name')}</th>
            <td>{report.student.name}</td>
            <th>{t('report.class')}</th>
            <td>{report.student.ck}</td>
            <th>{t('report.roll')}</th>
            <td>{report.student.roll}</td>
          </tr>
          <tr>
            <th>{t('report.father')}</th>
            <td>{report.student.father}</td>
            <th>{t('report.admNo')}</th>
            <td>{report.student.adm}</td>
            <th>{t('report.dob')}</th>
            <td>{formatDate(report.student.dob)}</td>
          </tr>
        </tbody>
      </table>
      <table>
        <thead>
          <tr>
            <th>{t('report.subject')}</th>
            {report.exams.map((exam) => (
              <th key={exam.id}>
                {exam.name}
                <br />
                <small>{t('report.outOf', { max: exam.max })}</small>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {report.rows.map((row) => (
            <tr key={row.subject}>
              <td>{row.subject}</td>
              {row.byExam.map((value, index) => (
                <td key={report.exams[index].id}>{value}</td>
              ))}
            </tr>
          ))}
          <tr>
            <th>{t('report.total')}</th>
            {report.totals.map((total, index) => (
              <th key={report.exams[index].id}>{total.entered ? `${total.got}/${total.max}` : '—'}</th>
            ))}
          </tr>
          <tr>
            <th>{t('report.percentage')}</th>
            {report.totals.map((total, index) => (
              <th key={report.exams[index].id}>{total.pct == null ? '—' : `${total.pct}%`}</th>
            ))}
          </tr>
          <tr>
            <th>{t('report.grade')}</th>
            {report.totals.map((total, index) => (
              <th key={report.exams[index].id}>{total.entered ? total.grade : '—'}</th>
            ))}
          </tr>
        </tbody>
      </table>
      <p>
        {t('report.attendance')}:{' '}
        {report.attendance.t
          ? t('students.daysPct', {
              p: report.attendance.p,
              t: report.attendance.t,
              pct: report.attendance.pct,
            })
          : t('report.notRecorded')}
      </p>
      <p>{t('report.remarks')}: ____________________________________________</p>
      <div className="spread signatures">
        <span>{t('report.classTeacher')}</span>
        <span>{t('report.principal')}</span>
        <span>{t('report.parent')}</span>
      </div>
    </article>
  );
}
