// Principal Home sample data — copied VERBATIM from the mock's renderVals()
// (design/screens/Main.dc.html, docs/01-MOCK-SPEC.md §8). Phase 1 duplicates the
// mock numbers here; later phases reproduce them from the demo seed.
//
// Only DATA lives here (stat values/labels/notes, approval rows, class %, day
// amounts, fee total). The computed inline styles (cell/badge/bar) are rebuilt
// in PrincipalHomeScreen exactly as renderVals() does, so they are NOT stored.

/** One stat-strip cell. value/label/note are the exact mock strings. */
export interface HomeStat {
  value: string
  label: string
  note: string
}

/** Badge colours for an approval pill (mock: pill(bg, fg)). */
export interface ApprovalBadge {
  bg: string
  fg: string
}

/** One approval row. `badge.accent` marks the attendance-correction pill that
 *  uses the runtime accent (bg = accent-12, fg = accent). */
export interface HomeApproval {
  type: string
  badge: ApprovalBadge & { accent?: boolean }
  what: string
  who: string
  age: string
}

/** One "Attendance by class" row. pct is null when not submitted. */
export interface HomeClass {
  name: string
  pct: number | null
}

/** One "Fee collection" column. */
export interface HomeDay {
  day: string
  value: number
}

export interface PrincipalHomeData {
  stats: HomeStat[]
  approvals: HomeApproval[]
  classes: HomeClass[]
  days: HomeDay[]
  /** Fee-collection total (mock: '₹2,51,200'). */
  feeTotal: string
}

export const principalHomeFixture: PrincipalHomeData = {
  stats: [
    { value: '91.4%', label: 'attendance today', note: '612 of 670 marked · VII-B pending' },
    {
      value: '₹48,500',
      label: 'collected today',
      note: '23 receipts · ₹2,400 more waiting for server',
    },
    { value: '₹6,84,200', label: 'outstanding this term', note: '212 students with dues' },
    { value: '670', label: 'active students', note: '4 new admissions this week' },
  ],
  approvals: [
    {
      type: 'Marks correction',
      badge: { bg: '#E7E3F1', fg: '#4A3B78' },
      what: 'Half Yearly · VI-B Maths · Kavya Singh 62 → 72',
      who: 'Anita Rao, Teacher',
      age: '2 hours ago',
    },
    {
      type: 'Payment reversal',
      badge: { bg: '#F6E4E2', fg: '#8E2F2A' },
      what: 'Receipt R-A2-0418 · ₹1,500 · entered twice',
      who: 'Suresh Patel, Accountant',
      age: '5 hours ago',
    },
    {
      type: 'Attendance correction',
      // mock: pill(rgba(0.12), accent) → var(--accent-12) / var(--accent)
      badge: { bg: 'var(--accent-12)', fg: 'var(--accent)', accent: true },
      what: 'V-A · 19 Sep · Rahul Kumar: Absent → Present',
      who: 'Meena Iyer, Teacher',
      age: '1 day ago',
    },
    {
      type: 'Student details',
      badge: { bg: '#F4ECDC', fg: '#6B5220' },
      what: 'Aarav Gupta · father’s mobile number',
      who: 'Suresh Patel, Accountant',
      age: '2 days ago',
    },
  ],
  classes: [
    { name: 'Nursery', pct: 96 },
    { name: 'LKG', pct: 92 },
    { name: 'I-A', pct: 94 },
    { name: 'II-A', pct: 90 },
    { name: 'III-A', pct: 93 },
    { name: 'V-A', pct: 91 },
    { name: 'VII-B', pct: null },
  ],
  days: [
    { day: 'Thu', value: 32000 },
    { day: 'Fri', value: 41000 },
    { day: 'Sat', value: 28500 },
    { day: 'Mon', value: 55000 },
    { day: 'Tue', value: 46200 },
    { day: 'Today', value: 48500 },
  ],
  feeTotal: '₹2,51,200',
}

export default principalHomeFixture
