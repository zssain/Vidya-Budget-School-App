import type { Bundle } from '../index'

// Attendance screen strings (docs/01-MOCK-SPEC.md §8, design/screens/Attendance.dc.html).
// en = the mock's visible text word-for-word; hi mirrors every key with
// "TODO-HI: <english>" (Phase 8 fills real Hindi). No visible string is
// hard-coded in AttendanceScreen.tsx — it reads these via t('att.*').
const en: Record<string, string> = {
  'att.h1': 'Attendance.',
  'att.subtitle': 'Class V-A · Wed, 23 Sep',
  'att.count.present': 'Present',
  'att.count.absent': 'Absent',
  'att.count.leave': 'Leave',
  'att.count.notMarked': 'Not marked',
  'att.hint': '34 students · tap P, A or L',
  'att.markAll': 'Mark all present',
  'att.undo': 'Undo mark all',
  'att.notMarked': '{n} students not marked yet',
  'att.saveDraft': 'Save draft',
  'att.submit': 'Submit attendance',
  'att.submittedTitle': 'Submitted, saved on this phone.',
  'att.submittedSub':
    'It will reach the school when you are online. Changes now need a correction request.',
  'att.toast': 'Draft saved on this phone',
  'att.net.offline': 'Offline · saved on this phone',
  'att.net.waiting': 'Offline · 1 waiting to send',
  'att.aria.present': '{name} present',
  'att.aria.absent': '{name} absent',
  'att.aria.leave': '{name} on leave',
}

const attendance: Bundle = {
  en,
  hi: Object.fromEntries(Object.entries(en).map(([k, v]) => [k, `TODO-HI: ${v}`])),
}
export default attendance
