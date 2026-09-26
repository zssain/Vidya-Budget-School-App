import type { Bundle } from '../index'

// Phase 16 — Timetable (Principal week view + slot editor + copy week) and the
// teacher "My timetable" phone screen. Hindi is a natural translation pending
// native-speaker review (OWNER-DECISIONS #12); Telugu falls back to English.
const timetable: Bundle = {
  en: {
    'nav.timetable': 'Timetable',
    'nav.calendar': 'Calendar',

    'tt.eyebrow': 'Academics',
    'tt.title': 'Timetable.',
    'tt.sub': 'Every period, every teacher.',
    'tt.class': 'Class',
    'tt.selectClass': 'Select a class',
    'tt.today': 'Today',
    'tt.period': 'Period',
    'tt.empty': 'Free',
    'tt.add': 'Add',
    'tt.print': 'Print',
    'tt.substitutesToday': 'Substitutes today',
    'tt.moduleOff': 'The Classroom module is off. Turn it on in Settings → Modules to use the timetable.',

    'tt.copyWeek': 'Copy week to…',
    'tt.copyTo': 'Copy this week to another class',
    'tt.copyConfirm': 'Copy',
    'tt.copyDone': 'Copied {copied} periods · {skipped} skipped (subject not offered).',

    'tt.editor.title': 'Edit period',
    'tt.editor.slot': '{day} · Period {period}',
    'tt.editor.subject': 'Subject and teacher',
    'tt.editor.pick': 'Choose a subject…',
    'tt.editor.noTeacher': 'no teacher assigned',
    'tt.editor.save': 'Save',
    'tt.editor.clear': 'Clear this period',
    'tt.editor.cancel': 'Cancel',

    'tt.clash.teacher_busy': 'That teacher already has a class in this period.',
    'tt.clash.class_busy': 'This class already has a subject in this period.',
    'tt.clash.teacher_not_assigned': 'That teacher is not assigned to this subject.',
    'tt.clash.generic': 'This slot clashes with the timetable.',

    'tt.d1': 'Mon',
    'tt.d2': 'Tue',
    'tt.d3': 'Wed',
    'tt.d4': 'Thu',
    'tt.d5': 'Fri',
    'tt.d6': 'Sat',

    'tt.mine.title': 'My timetable.',
    'tt.mine.today': 'Today',
    'tt.mine.none': 'No periods on your timetable yet.',
    'tt.mine.noneToday': 'No periods today.',
    'tt.mine.back': 'Back',
  },
  hi: {
    'nav.timetable': 'समय सारणी',
    'nav.calendar': 'कैलेंडर',

    'tt.eyebrow': 'शिक्षा',
    'tt.title': 'समय सारणी।',
    'tt.sub': 'हर घंटी, हर शिक्षक।',
    'tt.class': 'कक्षा',
    'tt.selectClass': 'कक्षा चुनें',
    'tt.today': 'आज',
    'tt.period': 'घंटी',
    'tt.empty': 'खाली',
    'tt.add': 'जोड़ें',
    'tt.print': 'प्रिंट',
    'tt.substitutesToday': 'आज के स्थानापन्न',
    'tt.moduleOff': 'कक्षा मॉड्यूल बंद है। समय सारणी उपयोग करने के लिए सेटिंग्स → मॉड्यूल में इसे चालू करें।',

    'tt.copyWeek': 'सप्ताह कॉपी करें…',
    'tt.copyTo': 'इस सप्ताह को दूसरी कक्षा में कॉपी करें',
    'tt.copyConfirm': 'कॉपी करें',
    'tt.copyDone': '{copied} घंटियाँ कॉपी हुईं · {skipped} छोड़ी गईं (विषय उपलब्ध नहीं)।',

    'tt.editor.title': 'घंटी संपादित करें',
    'tt.editor.slot': '{day} · घंटी {period}',
    'tt.editor.subject': 'विषय और शिक्षक',
    'tt.editor.pick': 'विषय चुनें…',
    'tt.editor.noTeacher': 'कोई शिक्षक नियुक्त नहीं',
    'tt.editor.save': 'सहेजें',
    'tt.editor.clear': 'यह घंटी हटाएँ',
    'tt.editor.cancel': 'रद्द करें',

    'tt.clash.teacher_busy': 'उस शिक्षक की इस घंटी में पहले से कक्षा है।',
    'tt.clash.class_busy': 'इस कक्षा में इस घंटी में पहले से विषय है।',
    'tt.clash.teacher_not_assigned': 'वह शिक्षक इस विषय के लिए नियुक्त नहीं है।',
    'tt.clash.generic': 'यह घंटी समय सारणी से टकराती है।',

    'tt.d1': 'सोम',
    'tt.d2': 'मंगल',
    'tt.d3': 'बुध',
    'tt.d4': 'गुरु',
    'tt.d5': 'शुक्र',
    'tt.d6': 'शनि',

    'tt.mine.title': 'मेरी समय सारणी।',
    'tt.mine.today': 'आज',
    'tt.mine.none': 'अभी आपकी समय सारणी में कोई घंटी नहीं है।',
    'tt.mine.noneToday': 'आज कोई घंटी नहीं।',
    'tt.mine.back': 'वापस',
  },
}

export default timetable
