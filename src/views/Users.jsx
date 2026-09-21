import { useMemo, useState } from 'react';
import { Button } from '../components/Button.jsx';
import { Field } from '../components/Field.jsx';
import { Pill } from '../components/Pill.jsx';
import { Slip } from '../components/Slip.jsx';
import { formatDateTime } from '../core/format.js';
import { useT } from '../core/i18n.jsx';
import { useMutation, useQuery } from '../core/useCommand.js';
import { useConfirm, useModal, useToast } from '../core/ui.jsx';
import * as commands from '../api/commands.js';

function UserForm({ user, onDone }) {
  const t = useT();
  const [name, setName] = useState(user?.name || '');
  const [mobile, setMobile] = useState(user?.mobile || '');
  const [role, setRole] = useState(user?.role || 'teacher');
  const settings = useQuery(commands.getSettings, []);
  const options = useMemo(
    () =>
      (settings.data?.classes || []).flatMap((c) =>
        c.sections.map((s) => ({ id: s.id, label: `${c.name}-${s.name}` })),
      ),
    [settings.data],
  );
  // Prefill the selection once the section options load (guarded render-time
  // update — the React-recommended pattern for state derived from async props).
  const [selected, setSelected] = useState(null);
  if (selected === null && options.length) {
    const labels = new Set(user?.sections || []);
    setSelected(new Set(options.filter((o) => labels.has(o.label)).map((o) => o.id)));
  }
  const chosen = selected ?? new Set();
  const mutation = useMutation(user ? commands.updateUser : commands.createUser);
  const toggleSection = (id) =>
    setSelected((prev) => {
      const next = new Set(prev ?? []);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const submit = async (event) => {
    event.preventDefault();
    try {
      const result = await mutation.run({
        userId: user?.id,
        name,
        mobile,
        role,
        sectionIds: [...chosen],
      });
      onDone(result, user ? null : result);
    } catch {
      // Validation errors are shown in the form.
    }
  };
  return (
    <form className="stack" onSubmit={submit}>
      <Field
        label={t('users.name')}
        value={name}
        onChange={(e) => setName(e.target.value)}
        error={mutation.fieldError('name')}
      />
      <Field
        label={t('users.mobile')}
        value={mobile}
        onChange={(e) => setMobile(e.target.value)}
        error={mutation.fieldError('mobile')}
      />
      {!user && (
        <Field as="select" label={t('users.role')} value={role} onChange={(e) => setRole(e.target.value)}>
          <option value="teacher">{t('roles.teacher')}</option>
          <option value="accountant">{t('roles.accountant')}</option>
        </Field>
      )}
      {role === 'teacher' && (
        <div className="stack">
          <label className="lbl">{t('users.classes')}</label>
          <div className="chips">
            {options.map((o) => (
              <label key={o.id} className={`chip ${chosen.has(o.id) ? 'on' : ''}`}>
                <input type="checkbox" checked={chosen.has(o.id)} onChange={() => toggleSection(o.id)} />
                {o.label}
              </label>
            ))}
          </div>
          {mutation.fieldError('sections') && <div className="err">{mutation.fieldError('sections')}</div>}
        </div>
      )}
      {mutation.error && !mutation.error.field && <div className="err">{mutation.error.message}</div>}
      <Button type="submit">{t('common.save')}</Button>
    </form>
  );
}

export function Users() {
  const t = useT();
  const query = useQuery(commands.listUsers, []);
  const { openModal, closeModal } = useModal();
  const { toast } = useToast();
  const confirm = useConfirm();
  const showSlip = (slip) =>
    openModal({
      title: t('users.credentials'),
      locked: true,
      body: (
        <>
          <div className="note n-orange">{t('users.once')}</div>
          <Slip credential={slip} />
        </>
      ),
      footer: <Button onClick={closeModal}>{t('common.done')}</Button>,
    });
  const edit = (user) =>
    openModal({
      title: user ? t('users.edit') : t('users.add'),
      body: (
        <UserForm
          user={user}
          onDone={async (_result, slip) => {
            closeModal();
            if (slip) showSlip(slip);
            else toast(t('users.saved'), { kind: 'ok' });
            await query.reload();
          }}
        />
      ),
    });
  const reset = async (userId) => showSlip(await commands.resetUserPassword({ userId }));
  const unlock = async (userId) => {
    await commands.unlockUser({ userId });
    toast(t('users.unlocked'), { kind: 'ok' });
    await query.reload();
  };
  const toggle = async (user) => {
    const isActive = user.status !== 'switched_off';
    if (
      isActive &&
      !(await confirm({
        title: t('users.switchOffTitle'),
        message: t('users.switchOffMessage'),
        confirmLabel: t('users.switchOff'),
        danger: true,
      }))
    )
      return;
    await commands.setUserActive({ userId: user.id, active: !isActive });
    await query.reload();
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t('nav.users')}</h1>
            <p className="mut">{t('users.subtitle')}</p>
          </div>
          <Button onClick={() => edit(null)}>{t('users.add')}</Button>
        </div>
        <div className="card">
          {query.data?.map((user) => {
            const isActive = user.status !== 'switched_off';
            return (
              <div className="userrow" key={user.id}>
                <div className="av">
                  {user.name
                    .split(/\s+/)
                    .map((word) => word[0])
                    .slice(0, 2)
                    .join('')}
                </div>
                <div className="grow">
                  <span className="b">{user.name}</span>
                  <div className="xs mut">
                    {user.username} · {t(`roles.${user.role}`)}
                    {user.sections?.length ? ` · ${user.sections.join(', ')}` : ''}
                  </div>
                  <div className="xs mut">
                    {user.lastLoginAt
                      ? t('users.lastLogin', { at: formatDateTime(user.lastLoginAt) })
                      : t('users.neverSignedIn')}
                  </div>
                </div>
                <Pill
                  kind={
                    user.status === 'active' ? 'p-green' : user.status === 'locked' ? 'p-red' : 'p-orange'
                  }
                >
                  {t(`users.status.${user.status}`)}
                </Pill>
                <Button kind="outline small" onClick={() => edit(user)}>
                  {t('common.edit')}
                </Button>
                {user.role !== 'principal' && (
                  <>
                    <Button kind="quiet small" onClick={() => reset(user.id)}>
                      {t('users.reset')}
                    </Button>
                    <Button kind="quiet small" onClick={() => toggle(user)}>
                      {isActive ? t('users.switchOff') : t('users.switchOn')}
                    </Button>
                  </>
                )}
                {user.status === 'locked' && (
                  <Button kind="outline small" onClick={() => unlock(user.id)}>
                    {t('users.unlock')}
                  </Button>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
