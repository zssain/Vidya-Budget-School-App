import { useMemo, useState } from 'react';
import { Button } from '../components/Button.jsx';
import { useT } from '../core/i18n.jsx';
import { useCurrentUser } from '../core/session.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

function MarksSheet({ sheet, sectionId, setExamId, reload }) {
  const t = useT();
  const { toast } = useToast();
  const [values, setValues] = useState(() =>
    Object.fromEntries(
      sheet.students.flatMap((student) =>
        sheet.subjects.map((subject) => [`${student.adm}|${subject}`, student.marks[subject] ?? '']),
      ),
    ),
  );
  const mutation = useMutation(commands.saveMarks);
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
                <th key={subject}>{subject}</th>
              ))}
              <th>{t('report.total')}</th>
              <th>{t('report.grade')}</th>
            </tr>
          </thead>
          <tbody>
            {sheet.students.map((student) => (
              <tr key={student.adm}>
                <td>{student.roll}</td>
                <td>
                  <span className="b">{student.name}</span>
                </td>
                {sheet.subjects.map((subject) => {
                  const key = `${student.adm}|${subject}`;
                  return (
                    <td key={subject}>
                      <input
                        className="inp num-in"
                        aria-label={`${student.name} ${subject}`}
                        value={values[key]}
                        disabled={!sheet.editable}
                        onChange={(event) => setValues((old) => ({ ...old, [key]: event.target.value }))}
                      />
                    </td>
                  );
                })}
                <td>{student.total}</td>
                <td>
                  <span className="grade">{student.grade}</span>
                </td>
              </tr>
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
  const [examId, setExamId] = useState('');
  const query = useQuery(
    () => (sectionId ? commands.getMarksSheet({ examId, sectionId }) : Promise.resolve(null)),
    [sectionId, examId],
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
          value={sectionId}
          onChange={(event) => {
            setSectionId(event.target.value);
            setExamId('');
          }}
        >
          {sections.map((value) => (
            <option key={value}>{value}</option>
          ))}
        </select>
        {query.error && (
          <div className="note n-red" role="alert">
            {query.error.message}
          </div>
        )}
        {query.data && (
          <MarksSheet
            key={`${sectionId}-${query.data.exam.id}-${query.data.savedAt || ''}`}
            sheet={query.data}
            sectionId={sectionId}
            setExamId={setExamId}
            reload={query.reload}
          />
        )}
      </div>
    </section>
  );
}
