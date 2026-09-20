import { Button } from '../../components/Button.jsx';
import { Pill } from '../../components/Pill.jsx';
import { StatCard } from '../../components/StatCard.jsx';
import { formatDate, formatRupees } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useQuery } from '../../core/useCommand.js';
import * as commands from '../../api/commands.js';

export function StudentDetail({ studentId }) {
  const t = useT();
  const router = useRouter();
  const { can } = useCurrentUser();
  const query = useQuery(() => commands.getStudent({ studentId }), [studentId]);
  const student = query.data;
  if (query.loading && !student) return null;
  if (query.error)
    return (
      <section className="view">
        <div className="note n-red" role="alert">
          {query.error.message}
        </div>
      </section>
    );
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread wrap g12">
          <div>
            <Button kind="quiet" onClick={router.back}>
              {t('common.back')}
            </Button>
            <h1>{student.name}</h1>
            <p className="mut">
              {t('students.classRoll', { ck: student.ck, roll: student.roll })} · {student.adm}
            </p>
          </div>
          <div className="row g8">
            {can('students.edit') && (
              <Button kind="outline" onClick={() => router.go('student-form', { studentId: student.adm })}>
                {t('common.edit')}
              </Button>
            )}
            <Button onClick={() => router.go('reportcard', { studentId: student.adm })}>
              {t('students.reportCard')}
            </Button>
          </div>
        </div>
        {student.status === 'left' && (
          <div className="note n-orange">
            {t('students.leftSchool')} · {formatDate(student.leftOn)}
          </div>
        )}
        <div className="card cb">
          <h2>{t('students.details')}</h2>
          <div className="fgrid">
            <p>
              <span className="mut">{t('students.father')}</span>
              <br />
              <b>{student.father}</b>
            </p>
            <p>
              <span className="mut">{t('students.mother')}</span>
              <br />
              <b>{student.mother || '—'}</b>
            </p>
            <p>
              <span className="mut">{t('students.mobile')}</span>
              <br />
              <b>{student.mobile}</b>
            </p>
            <p>
              <span className="mut">{t('students.dob')}</span>
              <br />
              <b>{formatDate(student.dob)}</b>
            </p>
            <p>
              <span className="mut">{t('students.gender')}</span>
              <br />
              <b>{student.gender}</b>
            </p>
            <p>
              <span className="mut">{t('students.category')}</span>
              <br />
              <Pill>{student.cat}</Pill>
            </p>
          </div>
        </div>
        {can('fees.view') && (
          <div className="card cb">
            <h2>{t('students.feesSession')}</h2>
            {student.rte ? (
              <div className="note n-green">{t('students.rteNote')}</div>
            ) : (
              <div className="stats">
                <StatCard label={t('students.totalDue')} value={formatRupees(student.feeDue)} />
                <StatCard label={t('fees.collected')} value={formatRupees(student.paid)} />
                <StatCard label={t('fees.balance')} value={formatRupees(student.balance)} />
              </div>
            )}
          </div>
        )}
        <div className="card cb">
          <h2>{t('students.attendanceMarks')}</h2>
          <div className="stats">
            <StatCard
              label={t('students.thisMonth')}
              value={student.attendanceMonth?.pct == null ? '—' : `${student.attendanceMonth.pct}%`}
            />
            <StatCard
              label={t('students.thisSession')}
              value={student.attendanceSession?.pct == null ? '—' : `${student.attendanceSession.pct}%`}
            />
            {student.exams?.map((exam) => (
              <StatCard
                key={exam.id}
                label={exam.name}
                value={exam.entered ? `${exam.pct}% · ${exam.grade}` : t('students.noMarksYet')}
              />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
