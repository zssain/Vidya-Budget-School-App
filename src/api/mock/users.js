// Staff logins. Ported from the prototype (principal only).
import * as db from './db.js';

function userDto(u) {
  return {
    id: u.id,
    name: u.name,
    username: db.fullUser(u),
    role: u.role,
    mobile: u.mobile || '',
    classes: u.classes || [],
    active: u.active,
    locked: !!u.locked,
    mustChange: !!u.mustChange,
    lastLogin: u.lastLogin,
    createdAt: u.createdAt,
    status: !u.active ? 'off' : u.locked ? 'locked' : u.mustChange ? 'pending' : 'active',
  };
}

export function listUsers() {
  db.need('users.manage');
  const order = { principal: 0, accountant: 1, teacher: 2 };
  return [...db.state.DB.users]
    .sort((a, b) => order[a.role] - order[b.role] || a.name.localeCompare(b.name))
    .map(userDto);
}

function slip(u, temp) {
  return {
    name: u.name,
    role: u.role,
    username: db.fullUser(u),
    tempPassword: temp,
    classes: u.classes || [],
  };
}

export async function createUser({ name, mobile, role, sectionIds }) {
  db.need('users.manage');
  const clean = String(name || '')
    .replace(/\s+/g, ' ')
    .trim();
  if (clean.length < 2) throw db.userError('Enter the full name.', 'name');
  if (!['teacher', 'accountant'].includes(role))
    throw db.userError('Only teacher and accountant logins can be added.', 'role');
  const classes = role === 'teacher' ? db.allCK().filter((ck) => (sectionIds || []).includes(ck)) : [];
  if (role === 'teacher' && !classes.length) throw db.userError('Pick at least one class.', 'sectionIds');
  if (db.state.DB.users.filter((u) => u.active).length >= db.state.DB.license.maxUsers)
    throw db.userError(
      `This license allows ${db.state.DB.license.maxUsers} active logins. Switch off an old login first.`,
    );
  const temp = db.tempPassword();
  const taken = new Set(db.state.DB.users.map((u) => u.username));
  const u = {
    id: db.uid('u'),
    username: db.uniqueUsername(clean, taken, role),
    name: clean,
    role,
    mobile: mobile || '',
    classes,
    ...(await db.hashPassword(temp)),
    mustChange: true,
    failed: 0,
    locked: false,
    active: true,
    createdAt: db.nowISO(),
    lastLogin: null,
  };
  db.state.DB.users.push(u);
  db.commit('user', `Created ${db.ROLE_LABEL[u.role].toLowerCase()} login ${db.fullUser(u)} for ${u.name}`);
  return slip(u, temp);
}

export function updateUser({ userId, name, mobile, sectionIds }) {
  db.need('users.manage');
  const u = db.userBy(userId);
  if (!u) throw db.notFound('Login not found.');
  const clean = String(name || '')
    .replace(/\s+/g, ' ')
    .trim();
  if (clean.length < 2) throw db.userError('Enter the full name.', 'name');
  u.name = clean;
  u.mobile = mobile || '';
  if (u.role === 'teacher') {
    const classes = db.allCK().filter((ck) => (sectionIds || []).includes(ck));
    if (!classes.length) throw db.userError('Pick at least one class.', 'sectionIds');
    u.classes = classes;
  }
  db.commit(
    'user',
    `Updated login for ${u.name}${u.role === 'teacher' ? ': classes ' + u.classes.join(', ') : ''}`,
  );
  return userDto(u);
}

export async function resetUserPassword({ userId }) {
  db.need('users.manage');
  const u = db.userBy(userId);
  if (!u) throw db.notFound('Login not found.');
  if (u.role === 'principal') throw db.userError('The principal changes their own password from My account.');
  const temp = db.tempPassword();
  Object.assign(u, await db.hashPassword(temp), { mustChange: true, locked: false, failed: 0 });
  db.commit('user', `Reset password for ${u.name}`);
  return slip(u, temp);
}

export function unlockUser({ userId }) {
  db.need('users.manage');
  const u = db.userBy(userId);
  if (!u) throw db.notFound('Login not found.');
  u.locked = false;
  u.failed = 0;
  db.commit('user', `Unlocked login for ${u.name}`);
  return userDto(u);
}

export function setUserActive({ userId, active }) {
  db.need('users.manage');
  const u = db.userBy(userId);
  if (!u) throw db.notFound('Login not found.');
  if (u.role === 'principal') throw db.userError('The principal login cannot be switched off.');
  if (!active) {
    // switching off is always allowed
  } else if (db.state.DB.users.filter((x) => x.active).length >= db.state.DB.license.maxUsers) {
    throw db.userError('The license limit is reached.');
  }
  u.active = !!active;
  db.commit('user', `${u.active ? 'Switched on' : 'Switched off'} login for ${u.name}`);
  return userDto(u);
}
