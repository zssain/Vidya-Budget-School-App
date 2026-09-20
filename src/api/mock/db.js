// In-memory mock database and the helpers ported from the prototype.
//
// TEMPORARY (see docs/prompts/P1.2): business logic is allowed ONLY inside
// src/api/mock/, because Rust does not exist yet. It is deleted piece by piece
// from P2.7 to P4.4. Nothing outside src/api/mock/ may contain business rules.

export const APP_VERSION = '0.1.0';
export const PW_ITER = 120000;
export const BK_ITER = 200000;
export const MAX_FAILED = 5;

// ---- Mutable state ----
export const state = {
  DB: null, // the whole school database
  DIRTY: false, // changes since the last backup file was saved
  session: { user: null, pending: null },
  _paid: null, // cache: adm -> amount paid
};

// ---- Errors (AppError-shaped plain objects; api/errors.toAppError wraps them) ----
export function permissionError() {
  return {
    kind: 'permission',
    messageKey: 'permission.denied',
    message: 'You do not have permission to do this.',
  };
}
export function userError(message, field) {
  return { kind: 'validation', messageKey: 'errors.generic', message, field };
}
export function authError(message) {
  return { kind: 'auth', messageKey: 'errors.generic', message };
}
export function notFound(message) {
  return { kind: 'not_found', messageKey: 'errors.generic', message: message || 'Not found.' };
}

// ---- Small helpers ----
export const pad = (n, w = 2) => String(n).padStart(w, '0');
export const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
export const dateKey = (d) => d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
export const todayKey = () => dateKey(new Date());
export const nowISO = () => new Date().toISOString();
export const rs = (n) => '₹' + Math.round(Number(n) || 0).toLocaleString('en-IN');
export const uid = (p) =>
  p +
  '_' +
  Array.from(crypto.getRandomValues(new Uint8Array(6)), (b) => b.toString(16).padStart(2, '0')).join('');
export const initials = (n) =>
  String(n || '?')
    .trim()
    .split(/\s+/)
    .map((w) => w[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();
export const sum = (arr, f) => arr.reduce((a, x) => a + (f ? f(x) : x), 0);

// ---- Crypto (Web Crypto API) ----
const enc = new TextEncoder();
const dec = new TextDecoder();
export function b64(buf) {
  const b = new Uint8Array(buf);
  let s = '';
  for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
  return btoa(s);
}
export const unb64 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
export const randBytes = (n) => crypto.getRandomValues(new Uint8Array(n));
async function pbkdf2(pass, salt, iter) {
  const k = await crypto.subtle.importKey('raw', enc.encode(pass), 'PBKDF2', false, ['deriveBits']);
  return crypto.subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt, iterations: iter }, k, 256);
}
export async function hashPassword(pass) {
  const salt = randBytes(16);
  return { salt: b64(salt), hash: b64(await pbkdf2(pass, salt, PW_ITER)), iter: PW_ITER };
}
export async function verifyPassword(pass, rec) {
  const h = b64(await pbkdf2(pass, unb64(rec.salt), rec.iter));
  if (h.length !== rec.hash.length) return false;
  let diff = 0;
  for (let i = 0; i < h.length; i++) diff |= h.charCodeAt(i) ^ rec.hash.charCodeAt(i);
  return diff === 0;
}
export async function aesKeyFrom(pass, salt, iter) {
  const bits = await pbkdf2(pass, salt, iter);
  return crypto.subtle.importKey('raw', bits, 'AES-GCM', false, ['encrypt', 'decrypt']);
}
export { enc, dec };
export function tempPassword() {
  const L = 'abcdefghjkmnpqrstuvwxyz';
  const D = '23456789';
  const r = randBytes(10);
  let p = '';
  for (let i = 0; i < 4; i++) p += L[r[i] % L.length];
  p += '-';
  for (let i = 4; i < 8; i++) p += D[r[i] % D.length];
  return p;
}
export function genDeviceId() {
  const A = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  const r = randBytes(12);
  let s = '';
  for (let i = 0; i < 12; i++) {
    s += A[r[i] % A.length];
    if (i === 3 || i === 7) s += '-';
  }
  return 'VD-' + s;
}

