import { useEffect, useMemo, useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { Field } from '../../components/Field.jsx';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useMutation, useQuery } from '../../core/useCommand.js';
import { useConfirm, useToast } from '../../core/ui.jsx';
import * as commands from '../../api/commands.js';

const empty = {
  name: '',
  gender: 'Male',
  sectionId: '',
  father: '',
  mother: '',
  mobile: '',
  dob: '',
  category: 'General',
  locality: '',
  concession: 0,
  status: 'active',
  leftOn: '',
  reason: '',
  rte: false,
  transport: false,
  aadhaarCollected: false,
  apaarCreated: false,
};

export function StudentForm({ studentId }) {
  const t = useT();
  const router = useRouter();
  const { can } = useCurrentUser();
  const { toast } = useToast();
  const confirm = useConfirm();
  const settings = useQuery(() => commands.getSettings(), []);
  const [form, setForm] = useState(empty);
  const [loaded, setLoaded] = useState(!studentId);
  const mutation = useMutation(studentId ? commands.updateStudent : commands.addStudent);
  const leaving = useMutation(commands.markStudentLeft);
  const sections = useMemo(() => {
    const classes = settings.data?.classes || [];
    return classes.flatMap((c) =>
      (c.sections || []).map((s) => ({ id: s.id, label: `${c.name}-${s.name}` })),
    );
  }, [settings.data]);
  useEffect(() => {
    if (studentId)
      commands.getStudent({ studentId }).then((student) => {
        setForm({ ...empty, ...student, leftOn: '', reason: '' });
        setLoaded(true);
      });
  }, [studentId]);
  const change = (name, value) => setForm((old) => ({ ...old, [name]: value }));
  const payload = () => ({
    name: form.name,
    gender: form.gender,
    dob: form.dob,
    father: form.father,
    mother: form.mother,
    mobile: form.mobile,
    category: form.category,
    locality: form.locality,
    sectionId: form.sectionId,
    rte: form.rte,
    transport: form.transport,
    concession: Number(form.concession) || 0,
    aadhaarCollected: form.aadhaarCollected,
    apaarCreated: form.apaarCreated,
  });
  const save = async (event, confirmDuplicate = false) => {
    event.preventDefault();
    if (studentId && form.status === 'left') {
      const result = await leaving.run({ studentId, leftOn: form.leftOn, reason: form.reason });
      toast(t('students.saved'), { kind: 'ok' });
      router.go('student', { studentId: result.id });
      return;
    }
    try {
      const input = studentId ? { ...payload(), studentId } : { ...payload(), confirmDuplicate };
      const result = await mutation.run(input);
      toast(t(studentId ? 'students.saved' : 'students.added'), { kind: 'ok' });
      router.go('student', { studentId: result.id });
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
  if (!loaded || settings.loading) return null;
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t(studentId ? 'students.editTitle' : 'students.newAdmission')}</h1>
            <p className="mut">{studentId ? form.admNo : t('students.admAuto')}</p>
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
              error={mutation.fieldError('gender')}
            >
              {['Male', 'Female', 'Other'].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </Field>
            <Field
              as="select"
              label={t('students.classSectionLabel')}
              value={form.sectionId}
              onChange={(e) => change('sectionId', e.target.value)}
              error={mutation.fieldError('sectionId')}
            >
              <option value="" disabled>
                {t('students.choose')}
              </option>
              {sections.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
                </option>
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
              error={mutation.fieldError('mother')}
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
              value={form.category}
              onChange={(e) => change('category', e.target.value)}
              error={mutation.fieldError('category')}
            >
              {['General', 'OBC', 'SC', 'ST'].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </Field>
            <Field
              label={t('students.localityLabel')}
              value={form.locality}
              onChange={(e) => change('locality', e.target.value)}
            />
            {can('students.set_concession') && (
              <Field
                label={t('students.concessionLabel')}
                inputMode="numeric"
                value={form.concession}
                onChange={(e) => change('concession', e.target.value)}
                error={mutation.fieldError('concession')}
              />
            )}
          </div>
          {studentId && can('students.mark_left') && (
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
                    value={form.leftOn}
                    onChange={(e) => change('leftOn', e.target.value)}
                    error={leaving.fieldError('leftOn')}
                  />
                  <Field
                    label={t('students.reasonLabel')}
                    value={form.reason}
                    onChange={(e) => change('reason', e.target.value)}
                    error={leaving.fieldError('reason')}
                  />
                </>
              )}
            </div>
          )}
          <div className="row g16 wrap">
            {[
              ['rte', 'students.rteSeat'],
              ['transport', 'students.usesBus'],
              ['aadhaarCollected', 'students.aadhaarCollected'],
              ['apaarCreated', 'students.apaarCreated'],
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
          {!can('students.set_concession') && (
            <div className="note n-blue">{t('students.concessionNote')}</div>
          )}
          {mutation.error && !mutation.error.field && (
            <div className="err" role="alert">
              {mutation.error.message}
            </div>
          )}
          {leaving.error && !leaving.error.field && (
            <div className="err" role="alert">
              {leaving.error.message}
            </div>
          )}
          <div className="row g8 end">
            <Button kind="outline" type="button" onClick={router.back}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={mutation.pending || leaving.pending}>
              {t('common.save')}
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}
