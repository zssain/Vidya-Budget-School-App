// Students and admissions. Ported from the prototype.
import * as db from './db.js';

function listItem(s) {
  return {
    id: s.adm,
    adm: s.adm,
    roll: s.roll,
    name: s.name,
    ck: db.ckOf(s),
    cls: s.cls,
    sec: s.sec,
    father: s.father,
    mobile: s.mobile,
    rte: s.rte,
    transport: s.transport,
    status: s.status,
    feeState: db.feeState(s),
    balance: db.balanceOf(s),
    feeDue: db.feeDue(s),
    paid: db.paidOf(s.adm),
  };
}
function receiptBrief(r) {
  return {
    no: r.no,
    date: r.date,
    at: r.at,
    mode: r.mode,
    amount: r.amount,
    byName: r.byName,
    cancelled: !!r.cancelled,
  };
}

export function listStudents(filter = {}) {
  db.need('students.view');
  const cks = db.visibleCK();
  const q = String(filter.q || '').toLowerCase();
  const status = filter.status || 'active';
  const list = db.state.DB.students
    .filter((s) => {
      if (!cks.includes(db.ckOf(s))) return false;
      if (s.status !== status) return false;
      if (filter.sectionId && filter.sectionId !== 'All' && db.ckOf(s) !== filter.sectionId) return false;
      if (!q) return true;
      return (
        s.name.toLowerCase().includes(q) ||
        s.adm.toLowerCase().includes(q) ||
        String(s.roll) === q ||
        s.father.toLowerCase().includes(q) ||
        s.mobile.includes(q)
      );
    })
    .sort((a, b) => db.allCK().indexOf(db.ckOf(a)) - db.allCK().indexOf(db.ckOf(b)) || a.roll - b.roll);
  return list.map(listItem);
}

export function getStudent({ studentId }) {
  db.need('students.view');
  const s = db.stuBy(studentId);
  if (!s || !db.visibleCK().includes(db.ckOf(s))) throw db.notFound('This student is not in your classes.');
  const mStart = db.todayKey().slice(0, 8) + '01';
  return {
    ...s,
    id: s.adm,
    ck: db.ckOf(s),
    feeDue: db.feeDue(s),
    paid: db.paidOf(s.adm),
    balance: db.balanceOf(s),
    feeState: db.feeState(s),
    termFee: db.termFee(s.cls),
    terms: db.state.DB.terms,
    transportFee: db.state.DB.transportFee,
    attendanceSession: db.attendanceOf(s.adm),
    attendanceMonth: db.attendanceOf(s.adm, mStart, db.todayKey()),
    exams: db.state.DB.exams.map((ex) => {
      const r = db.examResult(s, ex);
      return {
        id: ex.id,
        name: ex.name,
        got: r.got,
        max: r.max,
        entered: r.entered,
        grade: r.grade,
        pct: r.pct,
      };
    }),
    receipts: db.state.DB.receipts
      .filter((r) => r.adm === s.adm)
      .slice()
      .reverse()
      .map(receiptBrief),
  };
}

function validateStudentInput(d, isPrincipal) {
  const name = String(d.name || '')
    .replace(/\s+/g, ' ')
    .trim();
  const father = String(d.father || '')
    .replace(/\s+/g, ' ')
    .trim();
  if (name.length < 2) throw db.userError('Enter the student name.', 'name');
  if (!db.state.DB.classes.some((c) => c.name === d.cls && c.sections.includes(d.sec)))
    throw db.userError('Pick a valid class and section.', 'cls');
  if (father.length < 2) throw db.userError("Enter the father's name.", 'father');
  if (!/^[6-9]\d{9}$/.test(String(d.mobile || '')))
    throw db.userError('Parent mobile must be 10 digits starting with 6, 7, 8 or 9.', 'mobile');
  if (d.dob && d.dob > db.todayKey()) throw db.userError('Date of birth cannot be in the future.', 'dob');
  let concession;
  if (d.concession !== undefined && d.concession !== null && d.concession !== '') {
    if (!isPrincipal) throw db.userError('Only the principal can give a fee concession.', 'concession');
    if (!/^\d+$/.test(String(d.concession)))
      throw db.userError('Concession must be a whole number of rupees.', 'concession');
    concession = +d.concession;
  }
  return { name, father, concession };
}