// ---- Amount in words (English, Indian system) ----
export function inWords(n) {
  n = Math.round(n);
  const a = [
    '',
    'One',
    'Two',
    'Three',
    'Four',
    'Five',
    'Six',
    'Seven',
    'Eight',
    'Nine',
    'Ten',
    'Eleven',
    'Twelve',
    'Thirteen',
    'Fourteen',
    'Fifteen',
    'Sixteen',
    'Seventeen',
    'Eighteen',
    'Nineteen',
  ];
  const b = ['', '', 'Twenty', 'Thirty', 'Forty', 'Fifty', 'Sixty', 'Seventy', 'Eighty', 'Ninety'];
  const two = (x) => (x < 20 ? a[x] : b[Math.floor(x / 10)] + (x % 10 ? ' ' + a[x % 10] : ''));
  const three = (x) =>
    (x >= 100 ? a[Math.floor(x / 100)] + ' Hundred' + (x % 100 ? ' ' : '') : '') +
    (x % 100 ? two(x % 100) : '');
  if (n === 0) return 'Zero';
  let s = '';
  const cr = Math.floor(n / 1e7);
  n %= 1e7;
  const l = Math.floor(n / 1e5);
  n %= 1e5;
  const th = Math.floor(n / 1e3);
  n %= 1e3;
  if (cr) s += inWords(cr) + ' Crore ';
  if (l) s += two(l) + ' Lakh ';
  if (th) s += two(th) + ' Thousand ';
  if (n) s += three(n);
  return s.trim();
}

// ---- Change log ----
export function commit(kind, text) {
  const DB = state.DB;
  DB.meta.seq++;
  DB.changes.unshift({
    seq: DB.meta.seq,
    at: nowISO(),
    user: state.session.user ? state.session.user.id : null,
    who: state.session.user ? state.session.user.name : 'System',
    device: DB.meta.deviceCode,
    kind,
    text,
  });
  if (DB.changes.length > 3000) DB.changes.length = 3000;
  state.DIRTY = true;
  state._paid = null;
}

// ---- Permissions (checked on every save, not just hidden buttons) ----
export const ROLE_LABEL = { principal: 'Principal', accountant: 'Accountant', teacher: 'Teacher' };
const ROLE_CAN = {
  accountant: new Set([
    'students.view',
    'students.add',
    'students.edit',
    'fees.view',
    'fees.collect',
    'daybook.view',
    'backup.run',
  ]),
  teacher: new Set(['students.view', 'attendance.mark', 'marks.enter', 'reportcard.view']),
};
const OWN_SCOPED = new Set(['attendance.mark', 'marks.enter', 'reportcard.view', 'students.view']);
export function can(action, ck) {
  const u = state.session.user;
  if (!u || !u.active) return false;
  if (u.role === 'principal') return true;
  const set = ROLE_CAN[u.role];
  if (!set || !set.has(action)) return false;
  if (u.role === 'teacher' && OWN_SCOPED.has(action) && ck !== undefined)
    return (u.classes || []).includes(ck);
  return true;
}
export function need(action, ck) {
  if (!can(action, ck)) throw permissionError();
}

// ---- School data helpers ----
export const CODE = () => state.DB.license.schoolCode;
export const fullUser = (u) => u.username + '@' + CODE();
export const allCK = () => state.DB.classes.flatMap((c) => c.sections.map((s) => c.name + '-' + s));
export const splitCK = (ck) => {
  const i = ck.lastIndexOf('-');
  return [ck.slice(0, i), ck.slice(i + 1)];
};
export const visibleCK = () =>
  state.session.user.role === 'teacher'
    ? allCK().filter((ck) => (state.session.user.classes || []).includes(ck))
    : allCK();
export const activeStudents = () => state.DB.students.filter((s) => s.status === 'active');
export const ckOf = (s) => s.cls + '-' + s.sec;
export const inCK = (ck) => {
  const [c, s] = splitCK(ck);
  return activeStudents()
    .filter((x) => x.cls === c && x.sec === s)
    .sort((a, b) => a.roll - b.roll);
};
export const stuBy = (adm) => state.DB.students.find((s) => s.adm === adm);
export const userBy = (id) => state.DB.users.find((u) => u.id === id);
export const subjectsFor = (cls) => state.DB.subjects[cls] || [];
export const teachersOf = (ck) =>
  state.DB.users.filter((u) => u.role === 'teacher' && u.active && (u.classes || []).includes(ck));

export function termFee(cls) {
  const f = state.DB.fees[cls] || { tuition: 0, exam: 0, other: 0 };
  return (+f.tuition || 0) + (+f.exam || 0) + (+f.other || 0);
}
export function feeDue(s) {
  if (s.rte) return 0;
  const per = termFee(s.cls) + (s.transport ? +state.DB.transportFee || 0 : 0);
  return Math.max(0, per * state.DB.terms - (+s.concession || 0));
}
export function paidOf(adm) {
  if (!state._paid) {
    state._paid = {};
    for (const r of state.DB.receipts)
      if (!r.cancelled) state._paid[r.adm] = (state._paid[r.adm] || 0) + r.amount;
  }
  return state._paid[adm] || 0;
}
export const balanceOf = (s) => Math.max(0, feeDue(s) - paidOf(s.adm));
export function feeState(s) {
  if (s.rte) return 'rte';
  const d = feeDue(s);
  const p = paidOf(s.adm);
  if (p >= d) return 'paid';
  return p > 0 ? 'part' : 'due';
}
export const gradeOf = (pct) =>
  pct == null ? '—' : pct >= 80 ? 'A' : pct >= 65 ? 'B' : pct >= 50 ? 'C' : pct >= 33 ? 'D' : 'E';

