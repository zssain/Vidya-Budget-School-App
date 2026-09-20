import { useState } from 'react';
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
  const [sections, setSections] = useState((user?.classes || []).join(', '));
  const mutation = useMutation(user ? commands.updateUser : commands.createUser);
  const submit = async (event) => {
    event.preventDefault();
    try {
      const result = await mutation.run({
        userId: user?.id,
        name,
        mobile,
        role,
        sectionIds: sections
          .split(',')
          .map((value) => value.trim())
          .filter(Boolean),
      });
      onDone(result, user ? null : result);
    } catch {
      // The mutation hook exposes validation errors in the form.
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
      <Field label={t('users.classes')} value={sections} onChange={(e) => setSections(e.target.value)} />
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
    if (
      user.active &&
      !(await confirm({
        title: t('users.switchOffTitle'),
        message: t('users.switchOffMessage'),
        confirmLabel: t('users.switchOff'),
        danger: true,
      }))
    )
      return;
    await commands.setUserActive({ userId: user.id, active: !user.active });
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
          {query.data?.map((user) => (
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
                  {user.classes.length ? ` · ${user.classes.join(', ')}` : ''}
                </div>
                <div className="xs mut">
                  {user.lastLogin
                    ? t('users.lastLogin', { at: formatDateTime(user.lastLogin) })
                    : t('users.neverSignedIn')}
                </div>
              </div>
              <Pill
                kind={user.status === 'active' ? 'p-green' : user.status === 'locked' ? 'p-red' : 'p-orange'}
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
                    {user.active ? t('users.switchOff') : t('users.switchOn')}
                  </Button>
                </>
              )}
              {user.locked && (
                <Button kind="outline small" onClick={() => unlock(user.id)}>
                  {t('users.unlock')}
                </Button>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
