// Recover-an-existing-school screen (restore, §12). en + hi (te falls back, as in
// the rest of this bundle set). No apostrophes in `en` values so the single-quoted
// literals stay clean.
import type { Bundle } from '../index'

const en: Record<string, string> = {
  'recover.back': '← Back',
  'recover.title': 'Recover a school',
  'recover.intro':
    'Restore a school on this computer from a Vidya backup file (.vbak) and your recovery key. The other devices will be asked to rejoin.',
  'recover.step1': '1. Choose the backup',
  'recover.chooseFile': 'Choose a backup file (.vbak)',
  'recover.fromDrive': 'Restore from Google Drive',
  'recover.driveSignin': 'Opening Google…',
  'recover.driveListTitle': 'Backups in Google Drive',
  'recover.driveEmpty': 'No backups found in this Google account.',
  'recover.useThis': 'Use this',
  'recover.noFile': 'No backup chosen yet.',
  'recover.step2': '2. Enter your recovery key',
  'recover.keyPlaceholder': 'XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX',
  'recover.check': 'Check this backup',
  'recover.checking': 'Checking…',
  'recover.fromDate': 'Backup from {date}',
  'recover.students': 'Students',
  'recover.payments': 'Payments',
  'recover.lastReceipt': 'Last receipt',
  'recover.integrity': 'Integrity check',
  'recover.chainOk': 'Verified',
  'recover.chainBad': 'Warning: could not verify',
  'recover.staleWarn': 'This backup is from {date} — anything changed since then is not in it.',
  'recover.chooseAnother': 'Choose another',
  'recover.restoreBtn': 'Restore this school',
  'recover.restoring': 'Restoring…',
  'recover.fenceNote':
    'Restoring makes this the main computer. Other devices will need to rejoin, and the previous computer stops being the server. Your current data (if any) is kept as a safety copy.',
}

const hi: Record<string, string> = {
  'recover.back': '← वापस',
  'recover.title': 'स्कूल पुनर्प्राप्त करें',
  'recover.intro':
    'इस कंप्यूटर पर Vidya बैकअप फ़ाइल (.vbak) और अपनी रिकवरी कुंजी से स्कूल पुनर्स्थापित करें। बाकी डिवाइसों को फिर से जुड़ने के लिए कहा जाएगा।',
  'recover.step1': '1. बैकअप चुनें',
  'recover.chooseFile': 'बैकअप फ़ाइल चुनें (.vbak)',
  'recover.fromDrive': 'Google Drive से पुनर्स्थापित करें',
  'recover.driveSignin': 'Google खोला जा रहा है…',
  'recover.driveListTitle': 'Google Drive में बैकअप',
  'recover.driveEmpty': 'इस Google खाते में कोई बैकअप नहीं मिला।',
  'recover.useThis': 'इसे चुनें',
  'recover.noFile': 'अभी कोई बैकअप नहीं चुना।',
  'recover.step2': '2. अपनी रिकवरी कुंजी दर्ज करें',
  'recover.keyPlaceholder': 'XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX',
  'recover.check': 'यह बैकअप जाँचें',
  'recover.checking': 'जाँच हो रही है…',
  'recover.fromDate': '{date} का बैकअप',
  'recover.students': 'विद्यार्थी',
  'recover.payments': 'भुगतान',
  'recover.lastReceipt': 'अंतिम रसीद',
  'recover.integrity': 'अखंडता जाँच',
  'recover.chainOk': 'सत्यापित',
  'recover.chainBad': 'चेतावनी: सत्यापित नहीं हो सका',
  'recover.staleWarn': 'यह बैकअप {date} का है — उसके बाद के बदलाव इसमें नहीं हैं।',
  'recover.chooseAnother': 'दूसरा चुनें',
  'recover.restoreBtn': 'यह स्कूल पुनर्स्थापित करें',
  'recover.restoring': 'पुनर्स्थापित हो रहा है…',
  'recover.fenceNote':
    'पुनर्स्थापना इस कंप्यूटर को मुख्य कंप्यूटर बना देती है। बाकी डिवाइसों को फिर से जुड़ना होगा, और पिछला कंप्यूटर सर्वर नहीं रहेगा। आपका मौजूदा डेटा (यदि कोई हो) सुरक्षित प्रति के रूप में रखा जाता है।',
}

const recover: Bundle = { en, hi }
export default recover