export function attendanceOf(adm, fromKey, toKey) {
  let p = 0;
  let t = 0;
  for (const [d, byCk] of Object.entries(state.DB.attendance)) {
    if ((fromKey && d < fromKey) || (toKey && d > toKey)) continue;
    for (const rec of Object.values(byCk)) {
      const m = rec.marks[adm];
      if (m) {
        t++;
        if (m === 'P') p++;
      }
    }
  }
  return { p, t, pct: t ? Math.round((p / t) * 100) : null };
}

export function examResult(s, ex) {
  const m = (state.DB.marks[ex.id] || {})[s.adm] || {};
  const subs = subjectsFor(s.cls);
  let got = 0;
  let max = 0;
  let entered = 0;
  subs.forEach((sub) => {
    const v = m[sub];
    if (v === undefined) return;
    entered++;
    max += +ex.max;
    if (v !== 'AB') got += +v;
  });
  const pct = max ? Math.round((got / max) * 1000) / 10 : null;
  return { got, max, entered, pct, grade: gradeOf(pct) };
}

// ---- Setup constants ----
export const DEFAULT_CLASSES = [
  'Nursery',
  'LKG',
  'UKG',
  'I',
  'II',
  'III',
  'IV',
  'V',
  'VI',
  'VII',
  'VIII',
  'IX',
  'X',
];
export const DEFAULT_FEES = {
  Nursery: [1800, 200, 300],
  LKG: [1800, 200, 300],
  UKG: [1950, 200, 300],
  I: [2100, 250, 350],
  II: [2100, 250, 350],
  III: [2250, 250, 350],
  IV: [2250, 300, 400],
  V: [2400, 300, 400],
  VI: [2700, 350, 450],
  VII: [2700, 350, 450],
  VIII: [2900, 400, 500],
  IX: [3200, 450, 550],
  X: [3200, 450, 550],
};
export function defaultSubjects(cls) {
  if (['Nursery', 'LKG', 'UKG'].includes(cls)) return ['Hindi', 'English', 'Numbers', 'Drawing'];
  if (['VI', 'VII', 'VIII'].includes(cls))
    return ['Hindi', 'English', 'Mathematics', 'Science', 'Social Science', 'Computer'];
  if (['IX', 'X', 'XI', 'XII'].includes(cls))
    return ['Hindi', 'English', 'Mathematics', 'Science', 'Social Science'];
  return ['Hindi', 'English', 'Mathematics', 'EVS', 'Drawing'];
}
export const DEFAULT_EXAMS = () => [
  { id: 'ut1', name: 'Unit Test 1', max: 25 },
  { id: 'hy', name: 'Half Yearly', max: 100 },
  { id: 'ut2', name: 'Unit Test 2', max: 25 },
  { id: 'an', name: 'Annual', max: 100 },
];
export function defaultSession() {
  const d = new Date();
  const y = d.getMonth() >= 3 ? d.getFullYear() : d.getFullYear() - 1;
  return y + '-' + pad((y + 1) % 100);
}
export function slugName(name, fallback) {
  const f = (String(name).trim().split(/\s+/)[0] || '').toLowerCase().replace(/[^a-z0-9]/g, '');
  return f || fallback;
}
export function uniqueUsername(name, taken, fallback) {
  const base = slugName(name, fallback);
  let u = base;
  let n = 2;
  while (taken.has(u)) u = base + n++;
  taken.add(u);
  return u;
}
export function blankDB(deviceId, schoolCode, activationCode) {
  return {
    meta: {
      app: 'vidya',
      schema: 1,
      createdAt: nowISO(),
      deviceId,
      deviceCode: 'PC',
      seq: 0,
      lastBackupAt: null,
      lastBackupSeq: 0,
    },
    license: { schoolCode, activationCode, activatedAt: nowISO(), maxUsers: 40, maxDevices: 5, type: 'demo' },
    school: {},
    classes: [],
    fees: {},
    transportFee: 0,
    terms: 3,
    subjects: {},
    exams: DEFAULT_EXAMS(),
    users: [],
    students: [],
    counters: { adm: 1, rcpt: {} },
    attendance: {},
    marks: {},
    marksMeta: {},
    receipts: [],
    changes: [],
    backupCheck: null,
  };
}
export function backupOverdue() {
  return !state.DB.meta.lastBackupAt || Date.now() - new Date(state.DB.meta.lastBackupAt) > 7 * 86400e3;
}
