import { beforeAll, describe, expect, it } from 'vitest';
import * as cmd from './commands.js';
import { view as attendanceView } from '../views/attendance.js';
import { view as marksView } from '../views/marks.js';
import { view as reportsView } from '../views/reports.js';
import { view as usersView } from '../views/users.js';
import { view as activityView } from '../views/activity.js';
import { view as backupView } from '../views/backup.js';
import { view as settingsView } from '../views/settings.js';

// The mock uses Web Crypto (PBKDF2/AES). Skip if the test env lacks it.
const hasCrypto = typeof globalThis.crypto?.subtle?.importKey === 'function';

describe.skipIf(!hasCrypto)('mock engine (smoke)', () => {
  beforeAll(async () => {
    await cmd.loadSampleSchool();
    const r = await cmd.signIn({ username: 'sunita', password: 'vidya123' });
    expect(r.status).toBe('ok');
  });

  it('signs in the principal with the right permissions', async () => {
    const u = await cmd.currentUser();
    expect(u.role).toBe('principal');
    expect(u.permissions).toContain('fees.collect');
  });

  it('lists the sample students', async () => {
    const list = await cmd.listStudents({ status: 'active' });
    expect(list.length).toBeGreaterThan(100);
    expect(list[0]).toHaveProperty('adm');
    expect(list[0]).toHaveProperty('feeState');
  });

  it('collects a fee and returns a numbered receipt', async () => {
    const list = await cmd.listStudents({ status: 'active' });
    const payer = list.find((s) => !s.rte && s.balance > 0);
    const before = payer.balance;
    const r = await cmd.collectFee({ studentId: payer.adm, amount: 100, mode: 'Cash' });
    expect(r.no).toMatch(/^PC-\d{4}$/);
    expect(r.balanceAfter).toBe(before - 100);
  });

  it('rejects a fee larger than the balance', async () => {
    const list = await cmd.listStudents({ status: 'active' });
    const payer = list.find((s) => !s.rte && s.balance > 0);
    await expect(
      cmd.collectFee({ studentId: payer.adm, amount: 99999999, mode: 'Cash' }),
    ).rejects.toMatchObject({
      kind: 'validation',
      field: 'amount',
    });
  });

  it('saves attendance for a section', async () => {
    const today = new Date();
    const key = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
    const sheet = await cmd.getAttendance({ sectionId: 'V-A', date: key });
    const marks = {};
    sheet.students.forEach((s) => (marks[s.adm] = 'P'));
    const saved = await cmd.saveAttendance({ sectionId: 'V-A', date: key, marks });
    expect(saved.savedBy).toBeTruthy();
  });

  it('builds the principal home dashboard', async () => {
    const home = await cmd.homePrincipal();
    expect(home.students).toBeGreaterThan(100);
    expect(home.fees).toHaveProperty('pending');
  });

  it('renders every remaining feature view with real mock DTOs', async () => {
    const me = await cmd.currentUser();
    const views = [
      [attendanceView, { me }],
      [marksView, { me }],
      [reportsView],
      [usersView],
      [activityView],
      [backupView],
      [settingsView, { me, refreshChrome() {} }],
    ];
    for (const [view, params] of views) {
      const root = document.createElement('div');
      await view(root, params);
      expect(root.textContent.length).toBeGreaterThan(0);
    }
  });
});
