import type { Bundle } from '../index'

// Every visible string on the Welcome screen, verbatim from
// design/screens/Welcome.dc.html (docs/01-MOCK-SPEC.md §8). `en` is word-for-word
// the mock; `hi` is the natural-Hindi translation (Phase 8, Part F). The three
// radio-option title/sub strings live under welcome.opt.*.
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
  'welcome.footer.copyright': '© 2026 Zuhair Hussain',
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

const hi: Record<string, string> = {
  'welcome.help': 'सहायता',
  'welcome.eyebrow': 'आपका स्कूल Vidya पर',
  'welcome.encrypted': 'आपके डिवाइसों पर एन्क्रिप्टेड',
  'welcome.h1': 'नमस्ते।',
  'welcome.intro':
    'आप कैसे शुरू करना चाहेंगे? एक्टिवेशन कोड की ज़रूरत सिर्फ़ तभी होती है जब आप नया स्कूल सेट कर रहे हों।',
  'welcome.radiogroup.label': 'कैसे शुरू करें',
  'welcome.opt.setup.title': 'मेरा स्कूल सेट करें',
  'welcome.opt.setup.sub': 'प्रधानाचार्य के लिए · आपकी खरीद के एक्टिवेशन कोड का उपयोग करता है',
  'welcome.opt.join.title': 'मेरे स्कूल में शामिल हों',
  'welcome.opt.join.sub': 'शिक्षकों और लेखाकारों के लिए · आपके आमंत्रण का उपयोग करता है',
  'welcome.opt.recover.title': 'मौजूदा स्कूल पुनर्प्राप्त करें',
  'welcome.opt.recover.sub': 'बैकअप से Vidya को नए कंप्यूटर पर ले जाएँ',
  'welcome.field.codeLabel': 'एक्टिवेशन कोड',
  'welcome.field.codePlaceholder': 'VIDYA-XXXX-XXXX-XXXX',
  'welcome.field.codeHint': 'यह आपकी खरीद ईमेल में और आपके खाता पृष्ठ पर है।',
  'welcome.field.invLabel': 'आमंत्रण लिंक या कोड',
  'welcome.field.invPlaceholder': 'प्रधानाचार्य द्वारा भेजा गया लिंक पेस्ट करें',
  'welcome.field.invHint': 'फ़ोन पर आप आमंत्रण QR कोड भी स्कैन कर सकते हैं।',
  'welcome.recover.title': 'मालिक का सत्यापन आवश्यक',
  'welcome.recover.body':
    'कुछ भी पुनर्स्थापित करने से पहले हम स्कूल के लाइसेंस की पुष्टि करेंगे और आपका बैकअप जाँचेंगे।',
  'welcome.cta.setup': 'एक्टिवेट करें और जारी रखें',
  'welcome.cta.join': 'स्कूल में शामिल हों',
  'welcome.cta.recover': 'पुनर्प्राप्ति शुरू करें',
  'welcome.signin.prompt': 'इस डिवाइस पर पहले से सेट है? ',
  'welcome.signin.link': 'साइन इन करें',
  'welcome.footer.copyright': '© 2026 Zuhair Hussain',
  'welcome.footer.privacy': 'गोपनीयता',
  'welcome.footer.terms': 'शर्तें',
  'welcome.right.eyebrow': 'भारतीय स्कूलों के लिए स्कूल प्रबंधन',
  'welcome.right.h2': 'हर कक्षा। हर रुपया।',
  'welcome.right.h2Italic': 'एक स्पष्ट रिकॉर्ड।',
  'welcome.right.body':
    'उपस्थिति, अंक और फ़ीस पहले फ़ोन पर सहेजे जाते हैं, फिर आपके स्कूल के अपने कंप्यूटर द्वारा पुष्ट किए जाते हैं।',
  'welcome.right.doorCaption': 'हर कक्षा तक एक द्वार।',
  'welcome.right.foot.attendance': 'उपस्थिति।',
  'welcome.right.foot.marks': 'अंक।',
  'welcome.right.foot.fees': 'फ़ीस।',
  'welcome.right.foot.approvals': 'स्वीकृतियाँ।',
}

const welcome: Bundle = { en, hi }
export default welcome
