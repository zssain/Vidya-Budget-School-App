import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { EmptyState } from '../components/EmptyState.jsx';
import { Pill } from '../components/Pill.jsx';
import { StatCard } from '../components/StatCard.jsx';
import { formatRupees, timeAgo } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useRouter } from '../core/router.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useQuery } from '../core/useCommand.js';
import * as commands from '../api/commands.js';
function Hero({ name, subtitle, children }) {
  const t = useT();
  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'home.morning' : hour < 17 ? 'home.afternoon' : 'home.evening';
  return (
    <div className="card home-hero">
      <div>
        <h1>
          {t(greeting)}, {name.split(' ')[0]}
        </h1>
        <p>{subtitle}</p>
      </div>
      <div className="row g8">{children}</div>
    </div>
  );
}
function Principal() {
  const t = useT();
  const router = useRouter();
  const query = useQuery(commands.homePrincipal, []);
  const [now] = useState(() => Date.now());
  const data = query.data;
  if (!data) return null;
  return (
    <main className="view">
      <div className="inner stack">
        <Hero
          name={data.name}
          subtitle={t('home.principalSub', {
            marked: data.sections - data.pending.length,
            total: data.sections,
            unpaid: data.fees.unpaid,
          })}
        >
          <Button onClick={() => router.go('attendance')}>{t('nav.attendance')}</Button>
          <Button onClick={() => router.go('collect-fee')}>{t('common.collectFee')}</Button>
        </Hero>
        <div className="stats">
          <StatCard
            label={t('home.students')}
            value={data.students}
            note={t('home.classesSections', { classes: data.classes, sections: data.sections })}
          />
          <StatCard
            label={t('home.presentToday')}
            value={data.attendance.done ? data.attendance.p : '—'}
            note={
              data.attendance.done
                ? t('home.absentLeave', { absent: data.attendance.a, leave: data.attendance.l })
                : t('home.noSectionMarked')
            }
          />
          <StatCard
            label={t('home.feesCollected')}
            value={formatRupees(data.fees.paid)}
            note={t('home.ofTotal', { pct: data.fees.pctCollected, total: formatRupees(data.fees.due) })}
          />
          <StatCard
            label={t('home.feesPending')}
            value={formatRupees(data.fees.pending)}
            note={t('home.unpaidPart', { unpaid: data.fees.unpaid, part: data.fees.part })}
          />
        </div>
        <section className="card">
          <div className="ch">
            <h2>{t('home.attendancePending')}</h2>
          </div>
          {data.pending.length ? (
            data.pending.map((item) => (
              <div className="feerow" key={item.ck}>
                <Pill kind="p-blue">{item.ck}</Pill>
                <span className="grow home-row-copy">
                  <strong>{t('home.nStudents', { n: item.count })}</strong>
                  <small>{item.teachers.join(', ') || t('home.noTeacher')}</small>
                </span>
                <Button kind="small" onClick={() => router.go('attendance', { ck: item.ck })}>
                  {t('home.mark')}
                </Button>
              </div>
            ))
          ) : (
            <EmptyState title={t('home.everyMarked')} />
          )}
        </section>
        <section className="card">
          <div className="ch">
            <h2>{t('home.recentActivity')}</h2>
            <Button kind="quiet" onClick={() => router.go('activity')}>
              {t('home.seeAll')}
            </Button>
          </div>
          {data.recentActivity.length ? (
            data.recentActivity.map((item) => (
              <div className="activity" key={item.seq}>
                <span>{item.text}</span>
                <small>
                  {item.who} · {timeAgo(item.at, now)}
                </small>
              </div>
            ))
          ) : (
            <EmptyState title={t('home.nothingYet')} />
          )}
        </section>
      </div>
    </main>
  );
}
function Accountant() {
  const t = useT();
  const router = useRouter();
  const query = useQuery(commands.homeAccountant, []);
  const data = query.data;
  if (!data) return null;
  return (
    <main className="view">
      <div className="inner stack">
        <Hero
          name={data.name}
          subtitle={t('home.accountantSub', {
            receipts: data.receiptsToday,
            pending: formatRupees(data.fees.pending),
          })}
        >
          <Button onClick={() => router.go('collect-fee')}>{t('common.collectFee')}</Button>
        </Hero>
        <div className="stats">
          <StatCard
            label={t('home.collectedToday')}
            value={formatRupees(data.collectedToday)}
            note={t('home.byMode', {
              cash: formatRupees(data.byMode.Cash),
              upi: formatRupees(data.byMode.UPI),
              cheque: formatRupees(data.byMode.Cheque),
            })}
          />
          <StatCard
            label={t('home.receiptsToday')}
            value={data.receiptsToday}
            note={t('home.numberedPrefix', { prefix: data.devicePrefix })}
          />
          <StatCard label={t('home.pendingSession')} value={formatRupees(data.fees.pending)} />
          <StatCard label={t('home.collectedSession')} value={formatRupees(data.fees.paid)} />
        </div>
        <section className="card">
          {data.topDue.map((student) => (
            <div className="feerow" key={student.adm}>
              <span className="grow">
                <span className="b">{student.name}</span>
                <small>
                  {student.ck} · {student.father}
                </small>
              </span>
              <strong>{formatRupees(student.balance)}</strong>
              <Button kind="small" onClick={() => router.go('collect-fee', { studentId: student.adm })}>
                {t('home.collect')}
              </Button>
            </div>
          ))}
        </section>
      </div>
    </main>
  );
}
function Teacher() {
  const t = useT();
  const router = useRouter();
  const query = useQuery(commands.homeTeacher, []);
  const data = query.data;
  if (!data) return null;
  return (
    <main className="view">
      <div className="inner stack">
        <Hero
          name={data.name}
          subtitle={
            data.pending.length
              ? t('home.attendanceStillFor', { classes: data.pending.join(', ') })
              : t('home.attendanceDone')
          }
        >
          <Button onClick={() => router.go('attendance')}>{t('home.takeAttendance')}</Button>
          <Button kind="outline" onClick={() => router.go('marks')}>
            {t('home.enterMarks')}
          </Button>
        </Hero>
        <div className="home-classes">
          {data.classes.map((item) => (
            <section className="card" key={item.ck}>
              <div className="ch">
                <div>
                  <h2>{t('home.class', { ck: item.ck })}</h2>
                  <p>{t('home.nStudents', { n: item.count })}</p>
                </div>
                <Pill kind={item.marked ? 'p-green' : 'p-orange'}>
                  {item.marked ? t('home.nPresent', { n: item.present }) : t('home.notMarked')}
                </Pill>
              </div>
              <div className="cb row g8">
                <Button onClick={() => router.go('attendance', { ck: item.ck })}>
                  {item.marked ? t('home.viewAttendance') : t('home.takeAttendance')}
                </Button>
                <Button kind="outline" onClick={() => router.go('marks', { ck: item.ck })}>
                  {t('nav.marks')}
                </Button>
              </div>
            </section>
          ))}
        </div>
      </div>
    </main>
  );
}
export const PrincipalHome = Principal;
export const AccountantHome = Accountant;
export const TeacherHome = Teacher;
export function Home() {
  const { user } = useCurrentUser();
  return user.role === 'teacher' ? <Teacher /> : user.role === 'accountant' ? <Accountant /> : <Principal />;
}
