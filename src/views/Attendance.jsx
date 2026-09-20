import { useMemo, useState } from 'react';
import { Button } from '../components/Button.jsx';
import { useT } from '../core/i18n.jsx';
import { usePrint } from '../core/print.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

const today = () => new Date().toISOString().slice(0, 10);
function Register({ register }) {
  const t = useT();
  return (
    <article className="p-doc">
      <h2>{register.school?.name}</h2>
      <h3>
        {t('attendance.registerTitle', {
          section: register.sectionId,
          month: register.monthName,
          year: register.year,
        })}
      </h3>
      <table>
        <thead>
          <tr>
            <th>{t('students.roll')}</th>
            <th>{t('students.name')}</th>
            {Array.from({ length: register.days }, (_, index) => (
              <th key={index}>{index + 1}</th>
            ))}
            <th>{t('attendance.present')}</th>
          </tr>
        </thead>
        <tbody>
          {register.rows.map((row) => (
            <tr key={row.adm}>
              <td>{row.roll}</td>
              <td>{row.name}</td>
              {row.cells.map((value, index) => (
                <td key={index}>{value}</td>
              ))}
              <td>{row.present}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </article>
  );
}
function AttendanceSheet({ sheet, sectionId, date, reload }) {
  const t = useT();
  const { toast } = useToast();
  const [marks, setMarks] = useState(sheet.marks);
  const mutation = useMutation(commands.saveAttendance);
  const mark = (studentId, value) => setMarks((old) => ({ ...old, [studentId]: value }));
  const save = async () => {
    try {
      await mutation.run({ sectionId, date, marks });
      toast(t('attendance.saved'), { kind: 'ok' });
      await reload();
    } catch {
      // The mutation hook exposes the error below the attendance grid.
    }
  };
  return (
    <>
      {sheet.readOnlyReason && <div className="note n-orange">{sheet.readOnlyReason}</div>}
      <div className="spread wrap g10">
        <div className="sm mut">
          {sheet.savedBy ? t('attendance.savedBy', { name: sheet.savedBy }) : t('attendance.notSaved')}
        </div>
        {!sheet.readOnlyReason && (
          <Button
            kind="outline small"
            onClick={() => setMarks(Object.fromEntries(sheet.students.map((student) => [student.adm, 'P'])))}
          >
            {t('common.markAll')}
          </Button>
        )}
      </div>
      <div className="attgrid">
        {sheet.students.map((student) => (
          <div className="attcard" key={student.adm}>
            <div className="grow">
              <span className="b">{student.name}</span>
              <div className="xs mut">
                {t('students.roll')} {student.roll} · {student.adm}
              </div>
            </div>
            <div className="seg">
              {['P', 'A', 'L'].map((value) => (
                <button
                  key={value}
                  className={marks[student.adm] === value ? 'on' : ''}
                  disabled={Boolean(sheet.readOnlyReason)}
                  onClick={() => mark(student.adm, value)}
                >
                  {value}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
      {mutation.error && (
        <div className="err" role="alert">
          {mutation.error.message}
        </div>
      )}
      {!sheet.readOnlyReason && (
        <Button kind="large" onClick={save} disabled={mutation.pending}>
          {t('common.saveAttendance')}
        </Button>
      )}
    </>
  );
}
export function Attendance() {
  const t = useT();
  const { user } = useCurrentUser();
  const { printElement } = usePrint();
  const sections = useMemo(
    () =>
      user.sections?.length
        ? user.sections
        : (user.schoolClasses || []).flatMap((item) =>
            item.sections.map((section) => `${item.name}-${section}`),
          ),
    [user],
  );
  const [sectionId, setSectionId] = useState(sections[0] || '');
  const [date, setDate] = useState(today);
  const query = useQuery(
    () => (sectionId ? commands.getAttendance({ sectionId, date }) : Promise.resolve(null)),
    [sectionId, date],
  );
  const printRegister = async () => {
    if (!sectionId) return;
    const register = await commands.attendanceRegister({ sectionId, month: date.slice(0, 7) });
    await printElement(<Register register={register} />);
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread wrap g12">
          <div>
            <h1>{t('nav.attendance')}</h1>
            <p className="sm mut">{t('attendance.subtitle')}</p>
          </div>
          <Button kind="outline" onClick={printRegister}>
            {t('attendance.printRegister')}
          </Button>
        </div>
        <div className="row g10 wrap">
          <select
            className="inp"
            aria-label={t('students.class')}
            value={sectionId}
            onChange={(event) => setSectionId(event.target.value)}
          >
            {sections.map((value) => (
              <option key={value}>{value}</option>
            ))}
          </select>
          <input
            className="inp"
            aria-label={t('doc.date')}
            type="date"
            max={today()}
            value={date}
            onChange={(event) => setDate(event.target.value)}
          />
        </div>
        {query.error && (
          <div className="note n-red" role="alert">
            {query.error.message}
          </div>
        )}
        {query.data ? (
          <AttendanceSheet
            key={`${sectionId}-${date}-${query.data.savedAt || ''}`}
            sheet={query.data}
            sectionId={sectionId}
            date={date}
            reload={query.reload}
          />
        ) : (
          !query.loading && <div className="card empty">{t('attendance.noClasses')}</div>
        )}
      </div>
    </section>
  );
}
