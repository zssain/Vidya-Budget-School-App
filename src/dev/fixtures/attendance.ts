// Sample data for the Attendance screen, copied verbatim from the mock's
// `<script type="text/x-dc">` constructor (design/screens/Attendance.dc.html,
// docs/01-MOCK-SPEC.md §8). The 34 names are copied exactly; className/date are
// carried through for the header sub-line. v2 (Phase 11): attendance is
// Present/Absent only, so the initial-marks rule is (index >= 30 unmarked;
// 8 and 10 -> 'A'; else 'P') — no Leave — matching the prototype. It lives in the
// component via the `initialMarks()` helper exported here so both stay in one place.

/**
 * A single mark: 'P' present, 'A' absent, '' unmarked. `'L'` (Leave) is retained
 * only so history views can read legacy marks; it is never entered on new sheets.
 */
export type Mark = 'P' | 'A' | 'L' | ''

/** Everything the Attendance screen needs from a fixture / seed. */
export interface AttendanceData {
  /** Class label shown after the title, e.g. "V-A". */
  className: string
  /** Human date shown in the sub-line, e.g. "Wed, 23 Sep". */
  date: string
  /** The 34 student names, in roll order (roll = index + 1). */
  names: string[]
}

// names (verbatim from the mock constructor):
export const attendanceFixture: AttendanceData = {
  className: 'V-A',
  date: 'Wed, 23 Sep',
  names: [
    'Aadhya Sharma',
    'Aarav Gupta',
    'Ananya Reddy',
    'Arjun Yadav',
    'Diya Patel',
    'Ishaan Khan',
    'Kabir Joshi',
    'Meera Nair',
    'Mohammed Faiz',
    'Pooja Verma',
    'Rahul Kumar',
    'Riya Verma',
    'Saanvi Rao',
    'Vivaan Singh',
    'Aditi Mishra',
    'Ayaan Qureshi',
    'Bhavya Jain',
    'Dev Malhotra',
    'Fatima Sheikh',
    'Gaurav Chauhan',
    'Harini Iyer',
    'Ira Kapoor',
    'Karan Mehta',
    'Lakshmi Pillai',
    'Manav Tiwari',
    'Nandini Das',
    'Om Prakash',
    'Prisha Agarwal',
    'Reyansh Bose',
    'Sara Thomas',
    'Tanvi Kulkarni',
    'Uday Rathore',
    'Vanya Saxena',
    'Zoya Ansari',
  ],
}

/**
 * The prototype's initial marks (v2 Present/Absent only), from the constructor:
 *   names.map((_, i) => (i >= 30 ? '' : i === 8 || i === 10 ? 'A' : 'P'))
 */
export function initialMarks(names: string[]): Mark[] {
  return names.map((_, i) => (i >= 30 ? '' : i === 8 || i === 10 ? 'A' : 'P'))
}
