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
  const isOffice = student.shape === 'office';
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
              {student.className}-{student.sectionName} · {t('students.roll')} {student.roll} ·{' '}
              {student.admNo}
            </p>
          </div>
          {can('students.edit') && (
            <Button kind="outline" onClick={() => router.go('student-form', { studentId: student.id })}>
              {t('common.edit')}
            </Button>
          )}
        </div>
        {student.status === 'left' && (
          <div className="note n-orange">
            {t('students.leftSchool')}
            {student.leftOn ? ` · ${formatDate(student.leftOn)}` : ''}
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
              <b>{student.dob ? formatDate(student.dob) : '—'}</b>
            </p>
            <p>
              <span className="mut">{t('students.gender')}</span>
              <br />
              <b>{student.gender}</b>
            </p>
            {isOffice && (
              <p>
                <span className="mut">{t('students.category')}</span>
                <br />
                <Pill>{student.category}</Pill>
              </p>
            )}
          </div>
        </div>
        {isOffice && (
          <div className="card cb">
            <h2>{t('students.feesSession')}</h2>
            {student.rte ? (
              <div className="note n-green">{t('students.rteNote')}</div>
            ) : (
              <div className="stats">
                <StatCard label={t('students.totalDue')} value={formatRupees(student.fee.due)} />
                <StatCard label={t('fees.collected')} value={formatRupees(student.fee.paid)} />
                <StatCard label={t('fees.balance')} value={formatRupees(student.fee.balance)} />
              </div>
            )}
          </div>
        )}
        {isOffice && student.receipts?.length > 0 && (
          <div className="card cb">
            <h2>{t('students.receipts')}</h2>
            {student.receipts.map((r) => (
              <div className="spread xs" key={r.id}>
                <span className={r.cancelled ? 'strike' : ''}>
                  {r.receiptNo} · {formatDate(r.paidOn)} · {r.mode}
                </span>
                <b className={r.cancelled ? 'strike' : ''}>{formatRupees(r.amount)}</b>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