export function addStudent(input) {
  db.need('students.add');
  const isPrincipal = db.state.session.user.role === 'principal';
  const { name, father, concession } = validateStudentInput(input, isPrincipal);
  const dup = db.state.DB.students.find(
    (x) =>
      x.status === 'active' &&
      x.name.toLowerCase() === name.toLowerCase() &&
      x.father.toLowerCase() === father.toLowerCase(),
  );
  if (dup && !input.confirmDuplicate) {
    throw {
      kind: 'conflict',
      messageKey: 'students.possible_duplicate',
      message: `${dup.name}, child of ${dup.father}, is already in class ${db.ckOf(dup)} (${dup.adm}). Add another student anyway?`,
      params: { name: dup.name, father: dup.father, ck: db.ckOf(dup), adm: dup.adm },
    };
  }
  const roll =
    Math.max(
      0,
      ...db.state.DB.students.filter((x) => x.cls === input.cls && x.sec === input.sec).map((x) => x.roll),
    ) + 1;
  const s = {
    adm: 'ADM/' + db.pad(db.state.DB.counters.adm++, 4),
    name,
    father,
    gender: input.gender,
    cls: input.cls,
    sec: input.sec,
    mother: input.mother || '',
    mobile: input.mobile,
    dob: input.dob || '',
    cat: input.cat || 'General',
    village: input.village || '',
    rte: !!input.rte,
    transport: !!input.transport,
    aadhaar: !!input.aadhaar,
    apaar: !!input.apaar,
    concession: concession || 0,
    roll,
    status: 'active',
    admittedOn: db.todayKey(),
    leftOn: null,
    leftReason: '',
  };
  db.state.DB.students.push(s);
  db.commit('stu', `New admission: ${s.name} (${db.ckOf(s)}), ${s.adm}`);
  return getStudent({ studentId: s.adm });
}

export function updateStudent(input) {
  db.need('students.edit');
  const s = db.stuBy(input.studentId);
  if (!s) throw db.notFound('Student not found.');
  const isPrincipal = db.state.session.user.role === 'principal';
  const { name, father, concession } = validateStudentInput(input, isPrincipal);
  const d = {
    name,
    father,
    gender: input.gender,
    cls: input.cls,
    sec: input.sec,
    mother: input.mother || '',
    mobile: input.mobile,
    dob: input.dob || '',
    cat: input.cat || 'General',
    village: input.village || '',
    rte: !!input.rte,
    transport: !!input.transport,
    aadhaar: !!input.aadhaar,
    apaar: !!input.apaar,
  };
  if (concession !== undefined) d.concession = concession;
  if (input.status) {
    d.status = input.status;
    if (input.status === 'left') {
      d.leftOn = input.leftOn || db.todayKey();
      d.leftReason = input.leftReason || '';
    } else {
      d.leftOn = null;
      d.leftReason = '';
    }
  }
  const moved = d.cls !== s.cls || d.sec !== s.sec;
  if (moved)
    d.roll =
      Math.max(
        0,
        ...db.state.DB.students
          .filter((x) => x.cls === d.cls && x.sec === d.sec && x.adm !== s.adm)
          .map((x) => x.roll),
      ) + 1;
  const before = s.status;
  Object.assign(s, d);
  db.commit(
    'stu',
    `Updated ${s.name} (${db.ckOf(s)})${moved ? ', moved to ' + db.ckOf(s) : ''}${before !== s.status ? (s.status === 'left' ? ', marked as left school' : ', marked as studying again') : ''}`,
  );
  return getStudent({ studentId: s.adm });
}

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
