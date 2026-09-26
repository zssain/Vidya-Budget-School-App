import type { Bundle } from '../index'

// Phase 16 — Homework & class notes (teacher phone, prototype `notes`). Hindi is a
// natural translation pending native-speaker review (OWNER-DECISIONS #12); Telugu
// falls back to English.
const notes: Bundle = {
  en: {
    'notes.title': 'Homework & notes.',
    'notes.pick': 'Class and subject',
    'notes.pickClass': 'Choose a class and subject…',
    'notes.homework': 'Homework',
    'notes.classnotes': 'Class notes',
    'notes.text': 'What should students do?',
    'notes.attach': 'Add photo or PDF',
    'notes.warning': 'Photos are made smaller for mobile data. Study material only — no student photos or marks.',
    'notes.share': 'Share with parents',
    'notes.tooBig': 'Files are too large (max 10 MB each, 20 MB total).',
    'notes.needContent': 'Add some text or a file first.',
    'notes.back': 'Back',

    'notes.shareTo': 'Share to',
    'notes.whatsapp': 'WhatsApp',
    'notes.email': 'Email',
    'notes.save': 'Save in Vidya',
    'notes.saved': 'Saved in Vidya.',
    'notes.emailQueued': 'Queued {queued} emails · {skip} skipped.',
    'notes.waOpened': 'WhatsApp opened — pick the class group.',

    'notes.history': 'Recent for this class',
    'notes.empty': 'No homework or notes yet.',
    'notes.delete': 'Delete',
    'notes.deleteHint': 'You can delete your own within 24 hours.',
  },
  hi: {
    'notes.title': 'गृहकार्य और नोट्स।',
    'notes.pick': 'कक्षा और विषय',
    'notes.pickClass': 'कक्षा और विषय चुनें…',
    'notes.homework': 'गृहकार्य',
    'notes.classnotes': 'कक्षा नोट्स',
    'notes.text': 'छात्रों को क्या करना है?',
    'notes.attach': 'फ़ोटो या PDF जोड़ें',
    'notes.warning': 'मोबाइल डेटा के लिए फ़ोटो छोटी की जाती हैं। केवल अध्ययन सामग्री — छात्रों की फ़ोटो या अंक नहीं।',
    'notes.share': 'अभिभावकों के साथ साझा करें',
    'notes.tooBig': 'फ़ाइलें बहुत बड़ी हैं (प्रत्येक अधिकतम 10 MB, कुल 20 MB)।',
    'notes.needContent': 'पहले कुछ टेक्स्ट या फ़ाइल जोड़ें।',
    'notes.back': 'वापस',

    'notes.shareTo': 'साझा करें',
    'notes.whatsapp': 'व्हाट्सएप',
    'notes.email': 'ईमेल',
    'notes.save': 'Vidya में सहेजें',
    'notes.saved': 'Vidya में सहेजा गया।',
    'notes.emailQueued': '{queued} ईमेल कतार में · {skip} छोड़े गए।',
    'notes.waOpened': 'व्हाट्सएप खुला — कक्षा समूह चुनें।',

    'notes.history': 'इस कक्षा के लिए हाल के',
    'notes.empty': 'अभी कोई गृहकार्य या नोट्स नहीं।',
    'notes.delete': 'हटाएँ',
    'notes.deleteHint': 'आप अपने बनाए 24 घंटे के भीतर हटा सकते हैं।',
  },
}

export default notes
