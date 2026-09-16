// Desktop entry point: load styles, register the views, and start the app.
import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/print.css';

import { registerView } from './core/router.js';
import { start } from './views/shell.js';

import * as home from './views/home.js';
import * as students from './views/students/list.js';
import * as studentDetail from './views/students/detail.js';
import * as fees from './views/fees/register.js';
import * as attendance from './views/attendance.js';
import * as marks from './views/marks.js';
import * as reports from './views/reports.js';
import * as users from './views/users.js';
import * as activity from './views/activity.js';
import * as backup from './views/backup.js';
import * as settings from './views/settings.js';

// Nav order matches the prototype.
registerView('home', {
  title: 'nav.home',
  navIcon: '🏠',
  navBg: 'var(--blue-l)',
  nav: true,
  render: home.view,
});
registerView('students', {
  title: 'nav.students',
  navIcon: '👥',
  navBg: 'var(--purple-l)',
  nav: true,
  permission: 'students.view',
  render: students.view,
});
registerView('student', {
  title: 'nav.student',
  nav: false,
  permission: 'students.view',
  render: studentDetail.view,
});
registerView('fees', {
  title: 'nav.fees',
  navIcon: '₹',
  navBg: 'var(--orange-l)',
  nav: true,
  permission: 'fees.view',
  render: fees.view,
});
registerView('attendance', {
  title: 'nav.attendance',
  navIcon: '📋',
  navBg: 'var(--green-l)',
  nav: true,
  permission: 'attendance.mark',
  render: attendance.view,
});
registerView('marks', {
  title: 'nav.marks',
  navIcon: '📝',
  navBg: 'var(--blue-l)',
  nav: true,
  permission: 'marks.enter',
  render: marks.view,
});
registerView('reports', {
  title: 'nav.reports',
  navIcon: '📊',
  navBg: 'var(--purple-l)',
  nav: true,
  permission: 'reports.view',
  render: reports.view,
});
registerView('users', {
  title: 'nav.users',
  navIcon: '🔑',
  navBg: 'var(--purple-l)',
  nav: true,
  permission: 'users.manage',
  render: users.view,
});
registerView('activity', {
  title: 'nav.activity',
  navIcon: '🕘',
  navBg: 'var(--line2)',
  nav: true,
  permission: 'activity.view',
  render: activity.view,
});
registerView('backup', {
  title: 'nav.backup',
  navIcon: '💾',
  navBg: 'var(--green-l)',
  nav: true,
  permission: 'backup.manage',
  render: backup.view,
});
registerView('settings', {
  title: 'nav.settings',
  navIcon: '⚙',
  navBg: 'var(--line2)',
  nav: true,
  permission: 'settings.edit',
  render: settings.view,
});

start();

// Release builds do not offer a general webview context menu. Text fields keep
// their native editing menu so copy and paste remain available.
if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (event) => {
    const target = event.target;
    if (!(target instanceof Element) || !target.closest('input, textarea')) event.preventDefault();
  });
}
