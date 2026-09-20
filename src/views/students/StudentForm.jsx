import { useEffect, useMemo, useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { Field } from '../../components/Field.jsx';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useMutation } from '../../core/useCommand.js';
import { useConfirm, useToast } from '../../core/ui.jsx';
import * as commands from '../../api/commands.js';

const empty = {
  name: '',
  gender: 'Male',
  cls: '',
  sec: 'A',
  father: '',
  mother: '',
  mobile: '',
  dob: '',
  cat: 'General',
  village: '',
  concession: 0,
  status: 'active',
  leftOn: '',
  leftReason: '',
  rte: false,
  transport: false,
  aadhaar: false,
  apaar: false,
};
export function StudentForm({ studentId }) {
  const t = useT();
  const router = useRouter();
  const { user } = useCurrentUser();
  const { toast } = useToast();
  const confirm = useConfirm();
  const firstClass = user.schoolClasses?.[0];
  const [form, setForm] = useState(() => ({
    ...empty,
    cls: firstClass?.name || '',
    sec: firstClass?.sections[0] || 'A',
  }));
  const [loaded, setLoaded] = useState(!studentId);
  const mutation = useMutation(studentId ? commands.updateStudent : commands.addStudent);
  const classes = useMemo(() => user.schoolClasses || [], [user.schoolClasses]);
  const sections = useMemo(
    () => classes.find((item) => item.name === form.cls)?.sections || classes[0]?.sections || ['A'],
    [classes, form.cls],
  );
  useEffect(() => {
    if (studentId)
      commands.getStudent({ studentId }).then((student) => {
        setForm({ ...empty, ...student });
        setLoaded(true);
      });
  }, [studentId]);
  const change = (name, value) => setForm((old) => ({ ...old, [name]: value }));
  const save = async (event, confirmDuplicate = false) => {
    event.preventDefault();
    try {
      const result = await mutation.run({ ...form, studentId, confirmDuplicate });
      toast(t(studentId ? 'students.saved' : 'students.added'), { kind: 'ok' });
      router.go('student', { studentId: result.adm || result.id });
    } catch (error) {
      if (
        error.kind === 'conflict' &&
        !confirmDuplicate &&
        (await confirm({
          title: t('students.possibleDuplicate'),
          message: error.message,
          confirmLabel: t('students.addAnyway'),
        }))
      )
        void save(event, true);
    }
  };
  if (!loaded) return null;
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t(studentId ? 'students.editTitle' : 'students.newAdmission')}</h1>
            <p className="mut">{studentId || t('students.admAuto')}</p>
          </div>
          <Button kind="quiet" onClick={router.back}>
            {t('common.back')}
          </Button>
        </div>
        <form className="card cb" onSubmit={save}>
          <div className="fgrid">
            <Field
              label={t('students.nameLabel')}
              value={form.name}
              onChange={(e) => change('name', e.target.value)}
              error={mutation.fieldError('name')}
            />
            <Field
              as="select"
              label={t('students.genderLabel')}
              value={form.gender}
              onChange={(e) => change('gender', e.target.value)}
            >
              {['Male', 'Female', 'Other'].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </Field>
            <Field
              as="select"
              label={t('students.classLabel')}
              value={form.cls}
              onChange={(e) => change('cls', e.target.value)}
              error={mutation.fieldError('cls')}
            >
              {classes.map((item) => (
                <option key={item.name}>{item.name}</option>
              ))}
            </Field>
            <Field
              as="select"
              label={t('students.sectionLabel')}
              value={form.sec}
              onChange={(e) => change('sec', e.target.value)}
            >
              {sections.map((value) => (
                <option key={value}>{value}</option>
              ))}
            </Field>
            <Field
              label={t('students.fatherLabel')}
              value={form.father}
              onChange={(e) => change('father', e.target.value)}
              error={mutation.fieldError('father')}
            />
            <Field
              label={t('students.motherLabel')}
              value={form.mother}
              onChange={(e) => change('mother', e.target.value)}
            />
            <Field
              label={t('students.mobileLabel')}
              inputMode="numeric"
              maxLength="10"
              value={form.mobile}
              onChange={(e) => change('mobile', e.target.value)}
              error={mutation.fieldError('mobile')}
            />
            <Field
              label={t('students.dobLabel')}
              type="date"
              value={form.dob}
              onChange={(e) => change('dob', e.target.value)}
              error={mutation.fieldError('dob')}
            />
            <Field
              as="select"
              label={t('students.categoryLabel')}
              value={form.cat}
              onChange={(e) => change('cat', e.target.value)}
            >
              {['General', 'OBC', 'SC', 'ST'].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </Field>
            <Field
              label={t('students.localityLabel')}
              value={form.village}
              onChange={(e) => change('village', e.target.value)}
            />
            {user.role === 'principal' && (
              <Field
                label={t('students.concessionLabel')}
                inputMode="numeric"
                value={form.concession}
                onChange={(e) => change('concession', e.target.value)}
                error={mutation.fieldError('concession')}
              />
            )}
          </div>
          {studentId && (
            <div className="fgrid">
              <Field
                as="select"
                label={t('students.statusLabel')}
                value={form.status}
                onChange={(e) => change('status', e.target.value)}
              >
                <option value="active">{t('students.studying')}</option>
                <option value="left">{t('students.left')}</option>
              </Field>
              {form.status === 'left' && (
                <>
                  <Field
                    label={t('students.leftOnLabel')}
                    type="date"
                    value={form.leftOn || ''}
                    onChange={(e) => change('leftOn', e.target.value)}
                  />
                  <Field
                    label={t('students.reasonLabel')}
                    value={form.leftReason || ''}
                    onChange={(e) => change('leftReason', e.target.value)}
                  />
                </>
              )}
            </div>
          )}
          <div className="row g16 wrap">
            {[
              ['rte', 'students.rteSeat'],
              ['transport', 'students.usesBus'],
              ['aadhaar', 'students.aadhaarCollected'],
              ['apaar', 'students.apaarCreated'],
            ].map(([name, key]) => (
              <label className="row g8" key={name}>
                <input
                  type="checkbox"
                  checked={form[name]}
                  onChange={(e) => change(name, e.target.checked)}
                />
                {t(key)}
              </label>
            ))}
          </div>
          {user.role !== 'principal' && <div className="note n-blue">{t('students.concessionNote')}</div>}
          {mutation.error && !mutation.error.field && (
            <div className="err" role="alert">
              {mutation.error.message}
            </div>
          )}
          <div className="row g8 end">
            <Button kind="outline" type="button" onClick={router.back}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={mutation.pending}>
              {t('common.save')}
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}
