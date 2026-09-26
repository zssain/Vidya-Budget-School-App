// Desktop navigation model. The sidebar for each role is EXACTLY the mock's
// (Principal → design/screens/Main.dc.html sections; Accountant →
// design/screens/FeeCollection.dc.html flat list). AppShell renders these; App
// passes the active key per route. Sub-screens map to their parent nav key
// (e.g. a student profile → 'students', fee structure → 'fees').

import type { IconName } from '@/lib/icons'

export type Role = 'principal' | 'accountant' | 'teacher'

export interface NavItem {
  /** Active-state key (a screen tells AppShell which key it belongs to). */
  key: string
  labelKey: string
  icon: IconName
  path: string
  /** A live count badge, resolved by AppShell. */
  badge?: 'approvals' | 'requests'
  /** A `module_setting` key: when set and that module is OFF, the item is HIDDEN
   *  entirely (§14 — e.g. the optional School store). Items whose module defaults
   *  on (accounts, circulars) omit this and show a MODULE_OFF state on-screen. */
  module?: string
}

export interface NavSection {
  labelKey?: string
  items: NavItem[]
}

// Principal — prototype NAV_PRINCIPAL (VidyaPrototype.jsx): Overview / Academics /
// Finance / School. v2 (Phase 11): items whose module or screen does not exist yet
// are HIDDEN, not placeholders — Timetable & Calendar (P16/P13), Accounts & School
// store (P15) and Circulars (P14) appear as those phases build them. "Sync &
// devices" moved into Settings (a LinkRow there) and stays reachable from the
// header sync pill. "Day book" and "Reports" are reachable from the Fees screen
// (Day book becomes an Accounts tab in P15); their sub-routes highlight Fees.
export const PRINCIPAL_NAV: NavSection[] = [
  {
    labelKey: 'nav.section.overview',
    items: [
      { key: 'home', labelKey: 'nav.home', icon: 'home', path: '/principal/home' },
      { key: 'approvals', labelKey: 'nav.approvals', icon: 'approvals', path: '/principal/approvals', badge: 'approvals' },
    ],
  },
  {
    labelKey: 'nav.section.academics',
    items: [
      { key: 'students', labelKey: 'nav.students', icon: 'students', path: '/principal/students' },
      { key: 'attendance', labelKey: 'nav.attendance', icon: 'attendance', path: '/principal/attendance' },
      { key: 'marks', labelKey: 'nav.marks', icon: 'marks', path: '/principal/marks' },
      // Timetable (P16) — the `classroom` module is on by default (§14); hidden when
      // it is turned off. No `timetable` icon in the mock set (rule 6) → reuse `myClasses`.
      { key: 'timetable', labelKey: 'nav.timetable', icon: 'myClasses', path: '/principal/timetable', module: 'classroom' },
      // Calendar (P16 screen; P13 data) — foundational (attendance %/fees use it), so
      // always shown. No `calendar` icon in the mock set (rule 6) → reuse `clock`.
      { key: 'calendar', labelKey: 'nav.calendar', icon: 'clock', path: '/principal/calendar' },
    ],
  },
  {
    labelKey: 'nav.section.finance',
    items: [
      { key: 'fees', labelKey: 'nav.fees', icon: 'fees', path: '/principal/fees' },
      // Accounts (P15) — the `accounts` module is on by default (§14); the screen
      // shows a MODULE_OFF state if it is turned off. No `accounts` icon in the
      // mock set (rule 6) → reuse `daybook`.
      { key: 'accounts', labelKey: 'nav.accounts', icon: 'daybook', path: '/principal/accounts' },
    ],
  },
  {
    labelKey: 'nav.section.school',
    items: [
      { key: 'staff', labelKey: 'nav.staff', icon: 'staff', path: '/principal/staff' },
      // Circulars (P14) — the `circulars` module is on by default (§14); the screen
      // itself shows a MODULE_OFF state if it is turned off. No `circulars` icon in
      // the mock set (rule 6) → reuse `inbox`.
      { key: 'circulars', labelKey: 'nav.circulars', icon: 'inbox', path: '/principal/circulars' },
      // School store (P15) — OPTIONAL module `store`, default OFF (§14): the item
      // is HIDDEN entirely until the module is turned on. No `store` icon in the
      // mock set (rule 6) → reuse `receipts`.
      { key: 'schoolstore', labelKey: 'nav.schoolstore', icon: 'receipts', path: '/principal/schoolstore', module: 'store' },
      { key: 'backups', labelKey: 'nav.backups', icon: 'backups', path: '/principal/backups' },
      { key: 'settings', labelKey: 'nav.settings', icon: 'settings', path: '/principal/settings' },
    ],
  },
]

// Accountant — FeeCollection.dc.html: a flat list, no section labels.
export const ACCOUNTANT_NAV: NavSection[] = [
  {
    items: [
      { key: 'home', labelKey: 'nav.home', icon: 'home', path: '/accountant/home' },
      { key: 'collect', labelKey: 'nav.collect', icon: 'fees', path: '/accountant/collect' },
      { key: 'students', labelKey: 'nav.studentsAdmissions', icon: 'students', path: '/accountant/students' },
      { key: 'receipts', labelKey: 'nav.receipts', icon: 'receipts', path: '/accountant/receipts' },
      { key: 'daybook', labelKey: 'nav.daybook', icon: 'daybook', path: '/accountant/daybook' },
      { key: 'accounts', labelKey: 'nav.accounts', icon: 'daybook', path: '/accountant/accounts' },
      { key: 'schoolstore', labelKey: 'nav.schoolstore', icon: 'receipts', path: '/accountant/schoolstore', module: 'store' },
      // Calendar (P16): accountants view the school calendar (read-only, no edit).
      { key: 'calendar', labelKey: 'nav.calendar', icon: 'clock', path: '/accountant/calendar' },
      { key: 'requests', labelKey: 'nav.myRequests', icon: 'requests', path: '/accountant/requests', badge: 'requests' },
    ],
  },
]

export function navForRole(role: Role): NavSection[] {
  return role === 'accountant' ? ACCOUNTANT_NAV : PRINCIPAL_NAV
}
