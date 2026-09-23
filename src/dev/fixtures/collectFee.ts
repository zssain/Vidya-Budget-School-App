// Sample data for the Collect fee screen, copied VERBATIM from the mock's
// `<script type="text/x-dc">` `renderVals()` (design/screens/FeeCollection.dc.html,
// docs/01-MOCK-SPEC.md §8). Every string and number here matches the mock 1:1.
// The screen renders structural labels through i18n; this fixture carries the
// row/line-item data the way the mock's `rows` / statement lines compose it.

/** One row in the student search table. */
export interface CollectFeeStudentRow {
  /** Real student id (only set when wired to live data; not rendered). */
  id?: string
  /** Full name, e.g. "Kavya Singh". */
  name: string
  /** Admission number, e.g. "2023/0287". */
  adm: string
  /** Class label, e.g. "VI-B". */
  cls: string
  /** Formatted due amount, e.g. "₹3,100". */
  due: string
  /** Status label, e.g. "Part paid". */
  status: string
  /** Which pill variant to draw. */
  pill: 'partpaid' | 'paid' | 'unpaid'
}

/** The student the sheet is opened for. */
export interface CollectFeeStudent {
  /** Full name, e.g. "Kavya Singh". */
  name: string
  /** One-line meta under the name. */
  meta: string
  /** Two-letter initials on the navy avatar. */
  initials: string
}

/** One line in the fee statement inside the sheet. */
export interface CollectFeeLine {
  /** Line label, e.g. "Term 1 tuition". */
  label: string
  /** Formatted amount, e.g. "₹6,000". */
  amount: string
  /** Right-hand status text, e.g. "Paid" or "₹2,400 due". */
  right: string
  /** "Paid" (accent) renders differently from a "… due" balance. */
  paid: boolean
}

/** Everything the Collect fee screen needs from a fixture / seed. */
export interface CollectFeeData {
  /** The three search-result rows. */
  rows: CollectFeeStudentRow[]
  /** The student the sheet targets. */
  student: CollectFeeStudent
  /** The fee statement lines shown in the sheet. */
  lines: CollectFeeLine[]
  /** Formatted total due, e.g. "₹3,100". */
  totalDue: string
  /** Amount due, in whole rupees (drives the amount / balance / over logic). */
  due: number
}

export const collectFeeFixture: CollectFeeData = {
  // rows (verbatim from renderVals):
  //   { name:'Kavya Singh',  adm:'2023/0287', cls:'VI-B', due:'₹3,100', status:'Part paid', pill: partpaid }
  //   { name:'Kavya Mishra', adm:'2025/0142', cls:'II-A',  due:'₹0',     status:'Paid',      pill: paid }
  //   { name:'Kavya Reddy',  adm:'2024/0519', cls:'IV-A',  due:'₹6,500', status:'Unpaid',    pill: unpaid }
  rows: [
    { name: 'Kavya Singh', adm: '2023/0287', cls: 'VI-B', due: '₹3,100', status: 'Part paid', pill: 'partpaid' },
    { name: 'Kavya Mishra', adm: '2025/0142', cls: 'II-A', due: '₹0', status: 'Paid', pill: 'paid' },
    { name: 'Kavya Reddy', adm: '2024/0519', cls: 'IV-A', due: '₹6,500', status: 'Unpaid', pill: 'unpaid' },
  ],
  student: {
    name: 'Kavya Singh',
    meta: 'Class VI-B · Roll 14 · Adm. 2023/0287 · Transport',
    initials: 'KS',
  },
  // Fee statement lines (verbatim from the sheet body):
  lines: [
    { label: 'Term 1 tuition', amount: '₹6,000', right: 'Paid', paid: true },
    { label: 'Term 2 tuition', amount: '₹6,000', right: '₹2,400 due', paid: false },
    { label: 'Exam fee', amount: '₹500', right: '₹500 due', paid: false },
    { label: 'Transport · September', amount: '₹200', right: '₹200 due', paid: false },
  ],
  totalDue: '₹3,100',
  due: 3100,
}
