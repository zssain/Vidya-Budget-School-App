// Student list/detail/admission are real Rust commands now (P3.2). Only the
// Excel export is still mock, until its own prompt. TEMPORARY (see docs/prompts/P1.2).
import * as db from './db.js';

export function exportStudentsXlsx() {
  db.need('students.view');
  const rows = [
    [
      'Admission no',
      'Name',
      'Class',
      'Section',
      'Roll',
      'Gender',
      'Date of birth',
      'Father',
      'Mother',
      'Mobile',
      'Category',
      'RTE',
      'Bus',
      'Locality',
      'Aadhaar collected',
      'APAAR',
      'Status',
      'Fee due',
      'Paid',
      'Balance',
    ],
  ];
  db.state.DB.students.forEach((s) =>
    rows.push([
      s.adm,
      s.name,
      s.cls,
      s.sec,
      s.roll,
      s.gender,
      s.dob,
      s.father,
      s.mother,
      s.mobile,
      s.cat,
      s.rte ? 'Yes' : 'No',
      s.transport ? 'Yes' : 'No',
      s.village,
      s.aadhaar ? 'Yes' : 'No',
      s.apaar ? 'Yes' : 'No',
      s.status,
      db.feeDue(s),
      db.paidOf(s.adm),
      db.balanceOf(s),
    ]),
  );
  return { filename: `students-${db.CODE()}-${db.todayKey()}.csv`, rows };
}
