// Sign in, first password, account. Ported from the prototype.
import * as db from './db.js';

const PERMS = {
  principal: [
    'students.view',
    'students.add',
    'students.edit',
    'students.mark_left',
    'fees.view',
    'fees.collect',
    'fees.cancel_receipt',
    'daybook.view',
    'attendance.mark',
    'marks.enter',
    'reportcard.view',
    'reports.view',
    'users.manage',
    'activity.view',
    'backup.manage',
    'backup.run',
    'settings.edit',
  ],
  accountant: [
    'students.view',
    'students.add',
    'students.edit',
    'fees.view',
    'fees.collect',
    'daybook.view',
    'backup.run',
  ],
  teacher: ['students.view', 'attendance.mark', 'marks.enter', 'reportcard.view'],
};

function userDto(u) {
  return {
    id: u.id,
    name: u.name,
    username: db.fullUser(u),
    role: u.role,
    sections: u.classes || [],
    language: u.language || 'en',
    permissions: PERMS[u.role] || [],
    // Mock convenience for forms (the class/section structure). The real
    // CurrentUserDto does not carry this; forms will use a dedicated command.
    schoolClasses: db.state.DB.classes.map((c) => ({ name: c.name, sections: [...c.sections] })),
  };
}

function makeToken() {
  return db.b64(db.randBytes(24)).replace(/[+/=]/g, '');
}

export async function signIn({ username, password }) {
  const DB = db.state.DB;
  const raw = String(username || '').toLowerCase();
  const code = DB.license.schoolCode;
  const un = raw.endsWith('@' + code) ? raw.slice(0, -(code.length + 1)) : raw;
  if (!un || !password) throw db.userError('Enter your username and password.');
  if (raw.includes('@') && !raw.endsWith('@' + code))
    throw db.authError(`This computer belongs to school code @${code}.`);
  const u = DB.users.find((x) => x.username === un);
  if (!u || !u.active) throw db.authError('No active login with that username.');
  if (u.locked)
    throw db.authError('This login is locked after 5 wrong passwords. Ask the principal to unlock it.');
  if (u.lockedUntil && Date.now() < u.lockedUntil) {
    throw db.authError('Too many wrong passwords. Try again in a few minutes.');
  }
  const ok = await db.verifyPassword(password, u);
  if (!ok) {
    u.failed = (u.failed || 0) + 1;
    if (u.failed >= db.MAX_FAILED) {
      u.failed = 0;
      if (u.role === 'principal') {
        u.lockedUntil = Date.now() + 5 * 60e3;
        db.commit('auth', `Principal login paused for 5 minutes after ${db.MAX_FAILED} wrong passwords`);
        throw db.authError('Too many wrong passwords. Try again in 5 minutes.');
      }
      u.locked = true;
      db.commit('auth', `${u.name}'s login locked after ${db.MAX_FAILED} wrong passwords`);
      throw db.authError('This login is now locked. Ask the principal to unlock it.');
    }
    const left = db.MAX_FAILED - u.failed;
    throw db.authError(`Wrong password. ${left} ${left === 1 ? 'try' : 'tries'} left.`);
  }
  u.failed = 0;
  u.lockedUntil = null;
  if (u.mustChange) {
    db.state.session.pending = u;
    return { status: 'must_change_password', pendingToken: makeToken() };
  }
  u.lastLogin = db.nowISO();
  db.state.session.user = u;
  db.commit('auth', `${u.name} signed in`);
  return { status: 'ok', token: makeToken(), user: userDto(u) };
}

export async function setFirstPassword({ newPassword }) {
  const u = db.state.session.pending;
  if (!u) throw db.authError('Sign in again.');
  await applyNewPassword(u, newPassword, true);
  db.state.session.pending = null;
  db.state.session.user = u;
  u.lastLogin = db.nowISO();
  db.commit('auth', `${u.name} signed in for the first time and set a password`);
  return { token: makeToken(), user: userDto(u) };
}

export async function changePassword({ currentPassword, newPassword }) {
  const u = db.state.session.user;
  if (!u) throw db.authError('Sign in again.');
  if (!(await db.verifyPassword(currentPassword, u)))
    throw db.userError('The current password is wrong.', 'currentPassword');
  await applyNewPassword(u, newPassword, false);
  db.commit('auth', `${u.name} changed their password`);
  return {};
}

async function applyNewPassword(u, newPassword, forced) {
  if ((newPassword || '').length < 8) throw db.userError('Use at least 8 characters.', 'newPassword');
  if (newPassword.toLowerCase().includes(u.username))
    throw db.userError('Do not use your username inside the password.', 'newPassword');
  if (await db.verifyPassword(newPassword, u))
    throw db.userError('Choose a password different from the current one.', 'newPassword');
  Object.assign(u, await db.hashPassword(newPassword));
  u.mustChange = false;
  void forced;
}

export function signOut() {
  const u = db.state.session.user;
  if (u) db.commit('auth', `${u.name} signed out`);
  db.state.session.user = null;
  db.state.session.pending = null;
  return {};
}

export function currentUser() {
  const u = db.state.session.user;
  if (!u) throw db.authError('Sign in again.');
  return userDto(u);
}

export function setLanguage({ language }) {
  const u = db.state.session.user;
  if (u) u.language = language === 'hi' ? 'hi' : 'en';
  return {};
}
