import type { Bundle } from '../index'

// Collect fee screen strings (docs/01-MOCK-SPEC.md §8, design/screens/FeeCollection.dc.html).
// en = verbatim mock text; hi mirrors each key with "TODO-HI: <english>".
// Dynamic strings compose with {var} interpolation (see the screen's t(...) calls):
//   fee.over    = "That is more than the ₹{due} due. Please check the amount."
//   fee.record  = "Record ₹{amount} payment"
//   fee.balanceSub = "₹{due} due − ₹{pay} this payment"
//   fee.success.line = "₹{pay} by {mode} · Kavya Singh · ₹{bal} still due"
const en: Record<string, string> = {
  // Sidebar
  'fee.school': 'Saraswati Public School',
  'fee.nav.home': 'Home',
  'fee.nav.collect': 'Collect fee',
  'fee.nav.students': 'Students & admissions',
  'fee.nav.receipts': 'Receipts',
  'fee.nav.daybook': 'Day book',
  'fee.nav.requests': 'My requests',
  'fee.user.name': 'Suresh Patel',
  'fee.user.role': 'Accountant',

  // Page header
  'fee.eyebrow': 'Fees · Term 2',
  'fee.h1': 'Collect a fee.',
  'fee.h1sub': 'Find the student first.',
  'fee.search.aria': 'Search student',

  // Student table
  'fee.table.student': 'Student',
  'fee.table.class': 'Class',
  'fee.table.due': 'Due',
  'fee.table.status': 'Status',
  'fee.table.adm': 'Adm. {adm}',

  // Sheet header
  'fee.sheet.aria': 'Collect fee for Kavya Singh',
  'fee.sheet.recordPayment': 'Record a payment',
  'fee.close': 'Close',

  // Statement
  'fee.totalDueNow': 'Total due now',

  // Amount
  'fee.amountReceived': 'Amount received',
  'fee.over': 'That is more than the ₹{due} due. Please check the amount.',
  'fee.chip.fullDue': 'Full due · ₹3,100',
  'fee.chip.thousand': '₹1,000',
  'fee.chip.fiveHundred': '₹500',

  // Payment mode
  'fee.mode': 'Payment mode',
  'fee.mode.aria': 'Payment mode',
  'fee.mode.cash': 'Cash',
  'fee.mode.upi': 'UPI',
  'fee.mode.cheque': 'Cheque',

  // Reference field
  'fee.ref.upi.label': 'UPI transaction ID',
  'fee.ref.upi.hint': 'e.g. 426518903214',
  'fee.ref.cheque.label': 'Cheque number and bank',
  'fee.ref.cheque.hint': 'e.g. 004512 · SBI',

  // Footer
  'fee.balanceAfter': 'Balance after',
  'fee.balanceSub': '₹{due} due − ₹{pay} this payment',
  'fee.record': 'Record ₹{amount} payment',
  'fee.enterValid': 'Enter a valid amount',
  'fee.receiptNote': 'A receipt number is given when you record the payment.',

  // Success panel
  'fee.success.title': 'Payment recorded.',
  'fee.success.receipt': 'Receipt R-A2-0419.',
  'fee.success.line': '₹{pay} by {mode} · Kavya Singh · ₹{bal} still due',
  'fee.confirmed': 'Confirmed by school server',
  'fee.print': 'Print receipt',
  'fee.whatsapp': 'Share on WhatsApp',
  'fee.another': 'Collect another fee',
}

const hi: Record<string, string> = Object.fromEntries(
  Object.entries(en).map(([k, v]) => [k, `TODO-HI: ${v}`]),
)

const collectFee: Bundle = { en, hi }
export default collectFee
