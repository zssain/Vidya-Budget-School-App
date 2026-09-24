import type { Bundle } from '../index'

// Principal Home visible strings (docs/01-MOCK-SPEC.md §7/§8, design/screens/Main.dc.html).
// `en` = the mock's English text word-for-word; `hi` mirrors every key with
// real Hindi (Phase 8, Part F). No visible string in
// PrincipalHomeScreen is hard-coded — every static label comes from here via t().
// DATA (stat values, approval rows, class %, day amounts, notes) lives in the
// fixture, not here.
const en: Record<string, string> = {
  // Sidebar
  'home.school': 'Saraswati Public School',
  'home.logoAlt': 'Vidya Budget School',
  'home.nav.section.overview': 'Overview',
  'home.nav.home': 'Home',
  'home.nav.approvals': 'Approvals',
  'home.nav.section.academics': 'Academics',
  'home.nav.students': 'Students',
  'home.nav.attendance': 'Attendance',
  'home.nav.marks': 'Marks & reports',
  'home.nav.section.finance': 'Finance',
  'home.nav.fees': 'Fees',
  'home.nav.daybook': 'Day book',
  'home.nav.section.school': 'School',
  'home.nav.staff': 'Staff & access',
  'home.nav.sync': 'Sync & devices',
  'home.nav.backups': 'Backups',
  'home.nav.settings': 'Settings',
  'home.serverOnline': 'School server online',
  'home.thisPc': 'This PC · 9 devices connected',
  'home.user.initials': 'PS',
  'home.user.name': 'Priya Sharma',
  'home.user.role': 'Principal',
  // Header
  'home.session': 'Session 2026–27',
  'home.searchPlaceholder': 'Search students, receipts, staff…',
  'home.searchLabel': 'Search',
  'home.syncPill': 'All changes confirmed · 12 s ago',
  'home.bellLabel': 'Notifications, 3 unread',
  // Page title + actions
  'home.eyebrow': 'Wednesday, 23 September · Term 1',
  'home.h1': 'Good morning, Priya.',
  'home.h1sub': 'Four approvals are waiting.',
  'home.newAdmission': 'New admission',
  'home.reviewApprovals': 'Review approvals',
  // Approvals card
  'home.approvals.eyebrow': 'Approvals',
  'home.approvals.title': 'Waiting for your decision',
  'home.approvals.openAll': 'Open all',
  'home.approvals.review': 'Review',
  // Needs attention card
  'home.needs.eyebrow': 'Today',
  'home.needs.title': 'Needs attention',
  'home.needs.attendance.title': 'VII-B attendance not submitted',
  'home.needs.attendance.sub': 'Class teacher R. Nair · usually done by 10:30',
  'home.needs.conflict.title': 'One edit conflict to review',
  'home.needs.conflict.sub': 'Riya Verma · address changed on two phones',
  'home.lastBackup': 'Last backup',
  'home.lastBackupValue': 'Verified today at 6:02 AM · this PC and school Drive',
  // Attendance by class
  'home.attByClass': 'Attendance by class',
  'home.fullRegister': 'Full register',
  'home.classNotSubmitted': 'Not submitted',
  'home.classPresent': '{pct}% present',
  // Fee collection
  'home.feeCollection': 'Fee collection',
  'home.last6': 'Last 6 school days',
}

const hi: Record<string, string> = {
  // Sidebar
  'home.school': 'सरस्वती पब्लिक स्कूल',
  'home.logoAlt': 'Vidya Budget School',
  'home.nav.section.overview': 'अवलोकन',
  'home.nav.home': 'होम',
  'home.nav.approvals': 'स्वीकृतियाँ',
  'home.nav.section.academics': 'शैक्षणिक',
  'home.nav.students': 'विद्यार्थी',
  'home.nav.attendance': 'उपस्थिति',
  'home.nav.marks': 'अंक और रिपोर्ट',
  'home.nav.section.finance': 'वित्त',
  'home.nav.fees': 'फ़ीस',
  'home.nav.daybook': 'डे बुक',
  'home.nav.section.school': 'स्कूल',
  'home.nav.staff': 'स्टाफ़ और पहुँच',
  'home.nav.sync': 'सिंक और डिवाइस',
  'home.nav.backups': 'बैकअप',
  'home.nav.settings': 'सेटिंग्स',
  'home.serverOnline': 'स्कूल सर्वर ऑनलाइन',
  'home.thisPc': 'यह PC · 9 डिवाइस जुड़े',
  'home.user.initials': 'PS',
  'home.user.name': 'प्रिया शर्मा',
  'home.user.role': 'प्रधानाचार्य',
  // Header
  'home.session': 'सत्र 2026–27',
  'home.searchPlaceholder': 'विद्यार्थी, रसीद, स्टाफ़ खोजें…',
  'home.searchLabel': 'खोजें',
  'home.syncPill': 'सभी बदलाव पुष्ट · 12 सेकंड पहले',
  'home.bellLabel': 'सूचनाएँ, 3 अपठित',
  // Page title + actions
  'home.eyebrow': 'बुधवार, 23 सितंबर · टर्म 1',
  'home.h1': 'सुप्रभात, प्रिया।',
  'home.h1sub': 'चार स्वीकृतियाँ प्रतीक्षा में हैं।',
  'home.newAdmission': 'नया प्रवेश',
  'home.reviewApprovals': 'स्वीकृतियाँ देखें',
  // Approvals card
  'home.approvals.eyebrow': 'स्वीकृतियाँ',
  'home.approvals.title': 'आपके निर्णय की प्रतीक्षा में',
  'home.approvals.openAll': 'सभी खोलें',
  'home.approvals.review': 'देखें',
  // Needs attention card
  'home.needs.eyebrow': 'आज',
  'home.needs.title': 'ध्यान देने योग्य',
  'home.needs.attendance.title': 'VII-B उपस्थिति जमा नहीं',
  'home.needs.attendance.sub': 'कक्षा शिक्षक R. Nair · आमतौर पर 10:30 तक हो जाती है',
  'home.needs.conflict.title': 'समीक्षा हेतु एक संपादन टकराव',
  'home.needs.conflict.sub': 'रिया वर्मा · पता दो फ़ोनों पर बदला गया',
  'home.lastBackup': 'अंतिम बैकअप',
  'home.lastBackupValue': 'आज 6:02 AM पर सत्यापित · यह PC और स्कूल Drive',
  // Attendance by class
  'home.attByClass': 'कक्षावार उपस्थिति',
  'home.fullRegister': 'पूरा रजिस्टर',
  'home.classNotSubmitted': 'जमा नहीं',
  'home.classPresent': '{pct}% उपस्थित',
  // Fee collection
  'home.feeCollection': 'फ़ीस संग्रह',
  'home.last6': 'पिछले 6 स्कूल दिन',
}

const principalHome: Bundle = { en, hi }
export default principalHome
