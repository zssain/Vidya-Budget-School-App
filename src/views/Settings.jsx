import { useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { useT } from '../core/i18n.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useConfirm, useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

const BOARDS = ['State Board', 'CBSE', 'ICSE', 'U.P. Board', 'Other'];
const TERMS = [1, 2, 3, 4, 12];

function useSectionSave(command, message, reload) {
  const t = useT();
  const { toast } = useToast();
  const mutation = useMutation(command);
  const save = async (input) => {
    try {
      await mutation.run(input);
      toast(t(message), { kind: 'ok' });
      await reload();
      return true;
    } catch {
      return false;
    }
  };
  return { mutation, save };
}

function SchoolSettings({ school, reload }) {
  const t = useT();
  const [form, setForm] = useState(school);
  const { mutation, save } = useSectionSave(commands.saveSchool, 'settings.schoolSaved', reload);
  const set = (name, value) => setForm((old) => ({ ...old, [name]: value }));
  return (
    <section className="card cb stack">
      <h2>{t('settings.school')}</h2>
      <div className="fgrid">
        <Field
          label={t('settings.schoolName')}
          value={form.name}
          onChange={(e) => set('name', e.target.value)}
          error={mutation.fieldError('name')}
        />
        <Field
          label={t('settings.address')}
          value={form.address}
          onChange={(e) => set('address', e.target.value)}
        />
        <Field
          label={t('settings.udise')}
          value={form.udise}
          onChange={(e) => set('udise', e.target.value)}
          error={mutation.fieldError('udise')}
        />
        <Field
          as="select"
          label={t('settings.board')}
          value={form.board || 'State Board'}
          onChange={(e) => set('board', e.target.value)}
          error={mutation.fieldError('board')}
        >
          {BOARDS.map((b) => (
            <option key={b}>{b}</option>
          ))}
        </Field>
        <Field label={t('settings.phone')} value={form.phone} onChange={(e) => set('phone', e.target.value)} />
      </div>
      {mutation.error && !mutation.error.field && <div className="err">{mutation.error.message}</div>}
      <Button onClick={() => save(form)}>{t('settings.saveSchool')}</Button>
    </section>
  );
}

function ClassSettings({ classes, session, reload }) {
  const t = useT();
  const confirm = useConfirm();
  const [rows, setRows] = useState(
    classes.map((c) => ({
      id: c.id,
      name: c.name,
      sections: c.sections.length || 1,
      tuition: String(c.feePlan?.tuition ?? 0),
      exam: String(c.feePlan?.exam ?? 0),
      other: String(c.feePlan?.other ?? 0),
    })),
  );
  const [terms, setTerms] = useState(session?.terms ?? 3);
  const [transport, setTransport] = useState(String(session?.transportFeePerTerm ?? 0));
  const structure = useSectionSave(commands.saveClasses, 'settings.classesSaved', reload);
  const feeMutation = useMutation(commands.saveFeePlan);
  const { toast } = useToast();
  const change = (index, field, value) =>
    setRows((old) => old.map((r, i) => (i === index ? { ...r, [field]: value } : r)));
  const addClass = () => setRows((old) => [...old, { name: '', sections: 1, tuition: '0', exam: '0', other: '0' }]);
  const removeClass = (index) => setRows((old) => old.filter((_, i) => i !== index));
  const saveStructure = () =>
    structure.save({ classes: rows.map((r) => ({ id: r.id, name: r.name, sections: Number(r.sections) })) });
  const saveFees = async (confirmChange = false) => {
    const input = {
      plans: rows.filter((r) => r.id).map((r) => ({ classId: r.id, tuition: r.tuition, exam: r.exam, other: r.other })),
      terms: Number(terms),
      transportFeePerTerm: transport,
      confirm: confirmChange,
    };
    try {
      await feeMutation.run(input);
      toast(t('settings.feesSaved'), { kind: 'ok' });
      await reload();
    } catch (error) {
      if (
        error.kind === 'conflict' &&
        !confirmChange &&
        (await confirm({ title: t('settings.termsTitle'), message: t('settings.termsWarning') }))
      )
        await saveFees(true);
    }
  };
  return (
    <section className="card cb stack">
      <h2>{t('settings.classesFees')}</h2>
      <p className="sm mut">{t('settings.perTerm')}</p>
      <div className="scroll-x">
        <table className="tbl">
          <thead>
            <tr>
              <th>{t('students.class')}</th>
              <th>{t('settings.sections')}</th>
              <th>{t('settings.tuition')}</th>
              <th>{t('settings.exam')}</th>
              <th>{t('settings.other')}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <tr key={row.id || `new-${index}`}>
                <td>
                  <input className="inp" value={row.name} onChange={(e) => change(index, 'name', e.target.value)} />
                </td>
                <td>
                  <input
                    className="inp num-in"
                    inputMode="numeric"
                    value={row.sections}
                    onChange={(e) => change(index, 'sections', e.target.value)}
                  />
                </td>
                {['tuition', 'exam', 'other'].map((field) => (
                  <td key={field}>
                    <input
                      className="inp num-in"
                      inputMode="numeric"
                      value={row[field]}
                      onChange={(e) => change(index, field, e.target.value)}
                    />
                  </td>
                ))}
                <td>
                  <Button kind="quiet small" onClick={() => removeClass(index)}>
                    {t('common.remove')}
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="row g8">
        <Button kind="outline small" onClick={addClass}>
          {t('settings.addClass')}
        </Button>
      </div>
      {structure.mutation.error && <div className="err">{structure.mutation.error.message}</div>}
      <Button onClick={saveStructure}>{t('settings.saveClasses')}</Button>
      <div className="fgrid">
        <Field as="select" label={t('settings.terms')} value={terms} onChange={(e) => setTerms(e.target.value)}>
          {TERMS.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </Field>
        <Field label={t('settings.busFee')} value={transport} onChange={(e) => setTransport(e.target.value)} />
      </div>
      {feeMutation.error && !feeMutation.error.field && <div className="err">{feeMutation.error.message}</div>}
      <Button onClick={() => saveFees(false)} disabled={feeMutation.pending}>
        {t('settings.saveFees')}
      </Button>
    </section>
  );
}

function SubjectSettings({ classes, reload }) {
  const t = useT();
  const [byClass, setByClass] = useState(
    Object.fromEntries(classes.map((c) => [c.id, c.subjects.map((s) => s.name).join(', ')])),
  );
  const { mutation, save } = useSectionSave(commands.saveSubjects, 'settings.subjectsSaved', reload);
  return (
    <section className="card cb stack">
      <h2>{t('settings.subjects')}</h2>
      {classes.map((c) => (
        <div className="stack" key={c.id}>
          <label className="sm b">{c.name}</label>
          <div className="row g8">
            <input
              className="inp grow"
              aria-label={`${c.name} ${t('settings.subjects')}`}
              value={byClass[c.id] ?? ''}
              onChange={(e) => setByClass((old) => ({ ...old, [c.id]: e.target.value }))}
            />
            <Button
              kind="outline small"
              onClick={() =>
                save({
                  classId: c.id,
                  subjects: (byClass[c.id] || '')
                    .split(',')
                    .map((x) => x.trim())
                    .filter(Boolean),
                })
              }
            >
              {t('common.save')}
            </Button>
          </div>
        </div>
      ))}
      {mutation.error && <div className="err">{mutation.error.message}</div>}
    </section>
  );
}

function ExamSettings({ exams, reload }) {
  const t = useT();
  const [rows, setRows] = useState(exams.map((e) => ({ id: e.id, name: e.name, max: String(e.maxMarks) })));
  const { mutation, save } = useSectionSave(commands.saveExams, 'settings.examsSaved', reload);
  const change = (index, field, value) =>
    setRows((old) => old.map((r, i) => (i === index ? { ...r, [field]: value } : r)));
  return (
    <section className="card cb stack">
      <h2>{t('settings.exams')}</h2>
      {rows.map((exam, index) => (
        <div className="row g10" key={exam.id || `new-${index}`}>
          <input
            className="inp grow"
            aria-label={t('settings.exam')}
            value={exam.name}
            onChange={(e) => change(index, 'name', e.target.value)}
          />
          <input
            className="inp num-in"
            aria-label={t('settings.maxMarks')}
            inputMode="numeric"
            value={exam.max}
            onChange={(e) => change(index, 'max', e.target.value)}
          />
          <Button kind="quiet small" onClick={() => setRows((old) => old.filter((_, i) => i !== index))}>
            {t('common.remove')}
          </Button>
        </div>
      ))}
      <div className="row g8">
        <Button
          kind="outline small"
          onClick={() => setRows((old) => [...old, { name: '', max: '100' }])}
        >
          {t('settings.addExam')}
        </Button>
      </div>
      {mutation.error && <div className="err">{mutation.error.message}</div>}
      <Button onClick={() => save({ exams: rows.map((r) => ({ id: r.id, name: r.name, max: Number(r.max) })) })}>
        {t('settings.saveExams')}
      </Button>
    </section>
  );
}

function GradeScaleSettings({ gradeScale, reload }) {
  const t = useT();
  const [rows, setRows] = useState(gradeScale.map((g) => ({ grade: g.grade, minPercent: String(g.minPercent) })));
  const { mutation, save } = useSectionSave(commands.saveGradeScale, 'settings.gradesSaved', reload);
  const change = (index, field, value) =>
    setRows((old) => old.map((r, i) => (i === index ? { ...r, [field]: value } : r)));
  return (
    <section className="card cb stack">
      <h2>{t('settings.gradeScale')}</h2>
      {rows.map((band, index) => (
        <div className="row g10" key={index}>
          <input
            className="inp"
            aria-label={t('report.grade')}
            value={band.grade}
            onChange={(e) => change(index, 'grade', e.target.value)}
          />
          <input
            className="inp num-in"
            aria-label={t('settings.minPercent')}
            inputMode="numeric"
            value={band.minPercent}
            onChange={(e) => change(index, 'minPercent', e.target.value)}
          />
        </div>
      ))}
      {mutation.error && <div className="err">{mutation.error.message}</div>}
      <Button
        onClick={() =>
          save({ bands: rows.map((r) => ({ grade: r.grade, minPercent: Number(r.minPercent) })) })
        }
      >
        {t('common.save')}
      </Button>
    </section>
  );
}

function DeviceSettings({ appSettings, reload }) {
  const t = useT();
  const [code, setCode] = useState('');
  const [timeout, setTimeoutValue] = useState(appSettings.session_timeout_minutes || '30');
  const device = useSectionSave(commands.saveDeviceCode, 'settings.deviceSaved', reload);
  const app = useSectionSave(commands.saveAppSettings, 'settings.appSaved', reload);
  return (
    <section className="card cb stack">
      <h2>{t('settings.deviceLicense')}</h2>
      <div className="fgrid">
        <Field
          label={t('settings.receiptPrefix')}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          error={device.mutation.fieldError('code')}
        />
        <Field
          label={t('settings.sessionTimeout')}
          inputMode="numeric"
          value={timeout}
          onChange={(e) => setTimeoutValue(e.target.value)}
          error={app.mutation.fieldError('sessionTimeoutMinutes')}
        />
      </div>
      {device.mutation.error && !device.mutation.error.field && (
        <div className="err">{device.mutation.error.message}</div>
      )}
      <div className="row g8">
        <Button onClick={() => code && device.save({ code })}>{t('settings.saveDevice')}</Button>
        <Button
          kind="outline"
          onClick={() =>
            app.save({
              sessionTimeoutMinutes: Number(timeout),
              receiptPaper: appSettings.receipt_paper || 'A5',
              printLanguage: appSettings.print_language || 'en',
              tray: appSettings.tray === '1',
              startAtLogin: appSettings.start_at_login === '1',
              keepAwake: appSettings.keep_awake === '1',
              schoolStart: appSettings.school_start || '',
              schoolEnd: appSettings.school_end || '',
            })
          }
        >
          {t('settings.saveApp')}
        </Button>
      </div>
    </section>
  );
}

export function Settings() {
  const t = useT();
  const query = useQuery(commands.getSettings, []);
  const data = query.data;
  return (
    <section className="view">
      <div className="inner stack">
        <h1>{t('nav.settings')}</h1>
        {query.error && <div className="note n-red">{query.error.message}</div>}
        {data && (
          <div className="stack" key={data.school.name}>
            <SchoolSettings school={data.school} reload={query.reload} />
            <ClassSettings classes={data.classes} session={data.session} reload={query.reload} />
            <SubjectSettings classes={data.classes} reload={query.reload} />
            <ExamSettings exams={data.exams} reload={query.reload} />
            <GradeScaleSettings gradeScale={data.gradeScale} reload={query.reload} />
            <DeviceSettings appSettings={data.appSettings || {}} reload={query.reload} />
          </div>
        )}
      </div>
    </section>
  );
}
