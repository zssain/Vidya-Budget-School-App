import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/print.css';

import { registerView } from './core/router.js';
import { start } from './views/shell.js';
import * as home from './views/home.js';
import * as students from './views/students/list.js';
import * as student from './views/students/detail.js';
import * as attendance from './views/attendance.js';
import * as marks from './views/marks.js';
import * as fees from './views/fees/register.js';

registerView('home', { title: 'nav.home', navIcon: '🏠', nav: true, render: home.view });
registerView('students', {
  title: 'nav.students',
  navIcon: '👥',
  nav: true,
  permission: 'students.view',
  render: students.view,
});
registerView('student', { title: 'nav.student', permission: 'students.view', render: student.view });
registerView('attendance', {
  title: 'nav.attendance',
  navIcon: '📋',
  nav: true,
  permission: 'attendance.mark',
  render: attendance.view,
});
registerView('marks', {
  title: 'nav.marks',
  navIcon: '📝',
  nav: true,
  permission: 'marks.enter',
  render: marks.view,
});
registerView('fees', {
  title: 'nav.fees',
  navIcon: '₹',
  nav: true,
  permission: 'fees.view',
  render: fees.view,
});

start();
