import type { Bundle } from '../index'

// Teacher Home screen strings (docs/01-MOCK-SPEC.md §8). en = verbatim mock text
// from design/screens/TeacherHome.dc.html; hi = the natural-Hindi translation
// (Phase 8, Part F).
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

const hi: Record<string, string> = {
  'teacher.eyebrow': 'बुधवार, 23 सितंबर',
  'teacher.h1': 'सुप्रभात, मीना।',
  'teacher.h1sub': 'दो काम बाकी हैं।',
  'teacher.task.attendance': 'V-A उपस्थिति',
  'teacher.task.attendanceSub': 'जमा नहीं किया गया · 34 विद्यार्थी',
  'teacher.start': 'शुरू करें',
  'teacher.task.reply': 'प्रधानाचार्य को जवाब दें',
  'teacher.task.replySub': 'राहुल कुमार का उपस्थिति सुधार वापस भेजा गया',
  'teacher.tile.attendance': 'उपस्थिति',
  'teacher.tile.marks': 'अंक',
  'teacher.tile.reportCard': 'रिपोर्ट कार्ड',
  'teacher.tile.myClasses': 'मेरी कक्षाएँ',
  'teacher.tile.students': 'विद्यार्थी',
  'teacher.tile.requests': 'मेरे अनुरोध',
  'teacher.tile.inbox': 'इनबॉक्स',
  'teacher.tile.cloudSync': 'सिंक',
  'teacher.tile.profile': 'प्रोफ़ाइल',
  'teacher.footer': 'अद्यतन · 2 मिनट पहले सिंक हुआ',
  'teacher.aria.notifications': 'सूचनाएँ, 2 अपठित',
  'teacher.aria.account': 'खाता',
}

const teacherHome: Bundle = { en, hi }
export default teacherHome
