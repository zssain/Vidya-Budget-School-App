import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { formatRupees } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';
function Editor({ settings, reload }) {
  const t = useT();
  const { toast } = useToast();
  const [school, setSchool] = useState(settings.school);
  const [classes, setClasses] = useState(settings.classes);
  const [transportFee, setTransportFee] = useState(settings.transportFee);
  const [terms, setTerms] = useState(settings.terms);
  const [exams, setExams] = useState(settings.exams);
  const [deviceCode, setDeviceCode] = useState(settings.deviceCode);
  const schoolMutation = useMutation(commands.saveSchool);
  const classMutation = useMutation(commands.saveClasses);
  const examMutation = useMutation(commands.saveExams);
  const deviceMutation = useMutation(commands.saveDeviceCode);
  const changeClass = (index, field, value) =>
    setClasses((old) => old.map((item, at) => (at === index ? { ...item, [field]: value } : item)));
  const changeExam = (index, field, value) =>
    setExams((old) => old.map((item, at) => (at === index ? { ...item, [field]: value } : item)));
  const run = async (mutation, input, message) => {
    try {
      await mutation.run(input);
      toast(t(message), { kind: 'ok' });
      await reload();
    } catch {
      // The mutation hook renders the validation error in this section.
    }
  };
  return (
    <div className="stack">
      <section className="card cb stack">
        <h2>{t('settings.school')}</h2>
        <div className="fgrid">
          <Field
            label={t('settings.schoolName')}
            value={school.name}
            onChange={(e) => setSchool({ ...school, name: e.target.value })}
            error={schoolMutation.fieldError('name')}
          />
          <Field
            label={t('settings.address')}
            value={school.addr}
            onChange={(e) => setSchool({ ...school, addr: e.target.value })}
          />
          <Field
            label={t('settings.udise')}
            value={school.udise}
            onChange={(e) => setSchool({ ...school, udise: e.target.value })}
            error={schoolMutation.fieldError('udise')}
          />
          <Field
            label={t('settings.board')}
            value={school.board}
            onChange={(e) => setSchool({ ...school, board: e.target.value })}
          />
          <Field
            label={t('settings.session')}
            value={school.session}
            onChange={(e) => setSchool({ ...school, session: e.target.value })}
          />
          <Field
            label={t('settings.phone')}
            value={school.phone}
            onChange={(e) => setSchool({ ...school, phone: e.target.value })}
          />
        </div>
        {schoolMutation.error && !schoolMutation.error.field && (
          <div className="err">{schoolMutation.error.message}</div>
        )}
        <Button onClick={() => run(schoolMutation, school, 'settings.schoolSaved')}>
          {t('settings.saveSchool')}
        </Button>
      </section>
      <section className="card cb stack">
        <div>
          <h2>{t('settings.classesFees')}</h2>
          <p>{t('settings.perTerm')}</p>
        </div>
        <div className="scroll-x">
          <table className="tbl">
            <thead>
              <tr>
                <th>{t('students.class')}</th>
                <th>{t('settings.sections')}</th>
                <th>{t('settings.tuition')}</th>
                <th>{t('settings.exam')}</th>
                <th>{t('settings.other')}</th>
                <th>{t('settings.termTotal')}</th>
                <th>{t('settings.subjects')}</th>
              </tr>
            </thead>
            <tbody>
              {classes.map((item, index) => (
                <tr key={`${item.name}-${index}`}>
                  {['name', 'sections', 'tuition', 'exam', 'other'].map((field) => (
                    <td key={field}>
                      <input
                        className="inp"
                        value={item[field]}
                        onChange={(event) => changeClass(index, field, event.target.value)}
                      />
                    </td>
                  ))}
                  <td>{formatRupees(item.termFee)}</td>
                  <td>
                    <input
                      className="inp"
                      value={Array.isArray(item.subjects) ? item.subjects.join(', ') : item.subjects}
                      onChange={(event) => changeClass(index, 'subjects', event.target.value)}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="fgrid">
          <Field
            label={t('settings.busFee')}
            value={transportFee}
            onChange={(e) => setTransportFee(e.target.value)}
          />
          <Field label={t('settings.terms')} value={terms} onChange={(e) => setTerms(e.target.value)} />
        </div>
        {classMutation.error && <div className="err">{classMutation.error.message}</div>}
        <Button onClick={() => run(classMutation, { classes, transportFee, terms }, 'settings.classesSaved')}>
          {t('settings.saveClasses')}
        </Button>
      </section>
      <section className="card cb stack">
        <h2>{t('settings.exams')}</h2>
        {exams.map((exam, index) => (
          <div className="row g10" key={exam.id}>
            <input
              className="inp"
              aria-label={t('settings.exam')}
              value={exam.name}
              onChange={(event) => changeExam(index, 'name', event.target.value)}
            />
            <input
              className="inp num-in"
              aria-label={t('marks.subtitle', { max: exam.max })}
              value={exam.max}
              onChange={(event) => changeExam(index, 'max', event.target.value)}
            />
          </div>
        ))}
        {examMutation.error && <div className="err">{examMutation.error.message}</div>}
        <Button onClick={() => run(examMutation, { exams }, 'settings.examsSaved')}>
          {t('settings.saveExams')}
        </Button>
      </section>
      <section className="card cb stack">
        <h2>{t('settings.deviceLicense')}</h2>
        <div className="pl">
          <span>{t('settings.schoolCode')}</span>
          <b>@{settings.license.schoolCode}</b>
        </div>
        <div className="pl">
          <span>{t('settings.deviceId')}</span>
          <b>{settings.deviceId}</b>
        </div>
        <div className="pl">
          <span>{t('settings.activation')}</span>
          <b>{settings.license.activationCode}</b>
        </div>
        <div className="pl">
          <span>{t('settings.loginLimit')}</span>
          <b>{settings.license.maxUsers}</b>
        </div>
        <Field
          label={t('settings.receiptPrefix')}
          value={deviceCode}
          onChange={(e) => setDeviceCode(e.target.value)}
          error={deviceMutation.fieldError('code')}
        />
        <Button onClick={() => run(deviceMutation, { code: deviceCode }, 'settings.deviceSaved')}>
          {t('common.save')}
        </Button>
      </section>
    </div>
  );
}
export function Settings() {
  const t = useT();
  const query = useQuery(commands.getSettings, []);
  return (
    <section className="view">
      <div className="inner stack">
        <h1>{t('nav.settings')}</h1>
        {query.error && <div className="note n-red">{query.error.message}</div>}
        {query.data && (
          <Editor
            key={`${query.data.deviceCode}-${query.data.school.name}`}
            settings={query.data}
            reload={query.reload}
          />
        )}
      </div>
    </section>
  );
}
