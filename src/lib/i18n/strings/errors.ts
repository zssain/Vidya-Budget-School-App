import type { Bundle } from '../index'

// Error + licence + setup/PIN/approvals copy (prompts/P03). `error.<CODE>` keys
// mirror vidya-core's CoreError codes (Phase 2 handoff). Licence copy is verbatim
// from the prompt; copy marked [OWNER] is a sensible default pending confirmation.
const errors: Bundle = {
  en: {
    // vidya-core error codes → user copy.
    'error.LOCKED': 'Please sign in to continue.',
    'error.FORBIDDEN': 'You do not have permission to do that.',
    'error.NOT_FOUND': 'Not found.',
    'error.VALIDATION': 'Please check this and try again.',
    'error.AMOUNT_EXCEEDS_DUE': 'Amount is more than the total due.',
    'error.NO_DUES': 'Nothing is due for this student.',
    'error.SHEET_LOCKED': 'This sheet is submitted. Request a correction.',
    'error.INCOMPLETE_SHEET': '{remaining} students not marked yet',
    'error.REQUEST_ALREADY_PENDING': 'A request is already pending.',
    'error.REQUEST_STALE': 'The record changed — please review again.',
    'error.DUPLICATE_ADMISSION_NO': 'That admission number is already used.',
    'error.LICENCE_INVALID': 'This licence could not be verified. Contact support.',
    'error.LICENCE_REVOKED': 'This licence has been revoked. Contact support.',
    'error.LICENCE_MOVED': 'This school has moved to another computer.',
    'error.LICENCE_LIMIT': 'Your licence limit has been reached ({what}).',
    'error.PIN_WRONG': 'Wrong PIN. {remaining} tries left.',
    'error.PIN_LOCKED': 'Too many tries. Try again after {until}.',
    'error.SESSION_READ_ONLY': 'This session is read-only.',
    'error.LEASE_EXPIRED': 'Connect to your school to continue.',
    'error.EPOCH_OLD': 'This computer is no longer the school server.',
    'error.INTERNAL': 'Something went wrong. Please try again.',
    'error.DB_KEY_MISSING': "This device's key is missing. Recover to continue.",

    // Licence activation copy (prompt Step 3, verbatim).
    'licence.needs_internet': 'Activation needs internet once — please try again.',
    'licence.code_already_used': 'This code has already been used by another school. Contact support.',
    'licence.code_not_found': "That activation code wasn't found. Please check it and try again.",
    'licence.invalid': 'This licence could not be verified. Contact support.',
    'licence.banner_revoked': 'This licence has been revoked. Contact support.',
    'licence.banner_moved': 'This school has moved to another computer. This one is read-only.',

    // Setup wizard.
    'setup.title': 'Set up your school',
    'setup.step.school': 'School',
    'setup.step.session': 'Session & terms',
    'setup.step.classes': 'Classes',
    'setup.step.you': 'You',
    'setup.step.recovery': 'Recovery key',
    'setup.step.ready': 'Ready',
    'setup.continue': 'Continue',
    'setup.back': 'Back',
    'setup.school.name': 'School name',
    'setup.school.address': 'Address',
    'setup.school.board': 'Board',
    'setup.school.udise': 'UDISE code (optional)',
    'setup.school.phone': 'Phone',
    'setup.session.label': 'Session',
    'setup.session.term1': 'Term 1',
    'setup.session.term2': 'Term 2',
    'setup.classes.add': 'Add section',
    'setup.you.name': 'Your name',
    'setup.you.mobile': 'Mobile',
    'setup.you.pin': 'PIN (4–6 digits)',
    'setup.you.pin_again': 'Enter PIN again',
    'setup.recovery.intro':
      'This key restores backups and moves Vidya to a new PC. Vidya cannot recover it for you. Keep it safe.',
    'setup.recovery.print': 'Print',
    'setup.recovery.copy': 'Copy',
    'setup.recovery.retype': 'Type groups 3 and 5 to confirm you saved it',
    'setup.recovery.group3': 'Group 3',
    'setup.recovery.group5': 'Group 5',
    'setup.ready.licence': 'Licence active',
    'setup.ready.school': 'School created',
    'setup.ready.recovery': 'Recovery key saved',
    'setup.ready.staff': 'Invite staff (later)',
    'setup.ready.drive': 'Connect Google Drive (later)',
    'setup.ready.finish': 'Open Vidya',

    // PIN / unlock.
    'pin.unlock': 'Unlock',
    'pin.enter': 'Enter your PIN',
    'pin.choose_user': 'Who are you?',
    'pin.switch_user': 'Switch user',

    // Approvals.
    'approvals.title': 'Approvals',
    'approvals.tab.all': 'All',
    'approvals.tab.marks': 'Marks',
    'approvals.tab.payment': 'Payment reversal',
    'approvals.tab.attendance': 'Attendance',
    'approvals.tab.details': 'Student details',
    'approvals.tab.access': 'Access',
    'approvals.approve': 'Approve',
    'approvals.return': 'Return',
    'approvals.reject': 'Reject',
    'approvals.before': 'Before',
    'approvals.after': 'After',
    'approvals.empty': 'No requests here.',

    // Collect fee (real-data wiring).
    'fee.success.receiptDyn': 'Receipt {no}.',
    'fee.comms.notice': 'Printing and WhatsApp share are coming in a later phase.',
  },
  hi: {},
}

// Mirror every English key into Hindi as "TODO-HI: <en>" (Phase 8 fills real Hindi).
errors.hi = Object.fromEntries(
  Object.entries(errors.en).map(([k, v]) => [k, `TODO-HI: ${v}`])
)

export default errors
