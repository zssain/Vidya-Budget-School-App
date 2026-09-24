import type { Bundle } from '../index'

// Settings screen (docs §6.2, prompts/P08 Part E). Real controls only — every
// section here is wired to a real command or a real navigation target.
const en: Record<string, string> = {
  'settings.title': 'Settings',
  'settings.subtitle': 'Manage your school, appearance and data.',

  'settings.school.title': 'School',
  'settings.school.name': 'Name',
  'settings.school.board': 'Board',
  'settings.school.session': 'Session',
  'settings.school.phone': 'Phone',

  'settings.appearance.title': 'Appearance',
  'settings.appearance.accent': 'Accent colour',

  'settings.language.title': 'Language',
  'settings.language.hint': 'Changes the app language everywhere.',

  'settings.academics.title': 'Academics',
  'settings.academics.gradeScale': 'Grade scale',
  'settings.academics.gradeScaleSub': 'Bands, cut-offs and grade points',
  'settings.academics.session': 'Session & terms',
  'settings.academics.sessionSub': 'Current year and term dates',

  'settings.data.title': 'Backups & data',
  'settings.data.backups': 'Backups',
  'settings.data.backupsSub': 'Protect your school’s data',
  'settings.data.sync': 'Sync & devices',
  'settings.data.syncSub': 'This computer and connected devices',

  'settings.security.title': 'Security',
  'settings.security.encrypted': 'Your data is encrypted on this computer.',
  'settings.security.chainOk': 'Records verified · the tamper-evident log is intact.',
  'settings.security.chainBad': 'Warning: the tamper-evident log failed verification (seq {seq}).',
  'settings.security.chainChecking': 'Checking records…',

  'settings.licence.title': 'Licence',
  'settings.licence.status': 'Status',
  'settings.licence.active': 'Active',
  'settings.licence.revoked': 'Revoked',
  'settings.licence.moved': 'Moved to another computer',
  'settings.licence.note': 'One-time purchase · perpetual · no expiry.',

  'settings.about.title': 'About',
  'settings.about.version': 'Version',
  'settings.about.product': 'Vidya Budget School',
  'settings.about.tagline': 'School management, built to work offline first.',

  'settings.open': 'Open',

  // Backups screen
  'backups.title': 'Backups',
  'backups.subtitle': 'Keep your school’s data safe.',
  'backups.how.title': 'How your data is protected',
  'backups.how.1': 'Once enabled, Vidya backs up automatically every day.',
  'backups.how.2': 'Backups are encrypted with your recovery key.',
  'backups.how.3': 'Kept on this computer, and on Google Drive when connected.',
  'backups.restore.title': 'Restore to a new computer',
  'backups.restore.body': 'On a new PC, open Vidya → Recover and enter your recovery key to restore from a backup.',
  'backups.history.title': 'Backup history',
  'backups.history.empty': 'No backups yet.',
  'backups.history.note': 'Automatic daily backups and a manual “Back up now” are being finalized — the backup engine is built and tested; enabling it safely is the next step.',
}

const hi: Record<string, string> = {
  'settings.title': 'सेटिंग्स',
  'settings.subtitle': 'अपना स्कूल, दिखावट और डेटा प्रबंधित करें।',

  'settings.school.title': 'स्कूल',
  'settings.school.name': 'नाम',
  'settings.school.board': 'बोर्ड',
  'settings.school.session': 'सत्र',
  'settings.school.phone': 'फ़ोन',

  'settings.appearance.title': 'दिखावट',
  'settings.appearance.accent': 'एक्सेंट रंग',

  'settings.language.title': 'भाषा',
  'settings.language.hint': 'ऐप की भाषा हर जगह बदलती है।',

  'settings.academics.title': 'शैक्षणिक',
  'settings.academics.gradeScale': 'ग्रेड स्केल',
  'settings.academics.gradeScaleSub': 'बैंड, कट-ऑफ़ और ग्रेड पॉइंट',
  'settings.academics.session': 'सत्र और टर्म',
  'settings.academics.sessionSub': 'वर्तमान वर्ष और टर्म की तिथियाँ',

  'settings.data.title': 'बैकअप और डेटा',
  'settings.data.backups': 'बैकअप',
  'settings.data.backupsSub': 'अपने स्कूल का डेटा सुरक्षित रखें',
  'settings.data.sync': 'सिंक और डिवाइस',
  'settings.data.syncSub': 'यह कंप्यूटर और जुड़े डिवाइस',

  'settings.security.title': 'सुरक्षा',
  'settings.security.encrypted': 'आपका डेटा इस कंप्यूटर पर एन्क्रिप्टेड है।',
  'settings.security.chainOk': 'रिकॉर्ड सत्यापित · छेड़छाड़-रोधी लॉग सुरक्षित है।',
  'settings.security.chainBad': 'चेतावनी: छेड़छाड़-रोधी लॉग सत्यापन विफल (seq {seq})।',
  'settings.security.chainChecking': 'रिकॉर्ड जाँचे जा रहे हैं…',

  'settings.licence.title': 'लाइसेंस',
  'settings.licence.status': 'स्थिति',
  'settings.licence.active': 'सक्रिय',
  'settings.licence.revoked': 'रद्द',
  'settings.licence.moved': 'दूसरे कंप्यूटर पर स्थानांतरित',
  'settings.licence.note': 'एकमुश्त खरीद · स्थायी · कोई समाप्ति नहीं।',

  'settings.about.title': 'परिचय',
  'settings.about.version': 'संस्करण',
  'settings.about.product': 'Vidya Budget School',
  'settings.about.tagline': 'स्कूल प्रबंधन, ऑफ़लाइन-पहले काम करने के लिए बना।',

  'settings.open': 'खोलें',

  // Backups screen
  'backups.title': 'बैकअप',
  'backups.subtitle': 'अपने स्कूल का डेटा सुरक्षित रखें।',
  'backups.how.title': 'आपका डेटा कैसे सुरक्षित रहता है',
  'backups.how.1': 'सक्षम होने पर, Vidya हर दिन अपने आप बैकअप लेता है।',
  'backups.how.2': 'बैकअप आपकी रिकवरी कुंजी से एन्क्रिप्ट होते हैं।',
  'backups.how.3': 'इस कंप्यूटर पर, और कनेक्ट होने पर Google Drive पर रखे जाते हैं।',
  'backups.restore.title': 'नए कंप्यूटर पर पुनर्स्थापित करें',
  'backups.restore.body': 'नए PC पर, Vidya → रिकवर खोलें और बैकअप से पुनर्स्थापित करने के लिए अपनी रिकवरी कुंजी दर्ज करें।',
  'backups.history.title': 'बैकअप इतिहास',
  'backups.history.empty': 'अभी कोई बैकअप नहीं।',
  'backups.history.note': 'स्वचालित दैनिक बैकअप और मैन्युअल “अभी बैकअप लें” अंतिम रूप दिए जा रहे हैं — बैकअप इंजन बना और परखा हुआ है; इसे सुरक्षित रूप से सक्षम करना अगला कदम है।',
}

const settings: Bundle = { en, hi }
export default settings
