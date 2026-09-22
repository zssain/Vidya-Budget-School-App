import type { Bundle } from '../index'

// Principal Home visible strings (docs/01-MOCK-SPEC.md §7/§8, design/screens/Main.dc.html).
// `en` = the mock's English text word-for-word; `hi` mirrors every key with
// "TODO-HI: <english>" (Phase 8 fills real Hindi). No visible string in
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

const principalHome: Bundle = {
  en,
  hi: Object.fromEntries(
    Object.entries(en).map(([k, v]) => [k, 'TODO-HI: ' + v]),
  ) as Record<string, string>,
}
export default principalHome
