import { describe, expect, it } from 'vitest';
import { canView, navItems, registerView } from './router.js';

registerView('home', { title: 'nav.home', nav: true });
registerView('students', { title: 'nav.students', nav: true, permission: 'students.view' });
registerView('attendance', { title: 'nav.attendance', nav: true, permission: 'attendance.mark' });
registerView('fees', { title: 'nav.fees', nav: true, permission: 'fees.view' });
registerView('reports', { title: 'nav.reports', nav: true, permission: 'reports.view' });
registerView('users', { title: 'nav.users', nav: true, permission: 'users.manage' });
registerView('backup', { title: 'nav.backup', nav: true, permission: 'backup.manage' });
registerView('settings', { title: 'nav.settings', nav: true, permission: 'settings.edit' });

const teacher = ['students.view', 'attendance.mark', 'marks.enter', 'reportcard.view'];

describe('router navigation by permission', () => {
  it('a teacher does not see fees, reports, users, backup or settings', () => {
    const ids = navItems(teacher).map((v) => v.id);
    expect(ids).toEqual(expect.arrayContaining(['home', 'students', 'attendance']));
    for (const hidden of ['fees', 'reports', 'users', 'backup', 'settings']) {
      expect(ids).not.toContain(hidden);
    }
  });

  it('canView respects the view permission', () => {
    expect(canView('students', teacher)).toBe(true);
    expect(canView('fees', teacher)).toBe(false);
    expect(canView('unknown', teacher)).toBe(false);
  });
});
