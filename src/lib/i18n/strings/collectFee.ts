import type { Bundle } from '../index'

// Collect fee screen strings (docs/01-MOCK-SPEC.md §8, design/screens/FeeCollection.dc.html).
// en = verbatim mock text; hi = the natural-Hindi translation (Phase 8, Part F).
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

const hi: Record<string, string> = {
  // Sidebar
  'fee.school': 'सरस्वती पब्लिक स्कूल',
  'fee.nav.home': 'होम',
  'fee.nav.collect': 'फ़ीस लें',
  'fee.nav.students': 'विद्यार्थी और प्रवेश',
  'fee.nav.receipts': 'रसीदें',
  'fee.nav.daybook': 'डे बुक',
  'fee.nav.requests': 'मेरे अनुरोध',
  'fee.user.name': 'सुरेश पटेल',
  'fee.user.role': 'लेखाकार',

  // Page header
  'fee.eyebrow': 'फ़ीस · टर्म 2',
  'fee.h1': 'फ़ीस लें।',
  'fee.h1sub': 'पहले विद्यार्थी खोजें।',
  'fee.search.aria': 'विद्यार्थी खोजें',

  // Student table
  'fee.table.student': 'विद्यार्थी',
  'fee.table.class': 'कक्षा',
  'fee.table.due': 'बकाया',
  'fee.table.status': 'स्थिति',
  'fee.table.adm': 'प्रवेश {adm}',

  // Sheet header
  'fee.sheet.aria': 'कव्या सिंह की फ़ीस लें',
  'fee.sheet.recordPayment': 'भुगतान दर्ज करें',
  'fee.close': 'बंद करें',

  // Statement
  'fee.totalDueNow': 'अभी कुल बकाया',

  // Amount
  'fee.amountReceived': 'प्राप्त राशि',
  'fee.over': 'यह ₹{due} बकाया से अधिक है। कृपया राशि जाँचें।',
  'fee.chip.fullDue': 'पूरा बकाया · ₹3,100',
  'fee.chip.thousand': '₹1,000',
  'fee.chip.fiveHundred': '₹500',

  // Payment mode
  'fee.mode': 'भुगतान का तरीका',
  'fee.mode.aria': 'भुगतान का तरीका',
  'fee.mode.cash': 'नकद',
  'fee.mode.upi': 'UPI',
  'fee.mode.cheque': 'चेक',

  // Reference field
  'fee.ref.upi.label': 'UPI लेनदेन ID',
  'fee.ref.upi.hint': 'उदा. 426518903214',
  'fee.ref.cheque.label': 'चेक नंबर और बैंक',
  'fee.ref.cheque.hint': 'उदा. 004512 · SBI',

  // Footer
  'fee.balanceAfter': 'भुगतान के बाद शेष',
  'fee.balanceSub': '₹{due} बकाया − ₹{pay} यह भुगतान',
  'fee.record': '₹{amount} का भुगतान दर्ज करें',
  'fee.enterValid': 'मान्य राशि दर्ज करें',
  'fee.receiptNote': 'भुगतान दर्ज करने पर रसीद नंबर मिलता है।',

  // Success panel
  'fee.success.title': 'भुगतान दर्ज हो गया।',
  'fee.success.receipt': 'रसीद R-A2-0419.',
  'fee.success.line': '₹{pay} {mode} से · कव्या सिंह · ₹{bal} अभी बकाया',
  'fee.confirmed': 'स्कूल सर्वर द्वारा पुष्ट',
  'fee.print': 'रसीद प्रिंट करें',
  'fee.whatsapp': 'WhatsApp पर साझा करें',
  'fee.another': 'एक और फ़ीस लें',
}

const collectFee: Bundle = { en, hi }
export default collectFee
