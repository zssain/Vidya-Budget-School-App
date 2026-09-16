import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/commands.js', () => ({
  appStatus: vi.fn(),
  signIn: vi.fn(),
  signOut: vi.fn(),
  setFirstPassword: vi.fn(),
  changePassword: vi.fn(),
  homePrincipal: vi.fn(),
  homeAccountant: vi.fn(),
  homeTeacher: vi.fn(),
  currentUser: vi.fn(),
  getStudent: vi.fn(),
  addStudent: vi.fn(),
  updateStudent: vi.fn(),
  getReportCard: vi.fn(),
  feeRegister: vi.fn(),
  getFeeAccount: vi.fn(),
  collectFee: vi.fn(),
  getReceipt: vi.fn(),
  cancelReceipt: vi.fn(),
  dayBook: vi.fn(),
  exportDuesXlsx: vi.fn(),
}));

import * as commands from '../api/commands.js';
import { view as homeView } from './home.js';
import { renderLogin } from './login.js';
import { renderPasswordModal } from './password.js';
import { view as detailView } from './students/detail.js';
import { openStudentForm } from './students/form.js';
import { openReportCard } from './reportcard.js';
import { view as feeView } from './fees/register.js';
import { openCollectPicker, openCollect } from './fees/collect.js';
import { showReceipt } from './fees/receipt.js';
import { openDayBook } from './fees/daybook.js';

const hostile = `Sierra D'Souza <b>x</b>`;
const school = { name: 'School', addr: '', udise: '', board: 'State Board', session: '2026-27', phone: '' };
const student = {
  adm: 'A1',
  roll: 1,
  name: hostile,
  ck: '1-A',
  father: 'Father',
  mother: '',
  mobile: '9876543210',
  dob: '2015-01-01',
  gender: 'Female',
  cat: 'General',
  village: '',
  admittedOn: '2026-04-01',
  status: 'active',
  rte: false,
  transport: false,
  concession: 0,
  terms: 3,
  termFee: 100,
  transportFee: 0,
  feeDue: 300,
  paid: 100,
  balance: 200,
  feeState: 'part',
  attendanceMonth: { p: 1, t: 1, pct: 100 },
  attendanceSession: { p: 1, t: 1, pct: 100 },
  exams: [],
};

const root = () => {
  const el = document.createElement('div');
  document.body.appendChild(el);
  return el;
};
const hasInjectedBold = (el = document) => [...el.querySelectorAll('b')].some((x) => x.textContent === 'x');

