import { useMemo, useReducer, useState } from 'react';
import { Button } from '../components/Button.jsx';
import { useT } from '../core/i18n.jsx';
import { usePrint } from '../core/print.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

const today = () => new Date().toISOString().slice(0, 10);

// UI-only cycling, in the same order as core `next_status`: none/L → P → A → L.
function nextStatus(current) {
  if (current === 'P') return 'A';
  if (current === 'A') return 'L';
  return 'P';
}
function marksReducer(state, action) {
  switch (action.type) {
    case 'cycle':
      return { ...state, [action.id]: nextStatus(state[action.id]) };
    case 'all':
      return Object.fromEntries(action.ids.map((id) => [id, 'P']));
    default:
      return state;
  }
}

function Register({ register }) {
  const t = useT();
  return (
    <article className="p-doc">
      <h2>{register.school?.name}</h2>
      <h3>
        {t('attendance.registerTitle', {
          section: register.sectionLabel,
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
  const [marks, dispatch] = useReducer(marksReducer, sheet.marks);
  const mutation = useMutation(commands.saveAttendance);
  const readOnly = Boolean(sheet.readOnlyReason);
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
        {!readOnly && (
          <Button
            kind="outline small"
            onClick={() => dispatch({ type: 'all', ids: sheet.students.map((student) => student.id) })}
          >
            {t('common.markAll')}
          </Button>
        )}
      </div>
      <div className="attgrid">
        {sheet.students.map((student) => (
          <button
            className={`attcard status-${marks[student.id] || 'none'}`}
            key={student.id}
            disabled={readOnly}
            aria-label={`${student.name} ${marks[student.id] || ''}`}
            onClick={() => dispatch({ type: 'cycle', id: student.id })}
          >
            <span className="grow">
              <span className="b">{student.name}</span>
              <span className="xs mut">
                {t('students.roll')} {student.roll} · {student.adm}
              </span>
            </span>
            <span className="seg-badge">{marks[student.id] || '—'}</span>
          </button>
        ))}
      </div>
      {mutation.error && (
        <div className="err" role="alert">
          {mutation.error.message}
        </div>
      )}
      {!readOnly && (
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
  const settings = useQuery(() => commands.getSettings(), []);
  const sections = useMemo(() => {
    const all = (settings.data?.classes || []).flatMap((c) =>
      (c.sections || []).map((s) => ({ id: s.id, label: `${c.name}-${s.name}` })),
    );
    // Teachers are scoped to their own sections; the office sees all.
    return user.sections?.length ? all.filter((s) => user.sections.includes(s.id)) : all;
  }, [settings.data, user.sections]);
  const [sectionId, setSectionId] = useState('');
  const [date, setDate] = useState(today);
  const active = sectionId || sections[0]?.id || '';
  const query = useQuery(
    () => (active ? commands.getAttendance({ sectionId: active, date }) : Promise.resolve(null)),
    [active, date],
  );
  const printRegister = async () => {
    if (!active) return;
    const register = await commands.attendanceRegister({ sectionId: active, month: date.slice(0, 7) });
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
            value={active}
            onChange={(event) => setSectionId(event.target.value)}
          >
            {sections.map((section) => (
              <option key={section.id} value={section.id}>
                {section.label}
              </option>
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
            key={`${active}-${date}-${query.data.savedAt || ''}`}
            sheet={query.data}
            sectionId={active}
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
