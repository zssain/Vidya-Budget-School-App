import type { Bundle } from '../index'

// Attendance screen strings (docs/01-MOCK-SPEC.md §8, design/screens/Attendance.dc.html).
// en = the mock's visible text word-for-word; hi = the natural-Hindi
// translation (Phase 8, Part F). No visible string is
// hard-coded in AttendanceScreen.tsx — it reads these via t('att.*').
const en: Record<string, string> = {
  // Absence alerts (P14 Step 4, prototype `absence`). Email = queued (the school
  // PC sends later — honest status, not "sent").
  'absence.title': 'Absent today.',
  'absence.back': 'Back to home',
  'absence.oneAbsent': '1 student absent',
  'absence.nAbsent': '{n} students absent',
  'absence.intro': 'Let parents know. Both are free: email from the school, or WhatsApp with the message ready.',
  'absence.guardian': 'Guardian',
  'absence.noGuardian': 'No guardian on file',
  'absence.email': 'Email',
  'absence.whatsapp': 'WhatsApp',
  'absence.queued': 'Email queued',
  'absence.opened': 'WhatsApp opened',
  'absence.noEmail': 'No email',
  'absence.noConsent': 'No consent',
  'absence.message': 'Message',
  'absence.emailAll': 'Email all parents',
  'absence.queuedNote': 'Emails are sent from the school computer when it is online.',
  'absence.none': 'No absentees today.',
  'absence.notSubmitted': 'Submit the attendance first.',
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
  'absence.title': 'आज अनुपस्थित।',
  'absence.back': 'होम पर वापस',
  'absence.oneAbsent': '1 विद्यार्थी अनुपस्थित',
  'absence.nAbsent': '{n} विद्यार्थी अनुपस्थित',
  'absence.intro': 'अभिभावकों को बताएँ। दोनों मुफ़्त हैं: स्कूल से ईमेल, या तैयार संदेश के साथ WhatsApp।',
  'absence.guardian': 'अभिभावक',
  'absence.noGuardian': 'कोई अभिभावक दर्ज नहीं',
  'absence.email': 'ईमेल',
  'absence.whatsapp': 'WhatsApp',
  'absence.queued': 'ईमेल कतार में',
  'absence.opened': 'WhatsApp खुला',
  'absence.noEmail': 'ईमेल नहीं',
  'absence.noConsent': 'सहमति नहीं',
  'absence.message': 'संदेश',
  'absence.emailAll': 'सभी अभिभावकों को ईमेल करें',
  'absence.queuedNote': 'स्कूल कंप्यूटर ऑनलाइन होने पर ईमेल भेजे जाते हैं।',
  'absence.none': 'आज कोई अनुपस्थित नहीं।',
  'absence.notSubmitted': 'पहले उपस्थिति जमा करें।',
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
