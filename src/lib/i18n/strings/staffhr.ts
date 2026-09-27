// Staff HR (P17, §10.5): Settings → Staff HR, phone check-in / apply-for-leave,
// the Approvals leave detail, and the salary/substitute links. English is
// authoritative; Hindi is a natural translation. Telugu falls back to English
// (full te translation is a native-speaker task, OWNER-DECISIONS #12).
import type { Bundle } from '../index'

const staffhr: Bundle = {
  en: {
    // Settings → Staff HR (Step 1)
    'staffhr.settings.eyebrow': 'Staff HR',
    'staffhr.settings.title': 'Staff attendance & leave',
    'staffhr.settings.start': 'School start time',
    'staffhr.settings.startHint': 'Check-ins after this time (plus grace) are marked late.',
    'staffhr.settings.grace': 'Late grace (minutes)',
    'staffhr.settings.allowAway': 'Allow check-in away from school',
    'staffhr.settings.allowAwayHint': 'Off: a check-in that is not on the school Wi-Fi waits for you to accept it.',
    'staffhr.settings.save': 'Save',
    'staffhr.settings.saved': 'Saved',
    'staffhr.settings.invalid': 'Check the start time (HH:MM).',
    'staffhr.leave.title': 'Leave types',
    'staffhr.leave.hint': 'Yearly quota is in working days. An unpaid type has no quota.',
    'staffhr.leave.name': 'Name',
    'staffhr.leave.quota': 'Quota (days / year)',
    'staffhr.leave.unlimited': 'Unlimited',
    'staffhr.leave.paid': 'Paid',
    'staffhr.leave.unpaid': 'Unpaid',
    'staffhr.leave.active': 'Active',
    'staffhr.leave.inactive': 'Hidden',
    'staffhr.leave.add': 'Add leave type',
    'staffhr.leave.save': 'Save',
    'staffhr.leave.namePlaceholder': 'e.g. Casual leave',
  },
  hi: {
    'staffhr.settings.eyebrow': 'स्टाफ़ एचआर',
    'staffhr.settings.title': 'स्टाफ़ उपस्थिति और अवकाश',
    'staffhr.settings.start': 'स्कूल शुरू होने का समय',
    'staffhr.settings.startHint': 'इस समय (और छूट) के बाद की चेक-इन देर से मानी जाती है।',
    'staffhr.settings.grace': 'देरी की छूट (मिनट)',
    'staffhr.settings.allowAway': 'स्कूल से दूर चेक-इन की अनुमति दें',
    'staffhr.settings.allowAwayHint': 'बंद: स्कूल के वाई-फ़ाई पर न होने वाली चेक-इन आपकी स्वीकृति का इंतज़ार करती है।',
    'staffhr.settings.save': 'सहेजें',
    'staffhr.settings.saved': 'सहेजा गया',
    'staffhr.settings.invalid': 'शुरू होने का समय जाँचें (HH:MM)।',
    'staffhr.leave.title': 'अवकाश के प्रकार',
    'staffhr.leave.hint': 'वार्षिक कोटा कार्यदिवसों में है। अवैतनिक प्रकार का कोई कोटा नहीं होता।',
    'staffhr.leave.name': 'नाम',
    'staffhr.leave.quota': 'कोटा (दिन / वर्ष)',
    'staffhr.leave.unlimited': 'असीमित',
    'staffhr.leave.paid': 'सवेतन',
    'staffhr.leave.unpaid': 'अवैतनिक',
    'staffhr.leave.active': 'सक्रिय',
    'staffhr.leave.inactive': 'छिपा हुआ',
    'staffhr.leave.add': 'अवकाश प्रकार जोड़ें',
    'staffhr.leave.save': 'सहेजें',
    'staffhr.leave.namePlaceholder': 'जैसे आकस्मिक अवकाश',
  },
}

export default staffhr
