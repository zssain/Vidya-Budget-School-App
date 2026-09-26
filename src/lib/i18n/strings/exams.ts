import type { Bundle } from '../index'

// Phase 16 — Exam seating & hall tickets (Marks & exams → Seating, prototype
// `exams`). Hindi pending native-speaker review (OWNER-DECISIONS #12).
const exams: Bundle = {
  en: {
    'exams.eyebrow': 'Exams',
    'exams.title': 'Seating plan.',
    'exams.sub': 'Room by room.',
    'exams.seatingTitle': 'Seating & hall tickets',
    'exams.pickExam': 'Choose an exam…',
    'exams.moduleOff': 'The Classroom module is off. Turn it on in Settings → Modules to plan exam seating.',

    'exams.stat.students': 'students',
    'exams.stat.rooms': 'rooms',
    'exams.stat.seated': 'seated',

    'exams.rooms': 'Rooms and invigilators',
    'exams.addRoom': 'Add room',
    'exams.roomName': 'Room name',
    'exams.rows': 'Rows',
    'exams.cols': 'Columns',
    'exams.invigilator': 'Invigilator',
    'exams.noInvigilator': 'No invigilator',
    'exams.save': 'Save',
    'exams.delete': 'Delete',
    'exams.cancel': 'Cancel',
    'exams.capacity': '{n} seats',

    'exams.generate': 'Generate seating',
    'exams.printCharts': 'Seating charts',
    'exams.printTickets': 'Print hall tickets',

    'exams.roomGrid': 'Room grid',
    'exams.board': 'BOARD',
    'exams.generated': '{seated} students seated across {rooms} room(s).',
    'exams.capacityShort': '{room}: {missing} seats short ({needed} needed, {capacity} available).',
    'exams.unpaired': '{n} class(es) had no room — add more rooms.',
    'exams.empty': 'Add rooms, then generate seating.',

    'exams.hallTicket': 'Hall ticket',
    'exams.seat': 'Seat',
    'exams.room': 'Room',
    'exams.roll': 'Roll',
    'exams.signature': 'Principal’s signature',
    'exams.back': 'Back',
  },
  hi: {
    'exams.eyebrow': 'परीक्षाएँ',
    'exams.title': 'बैठक योजना।',
    'exams.sub': 'कमरा दर कमरा।',
    'exams.seatingTitle': 'बैठक और हॉल टिकट',
    'exams.pickExam': 'परीक्षा चुनें…',
    'exams.moduleOff': 'कक्षा मॉड्यूल बंद है। परीक्षा बैठक योजना के लिए सेटिंग्स → मॉड्यूल में इसे चालू करें।',

    'exams.stat.students': 'छात्र',
    'exams.stat.rooms': 'कमरे',
    'exams.stat.seated': 'बैठाए गए',

    'exams.rooms': 'कमरे और निरीक्षक',
    'exams.addRoom': 'कमरा जोड़ें',
    'exams.roomName': 'कमरे का नाम',
    'exams.rows': 'पंक्तियाँ',
    'exams.cols': 'स्तंभ',
    'exams.invigilator': 'निरीक्षक',
    'exams.noInvigilator': 'कोई निरीक्षक नहीं',
    'exams.save': 'सहेजें',
    'exams.delete': 'हटाएँ',
    'exams.cancel': 'रद्द करें',
    'exams.capacity': '{n} सीटें',

    'exams.generate': 'बैठक बनाएँ',
    'exams.printCharts': 'बैठक चार्ट',
    'exams.printTickets': 'हॉल टिकट प्रिंट करें',

    'exams.roomGrid': 'कमरे का ग्रिड',
    'exams.board': 'बोर्ड',
    'exams.generated': '{rooms} कमरों में {seated} छात्र बैठाए गए।',
    'exams.capacityShort': '{room}: {missing} सीटें कम ({needed} चाहिए, {capacity} उपलब्ध)।',
    'exams.unpaired': '{n} कक्षा(ओं) के लिए कोई कमरा नहीं — और कमरे जोड़ें।',
    'exams.empty': 'कमरे जोड़ें, फिर बैठक बनाएँ।',

    'exams.hallTicket': 'हॉल टिकट',
    'exams.seat': 'सीट',
    'exams.room': 'कमरा',
    'exams.roll': 'क्रमांक',
    'exams.signature': 'प्रधानाचार्य के हस्ताक्षर',
    'exams.back': 'वापस',
  },
}

export default exams
