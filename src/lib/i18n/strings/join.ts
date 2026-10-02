import type { Bundle } from '../index'

// iPhone PWA join + onboarding (Phase 20, C2). The invite link opens the PWA; the
// device connects the school's Google Drive, drops a join request, waits for the
// school computer to approve, then sets a local PIN. All copy via t().
const join: Bundle = {
  en: {
    'join.web.title': 'Join your school',
    'join.web.invited': 'You’ve been invited to join {school}.',
    'join.web.no_invite': 'Open the invitation link from your Principal on this iPhone to join.',
    'join.web.connect': 'Connect Google Drive & join',
    'join.web.connecting': 'Connecting to Google Drive…',
    'join.web.waiting': 'Waiting for the school computer to approve — usually under a minute. Keep this screen open.',
    'join.web.setpin_title': 'Create a PIN',
    'join.web.setpin_hint': 'Use a 4–6 digit PIN to unlock Vidya on this iPhone.',
    'join.web.pin': 'PIN',
    'join.web.pin_confirm': 'Confirm PIN',
    'join.web.pin_mismatch': 'The two PINs do not match.',
    'join.web.finish': 'Finish',
    'join.web.retry': 'Try again',
  },
  hi: {
    'join.web.title': 'अपने स्कूल से जुड़ें',
    'join.web.invited': 'आपको {school} से जुड़ने के लिए आमंत्रित किया गया है।',
    'join.web.no_invite': 'जुड़ने के लिए इस iPhone पर प्रधानाचार्य से मिला निमंत्रण लिंक खोलें।',
    'join.web.connect': 'Google Drive कनेक्ट करें और जुड़ें',
    'join.web.connecting': 'Google Drive से कनेक्ट हो रहा है…',
    'join.web.waiting': 'स्कूल कंप्यूटर की स्वीकृति की प्रतीक्षा — आमतौर पर एक मिनट से कम। यह स्क्रीन खुली रखें।',
    'join.web.setpin_title': 'एक PIN बनाएँ',
    'join.web.setpin_hint': 'इस iPhone पर Vidya अनलॉक करने के लिए 4–6 अंकों का PIN चुनें।',
    'join.web.pin': 'PIN',
    'join.web.pin_confirm': 'PIN दोबारा डालें',
    'join.web.pin_mismatch': 'दोनों PIN मेल नहीं खाते।',
    'join.web.finish': 'पूरा करें',
    'join.web.retry': 'फिर से कोशिश करें',
  },
}

export default join
