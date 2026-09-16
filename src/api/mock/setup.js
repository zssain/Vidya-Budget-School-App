// Setup: app status, device id, activation and school creation.
import * as db from './db.js';

let deviceId = null;

export function appStatus() {
  const DB = db.state.DB;
  return {
    hasSchool: !!DB,
    licensed: !!DB,
    deviceLocked: false,
    wizardStep: 0,
    platform: 'web',
    version: db.APP_VERSION,
    schoolName: DB ? DB.school.name : undefined,
    schoolCode: DB ? DB.license.schoolCode : undefined,
  };
}

export function getDeviceId() {
  if (!deviceId) deviceId = db.genDeviceId();
  return { deviceId };
}

/** Validate an activation code. Test build accepts DEMO-<schoolCode>. */
export function activate({ code }) {
  const m = /^DEMO-([A-Za-z0-9]{3,12})$/.exec(String(code || '').trim());
  if (!m) throw db.userError('That activation code is not valid. In this test build use DEMO-VAANI.', 'code');
  return { schoolCode: m[1].toLowerCase(), type: 'demo', maxUsers: 40, maxDevices: 5 };
}

/**
 * Create the school from the wizard payload. Validates everything (the view
 * only shows empty-field hints). Returns the principal username and the staff
 * credential slips (temporary passwords are returned only here).
 */
export async function wizardCreateSchool(payload) {
  const w = payload;
  const fail = (m, field) => {
    throw db.userError(m, field);
  };

  const code = /^DEMO-([A-Za-z0-9]{3,12})$/.exec(String(w.code || '').trim());
  if (!code) fail('That activation code is not valid. In this test build use DEMO-VAANI.', 'code');
  const schoolCode = code[1].toLowerCase();

  if (!String(w.school.name || '').trim()) fail('Enter the school name.', 'name');
  if (w.school.udise && !/^\d{11}$/.test(String(w.school.udise).trim()))
    fail('A UDISE code has exactly 11 digits.', 'udise');
  if (!String(w.principal.name || '').trim()) fail('Enter your full name.', 'principalName');
  if ((w.principal.pw || '').length < 8) fail('The password needs at least 8 characters.', 'pw');
  if (w.principal.pw !== w.principal.pw2) fail('The two passwords do not match.', 'pw2');

  const onClasses = w.classes.filter((c) => c.on);
  if (!onClasses.length) fail('Tick at least one class.', 'classes');
  for (const c of onClasses)
    for (const f of ['tuition', 'exam', 'other'])
      if (!/^\d+$/.test(String(c[f]).trim()))
        fail(`Fee for class ${c.name} must be a whole number of rupees.`, 'classes');
  if (!/^\d+$/.test(String(w.transportFee).trim()))
    fail('Bus fee must be a whole number of rupees.', 'transportFee');

  const cks = onClasses.flatMap((c) =>
    Array.from({ length: +c.sections }, (_, i) => c.name + '-' + String.fromCharCode(65 + i)),
  );
  const teachers = (w.teachers || []).map((tc) => ({
    ...tc,
    classes: (tc.classes || []).filter((ck) => cks.includes(ck)),
  }));
  teachers.forEach((tc, i) => {
    if (!String(tc.name || '').trim()) fail(`Enter a name for teacher ${i + 1}.`, 'teachers');
    if (!tc.classes.length) fail(`Pick at least one class for ${String(tc.name).trim()}.`, 'teachers');
  });
  const accountants = w.accountants || [];
  accountants.forEach((a, i) => {
    if (!String(a.name || '').trim()) fail(`Enter a name for accountant ${i + 1}.`, 'accountants');
  });
  if (1 + teachers.length + accountants.length > 40) fail('This license allows 40 logins in total.');

  if ((w.backupPw || '').length < 8) fail('The backup password needs at least 8 characters.', 'backupPw');
  if (w.backupPw !== w.backupPw2) fail('The two backup passwords do not match.', 'backupPw2');
  if (w.backupPw === w.principal.pw) fail('Use a different password from your login password.', 'backupPw');
  if (!w.wroteDown) fail('Tick the box to confirm you will write the password down.', 'wroteDown');

  const DB = db.blankDB(w.deviceId || db.genDeviceId(), schoolCode, String(w.code).trim().toUpperCase());
  DB.school = { ...w.school, name: w.school.name.trim() };
  DB.classes = onClasses.map((c) => ({
    name: c.name,
    sections: Array.from({ length: +c.sections }, (_, i) => String.fromCharCode(65 + i)),
  }));
  onClasses.forEach((c) => {
    DB.fees[c.name] = { tuition: +c.tuition, exam: +c.exam, other: +c.other };
    DB.subjects[c.name] = db.defaultSubjects(c.name);
  });
  DB.transportFee = +w.transportFee;
  DB.terms = w.terms;
  DB.backupCheck = await db.hashPassword(w.backupPw);

  const taken = new Set();
  const creds = [];
  const pUser = {
    id: db.uid('u'),
    username: db.uniqueUsername(w.principal.name, taken, 'principal'),
    name: w.principal.name.trim(),
    role: 'principal',
    mobile: (w.principal.mobile || '').trim(),
    classes: [],
    ...(await db.hashPassword(w.principal.pw)),
    mustChange: false,
    failed: 0,
    locked: false,
    active: true,
    createdAt: db.nowISO(),
    lastLogin: null,
  };
  DB.users.push(pUser);
  const staff = [
    ...teachers.map((x) => ({ ...x, role: 'teacher' })),
    ...accountants.map((x) => ({ ...x, role: 'accountant', classes: [] })),
  ];
  for (const s of staff) {
    const temp = db.tempPassword();
    const u = {
      id: db.uid('u'),
      username: db.uniqueUsername(s.name, taken, s.role),
      name: s.name.trim(),
      role: s.role,
      mobile: (s.mobile || '').trim(),
      classes: [...(s.classes || [])],
      ...(await db.hashPassword(temp)),
      mustChange: true,
      failed: 0,
      locked: false,
      active: true,
      createdAt: db.nowISO(),
      lastLogin: null,
    };
    DB.users.push(u);
    creds.push({
      name: u.name,
      role: u.role,
      username: u.username + '@' + schoolCode,
      tempPassword: temp,
      classes: u.classes,
    });
  }
  db.state.DB = DB;
  db.state.DIRTY = true;
  db.commit('setup', `School set up: ${DB.school.name}. ${staff.length} staff logins created.`);
  return { principalUsername: pUser.username + '@' + schoolCode, credentials: creds };
}
