import type { Bundle } from '../index'

// Attendance screen strings (docs/01-MOCK-SPEC.md §8, design/screens/Attendance.dc.html).
// en = the mock's visible text word-for-word; hi = the natural-Hindi
// translation (Phase 8, Part F). No visible string is
// hard-coded in AttendanceScreen.tsx — it reads these via t('att.*').
const en: Record<string, string> = {
  'att.h1': 'Attendance.',
  'att.subtitle': 'Class V-A · Wed, 23 Sep',
  'att.count.present': 'Present',
  'att.count.absent': 'Absent',
  'att.count.notMarked': 'Not marked',
  'att.leaveOld': 'Leave (old)',
  'att.hint': '34 students · tap P or A',
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
}

const hi: Record<string, string> = {
  'att.h1': 'उपस्थिति।',
  'att.subtitle': 'कक्षा V-A · बुध, 23 सितंबर',
  'att.count.present': 'उपस्थित',
  'att.count.absent': 'अनुपस्थित',
  'att.count.notMarked': 'चिह्नित नहीं',
  'att.leaveOld': 'अवकाश (पुराना)',
  'att.hint': '34 विद्यार्थी · P या A दबाएँ',
  'att.markAll': 'सभी को उपस्थित करें',
  'att.undo': 'सभी उपस्थित पूर्ववत करें',
  'att.notMarked': '{n} विद्यार्थी अभी चिह्नित नहीं',
  'att.saveDraft': 'ड्राफ़्ट सहेजें',
  'att.submit': 'उपस्थिति जमा करें',
  'att.submittedTitle': 'जमा किया गया, इस फ़ोन पर सहेजा गया।',
  'att.submittedSub':
    'ऑनलाइन होने पर यह स्कूल तक पहुँच जाएगी। अब बदलाव के लिए सुधार अनुरोध चाहिए।',
  'att.toast': 'ड्राफ़्ट इस फ़ोन पर सहेजा गया',
  'att.net.offline': 'ऑफ़लाइन · इस फ़ोन पर सहेजा गया',
  'att.net.waiting': 'ऑफ़लाइन · 1 भेजने के लिए बाकी',
  'att.aria.present': '{name} उपस्थित',
  'att.aria.absent': '{name} अनुपस्थित',
}

const attendance: Bundle = { en, hi }
export default attendance
