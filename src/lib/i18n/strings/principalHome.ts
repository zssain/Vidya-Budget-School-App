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
  // Page title + actions (greeting/subtitle/eyebrow are DATA now — see the container)
  'home.greeting.morning': 'Good morning, {name}.',
  'home.greeting.afternoon': 'Good afternoon, {name}.',
  'home.greeting.evening': 'Good evening, {name}.',
  'home.subtitle.none': "You're all caught up.",
  'home.subtitle.one': 'One approval is waiting.',
  'home.subtitle.many': '{n} approvals are waiting.',
  'home.newAdmission': 'New admission',
  'home.reviewApprovals': 'Review approvals',
  // Approvals card
  'home.approvals.eyebrow': 'Approvals',
  'home.approvals.title': 'Waiting for your decision',
  'home.approvals.openAll': 'Open all',
  'home.approvals.review': 'Review',
  // Needs attention card (items are DATA now — see the container)
  'home.needs.eyebrow': 'Today',
  'home.needs.title': 'Needs attention',
  'home.needs.empty': 'Nothing needs your attention right now.',
  'home.needs.unsubmitted': '{class} attendance not submitted',
  'home.needs.unsubmittedSub': 'Not submitted yet today.',
  'home.needs.unsubmittedSubTeacher': 'Class teacher {name} · not submitted yet today',
  'home.needs.conflictOne': 'One edit conflict to review',
  'home.needs.conflictMany': '{n} edit conflicts to review',
  'home.needs.conflictSub': 'Open Conflicts to resolve.',
  'home.lastBackup': 'Last backup',
  'home.noBackup': 'No backup yet.',
  'home.backupLine': 'Backed up {when}',
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
  // Page title + actions (greeting/subtitle/eyebrow are DATA now — see the container)
  'home.greeting.morning': 'सुप्रभात, {name}।',
  'home.greeting.afternoon': 'नमस्ते, {name}।',
  'home.greeting.evening': 'शुभ संध्या, {name}।',
  'home.subtitle.none': 'सब कुछ पूरा है।',
  'home.subtitle.one': 'एक स्वीकृति प्रतीक्षा में है।',
  'home.subtitle.many': '{n} स्वीकृतियाँ प्रतीक्षा में हैं।',
  'home.newAdmission': 'नया प्रवेश',
  'home.reviewApprovals': 'स्वीकृतियाँ देखें',
  // Approvals card
  'home.approvals.eyebrow': 'स्वीकृतियाँ',
  'home.approvals.title': 'आपके निर्णय की प्रतीक्षा में',
  'home.approvals.openAll': 'सभी खोलें',
  'home.approvals.review': 'देखें',
  // Needs attention card (items are DATA now — see the container)
  'home.needs.eyebrow': 'आज',
  'home.needs.title': 'ध्यान देने योग्य',
  'home.needs.empty': 'अभी ध्यान देने योग्य कुछ नहीं है।',
  'home.needs.unsubmitted': '{class} उपस्थिति जमा नहीं',
  'home.needs.unsubmittedSub': 'आज अभी तक जमा नहीं।',
  'home.needs.unsubmittedSubTeacher': 'कक्षा शिक्षक {name} · आज अभी तक जमा नहीं',
  'home.needs.conflictOne': 'समीक्षा हेतु एक संपादन टकराव',
  'home.needs.conflictMany': 'समीक्षा हेतु {n} संपादन टकराव',
  'home.needs.conflictSub': 'हल करने के लिए टकराव खोलें।',
  'home.lastBackup': 'अंतिम बैकअप',
  'home.noBackup': 'अभी कोई बैकअप नहीं।',
  'home.backupLine': '{when} बैकअप हुआ',
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
