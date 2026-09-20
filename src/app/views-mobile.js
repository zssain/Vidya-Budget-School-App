import { desktopViews } from './views-desktop.js';
const allowed = new Set([
  'home',
  'students',
  'student',
  'student-form',
  'reportcard',
  'attendance',
  'marks',
  'fees',
  'collect-fee',
  'receipt',
  'daybook',
]);
export const mobileViews = desktopViews.filter((view) => allowed.has(view.id));
