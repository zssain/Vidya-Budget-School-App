import { useEffect, useMemo, useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { Field } from '../../components/Field.jsx';
import { ChipBar } from '../../components/ChipBar.jsx';
import { useT } from '../../core/i18n.jsx';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';

const classNames = ['Nursery', 'LKG', 'UKG', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];
const stepNames = ['activate', 'school', 'principal', 'classes', 'staff', 'backup', 'review'];
function initialState() {
  const year = new Date().getFullYear();
  return {
    deviceId: '',
    code: '',
    school: {
      name: '',
      addr: '',
      udise: '',
      board: 'State Board',
      session: `${year}-${String(year + 1).slice(-2)}`,
      phone: '',
    },
    principal: { name: '', mobile: '', pw: '', pw2: '' },
    classes: classNames.map((name) => ({
      name,
      on: !['IX', 'X'].includes(name),
      sections: name === 'Nursery' ? 1 : 2,
      tuition: 1200,
      exam: 200,
      other: 100,
    })),
    transportFee: 900,
    terms: 3,
    teachers: [{ name: '', mobile: '', classes: [] }],
    accountants: [{ name: '', mobile: '' }],
    backupPw: '',
    backupPw2: '',
    wroteDown: false,
  };
}
function setPath(old, path, value) {
  const next = structuredClone(old);
  const parts = path.split('.');
  let target = next;
  while (parts.length > 1) target = target[parts.shift()];
  target[parts[0]] = value;
  return next;
}
function WizardField({ state, setState, label, path, type = 'text' }) {
  const value = path.split('.').reduce((item, key) => item[key], state);
  return (
    <Field
      label={label}
      type={type}
      value={value}
      onChange={(event) => setState((old) => setPath(old, path, event.target.value))}
    />
  );
}

export function Wizard({ onBack, onComplete }) {
  const t = useT();
  const [step, setStep] = useState(0);
  const [state, setState] = useState(initialState);
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  useEffect(() => {
    commands
      .getDeviceId()
      .then(({ deviceId }) => setState((old) => ({ ...old, deviceId })))
      .catch((caught) => setError(toAppError(caught).message));
  }, []);
  const sections = useMemo(
    () =>
      state.classes
        .filter((item) => item.on)
        .flatMap((item) =>
          Array.from(
            { length: Number(item.sections) || 0 },
            (_, index) => `${item.name}-${String.fromCharCode(65 + index)}`,
          ),
        ),
    [state.classes],
  );
  const resizeStaff = (kind, count) =>
    setState((old) => {
      const next = structuredClone(old);
      const maximum = kind === 'teachers' ? 80 : 5;
      const size = Math.max(0, Math.min(maximum, Number(count) || 0));
      while (next[kind].length < size)
        next[kind].push(
          kind === 'teachers' ? { name: '', mobile: '', classes: [] } : { name: '', mobile: '' },
        );
      next[kind].length = size;
      return next;
    });
  const updateStaff = (kind, index, field, value) =>
    setState((old) => {
      const next = structuredClone(old);
      next[kind][index][field] = value;
      return next;
    });
  const updateClass = (index, field, value) =>
    setState((old) => {
      const next = structuredClone(old);
      next.classes[index][field] = value;
      return next;
    });
  const next = async () => {
    setError('');
    if (step < 6) {
      setStep(step + 1);
      return;
    }
    setPending(true);
    try {
      onComplete(await commands.wizardCreateSchool(state));
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setPending(false);
    }
  };
  let body;
  if (step === 0)
    body = (
      <>
        <p>{t('wizard.activateHelp')}</p>
        <Field label={t('wizard.deviceId')} value={state.deviceId} readOnly />
        <WizardField state={state} setState={setState} label={t('wizard.activationCode')} path="code" />
        <div className="note n-blue">{t('wizard.demoNote')}</div>
      </>
    );
  else if (step === 1)
    body = (
      <div className="fgrid">
        <WizardField state={state} setState={setState} label={t('settings.schoolName')} path="school.name" />
        <WizardField state={state} setState={setState} label={t('settings.address')} path="school.addr" />
        <WizardField state={state} setState={setState} label={t('settings.udise')} path="school.udise" />
        <WizardField state={state} setState={setState} label={t('settings.board')} path="school.board" />
        <WizardField state={state} setState={setState} label={t('settings.session')} path="school.session" />
        <WizardField state={state} setState={setState} label={t('settings.phone')} path="school.phone" />
      </div>
    );
  else if (step === 2)
    body = (
      <>
        <p>{t('wizard.principalHelp')}</p>
        <div className="fgrid">
          <WizardField state={state} setState={setState} label={t('wizard.fullName')} path="principal.name" />
          <WizardField
            state={state}
            setState={setState}
            label={t('settings.phone')}
            path="principal.mobile"
          />
          <WizardField
            state={state}
            setState={setState}
            label={t('wizard.password')}
            path="principal.pw"
            type="password"
          />
          <WizardField
            state={state}
            setState={setState}
            label={t('password.again')}
            path="principal.pw2"
            type="password"
          />
        </div>
      </>
    );
  else if (step === 3)
    body = (
      <>
        <p>{t('wizard.classesHelp')}</p>
        <div className="scroll-x">
          <table className="tbl">
            <thead>
              <tr>
                <th></th>
                <th>{t('students.class')}</th>
                <th>{t('settings.sections')}</th>
                <th>{t('settings.tuition')}</th>
                <th>{t('settings.exam')}</th>
                <th>{t('settings.other')}</th>
              </tr>
            </thead>
            <tbody>
              {state.classes.map((item, index) => (
                <tr key={item.name}>
                  <td>
                    <input
                      type="checkbox"
                      checked={item.on}
                      onChange={(event) => updateClass(index, 'on', event.target.checked)}
                    />
                  </td>
                  <td>
                    <b>{item.name}</b>
                  </td>
                  {['sections', 'tuition', 'exam', 'other'].map((field) => (
                    <td key={field}>
                      <input
                        className="inp num-in"
                        inputMode="numeric"
                        value={item[field]}
                        onChange={(event) => updateClass(index, field, event.target.value)}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="fgrid">
          <WizardField state={state} setState={setState} label={t('settings.busFee')} path="transportFee" />
          <WizardField state={state} setState={setState} label={t('settings.terms')} path="terms" />
        </div>
      </>
    );
  else if (step === 4)
    body = (
      <>
        <p>{t('wizard.staffHelp')}</p>
        {[
          ['teachers', 80],
          ['accountants', 5],
        ].map(([kind, max]) => (
          <section key={kind}>
            <div className="spread">
              <h3>{t(`wizard.${kind}`)}</h3>
              <Field
                label={t('wizard.howMany')}
                type="number"
                min="0"
                max={max}
                value={state[kind].length}
                onChange={(event) => resizeStaff(kind, event.target.value)}
              />
            </div>
            {state[kind].map((person, index) => (
              <div className="card cb" key={index}>
                <div className="fgrid">
                  <Field
                    label={t('users.name')}
                    value={person.name}
                    onChange={(event) => updateStaff(kind, index, 'name', event.target.value)}
                  />
                  <Field
                    label={t('users.mobile')}
                    value={person.mobile}
                    onChange={(event) => updateStaff(kind, index, 'mobile', event.target.value)}
                  />
                </div>
                {kind === 'teachers' && (
                  <ChipBar
                    items={sections.map((value) => ({ value, label: value }))}
                    value={person.classes[0]}
                    onChange={(value) =>
                      updateStaff(
                        kind,
                        index,
                        'classes',
                        person.classes.includes(value)
                          ? person.classes.filter((item) => item !== value)
                          : [...person.classes, value],
                      )
                    }
                  />
                )}
              </div>
            ))}
          </section>
        ))}
      </>
    );
  else if (step === 5)
    body = (
      <>
        <p>{t('wizard.backupHelp')}</p>
        <div className="fgrid">
          <WizardField
            state={state}
            setState={setState}
            label={t('wizard.backupPassword')}
            path="backupPw"
            type="password"
          />
          <WizardField
            state={state}
            setState={setState}
            label={t('password.again')}
            path="backupPw2"
            type="password"
          />
        </div>
        <div className="note n-orange">{t('wizard.backupNote')}</div>
        <label className="row g8">
          <input
            type="checkbox"
            checked={state.wroteDown}
            onChange={(event) => setState((old) => ({ ...old, wroteDown: event.target.checked }))}
          />
          {t('wizard.wroteDown')}
        </label>
      </>
    );
  else
    body = (
      <>
        <div className="pl">
          <span className="k">{t('wizard.school')}</span>
          <b>{state.school.name}</b>
        </div>
        <div className="pl">
          <span className="k">{t('wizard.classes')}</span>
          <span>{state.classes.filter((item) => item.on).length}</span>
        </div>
        <div className="pl">
          <span className="k">{t('wizard.teachers')}</span>
          <span>{state.teachers.length}</span>
        </div>
        <div className="pl">
          <span className="k">{t('wizard.accountants')}</span>
          <span>{state.accountants.length}</span>
        </div>
        <div className="note n-blue">{t('wizard.reviewNote')}</div>
      </>
    );
  return (
    <main className="wizard">
      <section className="wz">
        <div className="wz-h">
          <div className="xs mut b">{t('wizard.step', { n: step + 1, total: 7 })}</div>
          <h1>{t(`wizard.${stepNames[step]}`)}</h1>
          <div className="wz-bar">
            {stepNames.map((name, index) => (
              <i key={name} className={index < step ? 'done' : index === step ? 'on' : ''} />
            ))}
          </div>
        </div>
        <div className="wz-b">
          {body}
          {error && (
            <div className="err" role="alert">
              {error}
            </div>
          )}
        </div>
        <div className="wz-f">
          <Button kind="quiet" onClick={step === 0 ? onBack : () => setStep(step - 1)}>
            {step === 0 ? t('common.cancel') : t('common.back')}
          </Button>
          <Button disabled={pending} onClick={next}>
            {step === 6 ? t('wizard.create') : t('wizard.continue')}
          </Button>
        </div>
      </section>
    </main>
  );
}
