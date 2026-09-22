import type { Bundle } from '../index'

// Teacher Home screen strings (docs/01-MOCK-SPEC.md §8). en = verbatim mock text
// from design/screens/TeacherHome.dc.html; hi mirrors every key with
// "TODO-HI: <english>" until Phase 8 fills real Hindi.
const en: Record<string, string> = {
  'teacher.eyebrow': 'Wednesday, 23 September',
  'teacher.h1': 'Good morning, Meena.',
  'teacher.h1sub': 'Two things are due.',
  'teacher.task.attendance': 'V-A attendance',
  'teacher.task.attendanceSub': 'Not submitted · 34 students',
  'teacher.start': 'Start',
  'teacher.task.reply': 'Reply to the Principal',
  'teacher.task.replySub': 'Rahul Kumar’s attendance correction was returned',
  'teacher.tile.attendance': 'Attendance',
  'teacher.tile.marks': 'Marks',
  'teacher.tile.reportCard': 'Report cards',
  'teacher.tile.myClasses': 'My classes',
  'teacher.tile.students': 'Students',
  'teacher.tile.requests': 'My requests',
  'teacher.tile.inbox': 'Inbox',
  'teacher.tile.cloudSync': 'Sync',
  'teacher.tile.profile': 'Profile',
  'teacher.footer': 'Up to date · synced 2 min ago',
  'teacher.aria.notifications': 'Notifications, 2 unread',
  'teacher.aria.account': 'Account',
}

const hi: Record<string, string> = Object.fromEntries(
  Object.entries(en).map(([k, v]) => [k, `TODO-HI: ${v}`]),
)

const teacherHome: Bundle = { en, hi }
export default teacherHome
