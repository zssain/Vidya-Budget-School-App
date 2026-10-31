import type { Bundle } from '../index'

// Refused changes on the phone (repair B1): the school PC did not accept a change
// this device sent. Shown on the About screen and as a Teacher-home notice (PWA).
const en: Record<string, string> = {
  'syncp.title': 'Changes not accepted',
  'syncp.intro': 'The school computer did not accept these changes. They are kept here so nothing is lost without you knowing — please check with the office and enter them again if needed.',
  'syncp.none': 'All your changes were accepted.',
  'syncp.noReason': 'No reason was given.',
  'syncp.banner': '{n} change(s) not accepted by the school computer',
  'syncp.bannerSub': 'Tap to see why',
  'syncp.kind.attendance_sheet': 'Attendance',
  'syncp.kind.marks_sheet': 'Marks',
  'syncp.kind.report_remark': 'Report-card remark',
  'syncp.kind.homework_note': 'Homework / note',
  'syncp.kind.staff_attendance': 'Check-in / check-out',
  'syncp.kind.request': 'Leave request',
  'syncp.kind.message': 'Message to a parent',
  'syncp.kind.circular_read': 'Circular marked as read',
  'syncp.kind.other': 'A change',
}

const hi: Record<string, string> = {
  'syncp.title': 'स्वीकार न हुए बदलाव',
  'syncp.intro': 'स्कूल कंप्यूटर ने ये बदलाव स्वीकार नहीं किए। ये यहाँ रखे गए हैं ताकि आपकी जानकारी के बिना कुछ न खोए — कृपया ऑफ़िस से पूछें और ज़रूरत हो तो फिर से दर्ज करें।',
  'syncp.none': 'आपके सभी बदलाव स्वीकार हो गए।',
  'syncp.noReason': 'कोई कारण नहीं बताया गया।',
  'syncp.banner': '{n} बदलाव स्कूल कंप्यूटर ने स्वीकार नहीं किए',
  'syncp.bannerSub': 'कारण देखने के लिए टैप करें',
  'syncp.kind.attendance_sheet': 'हाज़िरी',
  'syncp.kind.marks_sheet': 'अंक',
  'syncp.kind.report_remark': 'रिपोर्ट कार्ड टिप्पणी',
  'syncp.kind.homework_note': 'होमवर्क / नोट',
  'syncp.kind.staff_attendance': 'चेक-इन / चेक-आउट',
  'syncp.kind.request': 'छुट्टी का अनुरोध',
  'syncp.kind.message': 'अभिभावक को संदेश',
  'syncp.kind.circular_read': 'परिपत्र पढ़ा गया',
  'syncp.kind.other': 'एक बदलाव',
}

// Telugu (native-speaker review pending, as for the rest of `te`).
const te: Record<string, string> = {
  'syncp.title': 'అంగీకరించని మార్పులు',
  'syncp.intro': 'స్కూల్ కంప్యూటర్ ఈ మార్పులను అంగీకరించలేదు. మీకు తెలియకుండా ఏదీ పోకుండా ఇవి ఇక్కడ ఉంచబడ్డాయి — దయచేసి ఆఫీసులో అడిగి, అవసరమైతే మళ్లీ నమోదు చేయండి.',
  'syncp.none': 'మీ మార్పులన్నీ అంగీకరించబడ్డాయి.',
  'syncp.noReason': 'కారణం ఇవ్వలేదు.',
  'syncp.banner': '{n} మార్పు(లు) స్కూల్ కంప్యూటర్ అంగీకరించలేదు',
  'syncp.bannerSub': 'కారణం చూడటానికి నొక్కండి',
  'syncp.kind.attendance_sheet': 'హాజరు',
  'syncp.kind.marks_sheet': 'మార్కులు',
  'syncp.kind.report_remark': 'రిపోర్ట్ కార్డ్ వ్యాఖ్య',
  'syncp.kind.homework_note': 'హోంవర్క్ / నోట్',
  'syncp.kind.staff_attendance': 'చెక్-ఇన్ / చెక్-అవుట్',
  'syncp.kind.request': 'సెలవు అభ్యర్థన',
  'syncp.kind.message': 'తల్లిదండ్రులకు సందేశం',
  'syncp.kind.circular_read': 'సర్క్యులర్ చదివినట్లు గుర్తించబడింది',
  'syncp.kind.other': 'ఒక మార్పు',
}

const syncProblems: Bundle = { en, hi, te }
export default syncProblems
