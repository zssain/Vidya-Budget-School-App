import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/commands.js', () => ({
  getAttendance: vi.fn(),
  saveAttendance: vi.fn(),
  attendanceRegister: vi.fn(),
  getMarksSheet: vi.fn(),
  saveMarks: vi.fn(),
  reportsSummary: vi.fn(),
  exportClassSummaryXlsx: vi.fn(),
  listActivity: vi.fn(),
  listUsers: vi.fn(),
  createUser: vi.fn(),
  updateUser: vi.fn(),
  resetUserPassword: vi.fn(),
  unlockUser: vi.fn(),
  setUserActive: vi.fn(),
  backupStatus: vi.fn(),
  backupSaveFile: vi.fn(),
  restoreInspect: vi.fn(),
  restoreCommit: vi.fn(),
  backupChangePassword: vi.fn(),
  getSettings: vi.fn(),
  saveSchool: vi.fn(),
  saveClasses: vi.fn(),
  saveExams: vi.fn(),
  saveDeviceCode: vi.fn(),
  getDeviceId: vi.fn(),
  wizardCreateSchool: vi.fn(),
  loadSampleSchool: vi.fn(),
}));

import * as commands from '../api/commands.js';
import { view as attendanceView } from './attendance.js';
import { view as marksView } from './marks.js';
import { view as reportsView } from './reports.js';
import { view as activityView } from './activity.js';
import { view as usersView } from './users.js';
import { view as backupView } from './backup.js';
import { view as settingsView } from './settings.js';
import { renderWelcome } from './setup/welcome.js';
import { renderWizard } from './setup/wizard.js';
import { renderCredentials } from './setup/credentials.js';

const hostile = `Sierra D'Souza <b>x</b>`;
const me = { role: 'principal', sections: ['1-A'], schoolClasses: [{ name: '1', sections: ['A'] }] };

function root() {
  const el = document.createElement('div');
  document.body.appendChild(el);
  return el;
}

beforeEach(() => {
  vi.clearAllMocks();
  document.body.replaceChildren();
  commands.getAttendance.mockResolvedValue({
    sectionId: '1-A',
    date: '2026-09-16',
    students: [{ adm: 'A1', roll: 1, name: hostile }],
    marks: {},
  });
  commands.getMarksSheet.mockResolvedValue({
    exam: { id: 'e1', name: 'Test', max: 100 },
    exams: [{ id: 'e1', name: 'Test' }],
    subjects: ['English'],
    students: [{ adm: 'A1', roll: 1, name: hostile, marks: {}, total: '—', grade: '—' }],
    editable: true,
  });
  commands.reportsSummary.mockResolvedValue({
    enrolment: 1,
    boys: 0,
    girls: 1,
    rte: 0,
    aadhaarPct: 100,
    aadhaarPending: 0,
    attendance30: 90,
    attendanceMarks: 1,
    byClass: [{ c: '1', n: 1, boys: 0, girls: 1, rte: 0, due: 100, col: 50 }],
    cats: [{ c: 'General', n: 1 }],
    feeStatus: [{ k: 'part', n: 1 }],
  });
  commands.listActivity.mockResolvedValue([
    { seq: 1, kind: 'stu', text: hostile, who: hostile, at: '2026-09-16T06:00:00Z', device: 'PC' },
  ]);
  commands.listUsers.mockResolvedValue([
    {
      id: 'u1',
      name: hostile,
      username: 'sierra@school',
      role: 'teacher',
      mobile: '',
      classes: ['1-A'],
      active: true,
      locked: false,
      status: 'active',
      lastLogin: null,
    },
  ]);
  commands.backupStatus.mockResolvedValue({ lastBackupAt: null, changesSince: 2, overdue: true });
  commands.getSettings.mockResolvedValue({
    school: { name: hostile, addr: '', udise: '', board: 'State Board', session: '2026-27', phone: '' },
    classes: [
      { name: '1', sections: 1, tuition: 100, exam: 10, other: 0, termFee: 110, subjects: ['English'] },
    ],
    transportFee: 0,
    terms: 3,
    exams: [{ id: 'e1', name: 'Test', max: 100 }],
    license: { schoolCode: 'school', activationCode: 'DEMO-SCHOOL', maxUsers: 40 },
    deviceId: 'VD-TEST',
    deviceCode: 'PC',
  });
  commands.getDeviceId.mockResolvedValue({ deviceId: 'VD-TEST' });
});

describe('remaining feature views', () => {
  it.each([
    ['attendance', attendanceView, () => ({ me })],
    ['marks', marksView, () => ({ me })],
    ['reports', reportsView, () => undefined],
    ['activity', activityView, () => undefined],
    ['users', usersView, () => undefined],
    ['backup', backupView, () => undefined],
    ['settings', settingsView, () => ({ me, refreshChrome: vi.fn() })],
  ])('%s renders safely', async (_name, view, params) => {
    const el = root();
    await view(el, params());
    expect(el.textContent.length).toBeGreaterThan(0);
    expect([...el.querySelectorAll('b')].some((x) => x.textContent === 'x')).toBe(false);
  });

  it('surfaces a validation rejection to the shell boundary', async () => {
    commands.getAttendance.mockRejectedValueOnce({ kind: 'validation', message: 'Choose a class.' });
    await expect(attendanceView(root(), { me })).rejects.toMatchObject({ message: 'Choose a class.' });
  });
});

describe('setup views', () => {
  const ctx = { showLogin: vi.fn(), showSetup: vi.fn() };

  it('renders welcome', () => {
    const el = root();
    renderWelcome(el, ctx);
    expect(el.textContent).toContain('Vidya');
  });

  it('renders the seven-step wizard', async () => {
    const el = root();
    await renderWizard(el, ctx);
    expect(el.textContent).toContain('Step 1 of 7');
  });

  it('escapes credential values', () => {
    const el = root();
    renderCredentials(el, ctx, {
      principalUsername: 'p@school',
      credentials: [
        { name: hostile, role: 'teacher', username: 's@school', tempPassword: 'safe-1234', classes: [] },
      ],
    });
    expect(el.textContent).toContain(hostile);
    expect([...el.querySelectorAll('b')].some((x) => x.textContent === 'x')).toBe(false);
  });
});
