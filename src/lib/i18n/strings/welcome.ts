import type { Bundle } from '../index'

// Every visible string on the Welcome screen, verbatim from
// design/screens/Welcome.dc.html (docs/01-MOCK-SPEC.md §8). `en` is word-for-word
// the mock; `hi` mirrors each key with "TODO-HI: <english>" (Phase 8 fills real
// Hindi). The three radio-option title/sub strings live under welcome.opt.*.
const en: Record<string, string> = {
  // Left panel — header
  'welcome.help': 'Help',
  // Left panel — hero
  'welcome.eyebrow': 'Your school on Vidya',
  'welcome.encrypted': 'Encrypted on your devices',
  'welcome.h1': 'Namaste.',
  'welcome.intro':
    'How would you like to begin? You only need an activation code when you are setting up a new school.',
  // Radio group
  'welcome.radiogroup.label': 'How to begin',
  'welcome.opt.setup.title': 'Set up my school',
  'welcome.opt.setup.sub': 'For the Principal · uses the activation code from your purchase',
  'welcome.opt.join.title': 'Join my school',
  'welcome.opt.join.sub': 'For teachers and accountants · uses your invitation',
  'welcome.opt.recover.title': 'Recover an existing school',
  'welcome.opt.recover.sub': 'Move Vidya to a new computer from a backup',
  // Setup field
  'welcome.field.codeLabel': 'Activation code',
  'welcome.field.codePlaceholder': 'VIDYA-XXXX-XXXX-XXXX',
  'welcome.field.codeHint': 'It is in your purchase email and on your account page.',
  // Join field
  'welcome.field.invLabel': 'Invitation link or code',
  'welcome.field.invPlaceholder': 'Paste the link your Principal sent',
  'welcome.field.invHint': 'On a phone you can also scan the invitation QR code.',
  // Recover note
  'welcome.recover.title': 'Owner verification needed',
  'welcome.recover.body':
    'We will confirm the school’s licence and check your backup before anything is restored.',
  // CTAs
  'welcome.cta.setup': 'Activate and continue',
  'welcome.cta.join': 'Join school',
  'welcome.cta.recover': 'Start recovery',
  // Sign-in row
  'welcome.signin.prompt': 'Already set up on this device? ',
  'welcome.signin.link': 'Sign in',
  // Footer
  'welcome.footer.copyright': '© 2026 [Your company]',
  'welcome.footer.privacy': 'Privacy',
  'welcome.footer.terms': 'Terms',
  // Right panel (navy)
  'welcome.right.eyebrow': 'School management for Indian schools',
  'welcome.right.h2': 'Every class. Every rupee.',
  'welcome.right.h2Italic': 'One clear record.',
  'welcome.right.body':
    'Attendance, marks and fees are saved on the phone first, then confirmed by your school’s own computer.',
  'welcome.right.doorCaption': 'A door to every classroom.',
  'welcome.right.foot.attendance': 'Attendance.',
  'welcome.right.foot.marks': 'Marks.',
  'welcome.right.foot.fees': 'Fees.',
  'welcome.right.foot.approvals': 'Approvals.',
}

const hi: Record<string, string> = Object.fromEntries(
  Object.entries(en).map(([k, v]) => [k, `TODO-HI: ${v}`]),
)

const welcome: Bundle = { en, hi }
export default welcome