beforeEach(() => {
  vi.clearAllMocks();
  document.body.replaceChildren();
  commands.appStatus.mockResolvedValue({ schoolName: 'School', schoolCode: 'school' });
  commands.homePrincipal.mockResolvedValue({
    name: 'Principal',
    students: 1,
    classes: 1,
    sections: 1,
    pending: [],
    attendance: { done: false, p: 0, a: 0, l: 0 },
    fees: { due: 300, paid: 100, pending: 200, unpaid: 0, part: 1 },
    recentActivity: [],
    backupOverdue: true,
    lastBackupAt: null,
  });
  commands.currentUser.mockResolvedValue({
    role: 'principal',
    schoolClasses: [{ name: '1', sections: ['A'] }],
  });
  commands.getStudent.mockResolvedValue(student);
  commands.getReportCard.mockResolvedValue({
    school,
    student: { name: hostile, ck: '1-A', roll: 1, adm: 'A1', father: 'Father', dob: '2015-01-01' },
    exams: [{ id: 'e1', name: 'Test', max: 100 }],
    rows: [{ subject: 'English', byExam: [90] }],
    totals: [{ entered: true, got: 90, max: 100, pct: 90, grade: 'A' }],
    attendance: { p: 1, t: 1, pct: 100 },
  });
  commands.feeRegister.mockResolvedValue({
    session: '2026-27',
    terms: 3,
    devicePrefix: 'PC',
    totals: { due: 300, paid: 100, pending: 200, payingCount: 1, unpaid: 0, part: 1 },
    collectedToday: 0,
    receiptsToday: 0,
    list: [{ ...student, due: 300 }],
  });
  commands.getFeeAccount.mockResolvedValue({
    name: hostile,
    ck: '1-A',
    roll: 1,
    due: 300,
    paid: 100,
    balance: 200,
    oneTerm: 100,
    terms: 3,
    rte: false,
  });
  commands.getReceipt.mockResolvedValue({
    no: 'PC-0001',
    at: '2026-09-16T06:00:00Z',
    date: '2026-09-16',
    studentName: hostile,
    ck: '1-A',
    roll: 1,
    adm: 'A1',
    father: 'Father',
    mode: 'Cash',
    ref: '',
    byName: 'Principal',
    device: 'PC',
    balanceAfter: 100,
    amount: 100,
    amountWords: 'One Hundred',
    school,
    cancelled: null,
  });
  commands.dayBook.mockResolvedValue({
    date: '2026-09-16',
    modes: [{ mode: 'Cash', total: 100, count: 1 }],
    total: 100,
    count: 1,
    receipts: [
      {
        no: 'PC-0001',
        at: '2026-09-16T06:00:00Z',
        studentName: hostile,
        ck: '1-A',
        mode: 'Cash',
        ref: '',
        byName: 'Principal',
        amount: 100,
      },
    ],
    school,
  });
});

describe('previously ported views', () => {
  it('renders principal home', async () => {
    const el = root();
    await homeView(el, { me: { role: 'principal' }, refreshChrome: vi.fn() });
    expect(el.textContent).toContain('Principal');
  });
  it('renders login', async () => {
    const el = root();
    await renderLogin(el, { enterApp: vi.fn(), showPasswordFor: vi.fn() });
    expect(el.textContent).toContain('Sign in');
  });
  it('renders password modal', () => {
    renderPasswordModal();
    expect(document.body.textContent).toContain('Change password');
  });
  it('renders student detail and escapes its name', async () => {
    const el = root();
    await detailView(el, {
      adm: 'A1',
      me: { role: 'principal', sections: [], permissions: ['students.edit', 'fees.view', 'fees.collect'] },
    });
    expect(el.textContent).toContain(hostile);
    expect(hasInjectedBold(el)).toBe(false);
  });
  it('renders student form', async () => {
    await openStudentForm();
    expect(document.body.textContent).toContain('New admission');
  });
  it('renders report card and escapes its name', async () => {
    await openReportCard('A1');
    expect(document.body.textContent).toContain(hostile);
    expect(hasInjectedBold()).toBe(false);
  });
  it('renders fee register and escapes its name', async () => {
    const el = root();
    await feeView(el, {});
    expect(el.textContent).toContain(hostile);
    expect(hasInjectedBold(el)).toBe(false);
  });
  it('renders collect picker', async () => {
    await openCollectPicker();
    await Promise.resolve();
    expect(document.body.textContent).toContain('Find the student');
  });
  it('renders collect form and escapes its name', async () => {
    await openCollect('A1');
    expect(document.body.textContent).toContain(hostile);
    expect(hasInjectedBold()).toBe(false);
  });
  it('renders receipt and escapes its name', async () => {
    await showReceipt('PC-0001');
    expect(document.body.textContent).toContain(hostile);
    expect(hasInjectedBold()).toBe(false);
  });
  it('renders day book and escapes its name', async () => {
    await openDayBook('2026-09-16');
    expect(document.body.textContent).toContain(hostile);
    expect(hasInjectedBold()).toBe(false);
  });
  it('shows command validation errors safely', async () => {
    commands.getReceipt.mockRejectedValueOnce({ kind: 'validation', message: 'Receipt is invalid.' });
    await showReceipt('bad');
    expect(document.body.textContent).toContain('Receipt is invalid.');
  });
});
