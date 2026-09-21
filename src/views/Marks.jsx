import { memo, useMemo, useState } from 'react';
import { Button } from '../components/Button.jsx';
import { useT } from '../core/i18n.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

const cellKey = (studentId, subjectId) => `${studentId}|${subjectId}`;

// Memoised so typing in one cell of a 45×8 table does not re-render every row.
const MarkRow = memo(function MarkRow({ student, subjects, values, editable, badCells, onChange }) {
  return (
    <tr>
      <td>{student.roll}</td>
      <td>
        <span className="b">{student.name}</span>
      </td>
      {subjects.map((subject) => {
        const key = cellKey(student.id, subject.id);
        return (
          <td key={subject.id}>
            <input
              className={`inp num-in${badCells.has(key) ? ' bad' : ''}`}
              aria-label={`${student.name} ${subject.name}`}
              value={values[key] ?? ''}
              disabled={!editable}
              onChange={(event) => onChange(key, event.target.value)}
            />
          </td>
        );
      })}
      <td>{student.total}</td>
      <td>
        <span className="grade">{student.grade}</span>
      </td>
    </tr>
  );
});

function MarksSheet({ sheet, sectionId, setExamId, reload }) {
  const t = useT();
  const { toast } = useToast();
  const [values, setValues] = useState(() =>
    Object.fromEntries(
      sheet.students.flatMap((student) =>
        sheet.subjects.map((subject) => [cellKey(student.id, subject.id), student.marks[subject.id] ?? '']),
      ),
    ),
  );
  const mutation = useMutation(commands.saveMarks);
  // Cells the server flagged in the last failed save (studentId|subjectId).
  const badCells = useMemo(() => {
    const set = new Set();
    const raw = mutation.error?.params?.cells;
    if (raw) {
      try {
        for (const cell of JSON.parse(raw)) set.add(cellKey(cell.studentId, cell.subjectId));
      } catch {
        // ignore malformed param
      }
    }
    return set;
  }, [mutation.error]);
  const onChange = (key, value) => setValues((old) => ({ ...old, [key]: value }));
  const save = async () => {
    const entries = Object.entries(values).map(([key, value]) => {
      const [studentId, subjectId] = key.split('|');
      return { studentId, subjectId, value };
    });
    try {
      await mutation.run({ examId: sheet.exam.id, sectionId, entries });
      toast(t('marks.saved'), { kind: 'ok' });
      await reload();
    } catch {
      // The mutation hook exposes the error below the marks table.
    }
  };
  return (
    <>
      <div className="row g10 wrap">
        <select
          className="inp"
          aria-label={t('settings.exam')}
          value={sheet.exam.id}
          onChange={(event) => setExamId(event.target.value)}
        >
          {sheet.exams.map((exam) => (
            <option key={exam.id} value={exam.id}>
              {exam.name}
            </option>
          ))}
        </select>
      </div>
      <div className="card scroll-x">
        <table className="tbl marks">
          <thead>
            <tr>
              <th>{t('students.roll')}</th>
              <th>{t('students.name')}</th>
              {sheet.subjects.map((subject) => (
                <th key={subject.id}>{subject.name}</th>
              ))}
              <th>{t('report.total')}</th>
              <th>{t('report.grade')}</th>
            </tr>
          </thead>
          <tbody>
            {sheet.students.map((student) => (
              <MarkRow
                key={student.id}
                student={student}
                subjects={sheet.subjects}
                values={values}
                editable={sheet.editable}
                badCells={badCells}
                onChange={onChange}
              />
            ))}
          </tbody>
        </table>
      </div>
      {mutation.error && (
        <div className="err" role="alert">
          {mutation.error.message}
        </div>
      )}
      {sheet.editable ? (
        <Button kind="large" onClick={save} disabled={mutation.pending}>
          {t('common.saveMarks')}
        </Button>
      ) : (
        <div className="note n-orange">{t('marks.readOnly')}</div>
      )}
    </>
  );
}

export function Marks() {
  const t = useT();
  const { user } = useCurrentUser();
  const settings = useQuery(() => commands.getSettings(), []);
  const sections = useMemo(() => {
    const all = (settings.data?.classes || []).flatMap((c) =>
      (c.sections || []).map((s) => ({ id: s.id, label: `${c.name}-${s.name}` })),
    );
    return user.sections?.length ? all.filter((s) => user.sections.includes(s.id)) : all;
  }, [settings.data, user.sections]);
  const [sectionId, setSectionId] = useState('');
  const [examId, setExamId] = useState('');
  const active = sectionId || sections[0]?.id || '';
  const query = useQuery(
    () => (active ? commands.getMarksSheet({ examId, sectionId: active }) : Promise.resolve(null)),
    [active, examId],
  );
  return (
    <section className="view">
      <div className="inner stack">
        <div>
          <h1>{t('nav.marks')}</h1>
          <p className="sm mut">
            {query.data ? t('marks.subtitle', { max: query.data.exam.max }) : t('marks.noClasses')}
          </p>
        </div>
        <select
          className="inp"
          aria-label={t('students.class')}
          value={active}
          onChange={(event) => {
            setSectionId(event.target.value);
            setExamId('');
          }}
        >
          {sections.map((section) => (
            <option key={section.id} value={section.id}>
              {section.label}
            </option>
          ))}
        </select>
        {query.error && (
          <div className="note n-red" role="alert">
            {query.error.message}
          </div>
        )}
        {query.data && (
          <MarksSheet
            key={`${active}-${query.data.exam.id}-${query.data.savedAt || ''}`}
            sheet={query.data}
            sectionId={active}
            setExamId={setExamId}
            reload={query.reload}
          />
        )}
      </div>
    </section>
  );
}
