// About screen (Phase 19, Step 7). Reached from the account button on the phone.
// On the iPhone PWA it also lists the honest limits from §18 / phase-19-spike Q5-Q6
// so staff know exactly what the web app does and does not do. en + hi (Telugu
// falls back to English, as elsewhere in Settings/About). No apostrophes in `en`
// values so the single-quoted string literals stay clean.
import type { Bundle } from '../index'

const en: Record<string, string> = {
  'about.title': 'About',
  'about.back': 'Back',
  'about.product': 'Vidya School Management',
  'about.tagline': 'School management, built to work offline first.',
  'about.version': 'Version',
  'about.support': 'Support',
  'about.website': 'Website',
  'about.developer': 'Developed by Zuhair Hussain',
  'about.copyright': '© 2026 Zuhair Hussain',

  // iPhone PWA honest-limits section (shown only when platform === 'web').
  'about.iphone.title': 'About this iPhone app',
  'about.iphone.intro':
    'This is the Vidya staff app for iPhone. A few things work differently here than on the school computer:',
  'about.iphone.sync.t': 'Syncs through Google Drive',
  'about.iphone.sync.d':
    'Your changes reach the other devices in about a minute through the school Google Drive — not over the local network.',
  'about.iphone.background.t': 'No background sync',
  'about.iphone.background.d':
    'Changes send and arrive only while Vidya is open. Open the app to sync.',
  'about.iphone.photos.t': 'Photos from camera or library',
  'about.iphone.photos.d':
    'When you attach a photo, iPhone lets you take one with the camera or pick from your library.',
  'about.iphone.notifications.t': 'No push notifications yet',
  'about.iphone.notifications.d':
    'Alerts appear inside the app while it is open. iPhone push notifications may come in a later version.',
  'about.iphone.storage.t': 'Your data stays on this iPhone',
  'about.iphone.storage.d':
    'Added to your Home Screen, Vidya keeps your data and does not clear it after 7 days. Only very low iPhone storage could remove it — keep Vidya synced.',
}

const hi: Record<string, string> = {
  'about.title': 'परिचय',
  'about.back': 'वापस',
  'about.product': 'Vidya School Management',
  'about.tagline': 'स्कूल प्रबंधन, पहले ऑफ़लाइन काम करने के लिए बनाया गया।',
  'about.version': 'संस्करण',
  'about.support': 'सहायता',
  'about.website': 'वेबसाइट',
  'about.developer': 'ज़ुहैर हुसैन द्वारा विकसित',
  'about.copyright': '© 2026 Zuhair Hussain',

  'about.iphone.title': 'इस iPhone ऐप के बारे में',
  'about.iphone.intro':
    'यह iPhone के लिए Vidya स्टाफ़ ऐप है। यहाँ कुछ चीज़ें स्कूल के कंप्यूटर से अलग तरह काम करती हैं:',
  'about.iphone.sync.t': 'Google Drive के ज़रिए सिंक',
  'about.iphone.sync.d':
    'आपके बदलाव स्कूल के Google Drive के ज़रिए करीब एक मिनट में दूसरे डिवाइस तक पहुँचते हैं — लोकल नेटवर्क से नहीं।',
  'about.iphone.background.t': 'बैकग्राउंड सिंक नहीं',
  'about.iphone.background.d':
    'बदलाव तभी भेजे और पाए जाते हैं जब Vidya खुला हो। सिंक करने के लिए ऐप खोलें।',
  'about.iphone.photos.t': 'कैमरा या लाइब्रेरी से फ़ोटो',
  'about.iphone.photos.d':
    'फ़ोटो जोड़ते समय iPhone आपको कैमरे से फ़ोटो लेने या लाइब्रेरी से चुनने देता है।',
  'about.iphone.notifications.t': 'अभी पुश सूचनाएँ नहीं',
  'about.iphone.notifications.d':
    'ऐप खुला होने पर सूचनाएँ ऐप के अंदर दिखती हैं। iPhone पुश सूचनाएँ किसी बाद के संस्करण में आ सकती हैं।',
  'about.iphone.storage.t': 'आपका डेटा इसी iPhone पर रहता है',
  'about.iphone.storage.d':
    'होम स्क्रीन पर जोड़ने के बाद Vidya आपका डेटा रखता है और 7 दिन बाद उसे नहीं हटाता। सिर्फ़ iPhone की बहुत कम स्टोरेज ही उसे हटा सकती है — Vidya को सिंक रखें।',
}

const about: Bundle = { en, hi }
export default about
