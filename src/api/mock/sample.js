// Deterministic sample school for testing (ported from the prototype).
import * as db from './db.js';

export async function loadSampleSchool() {
  const DB = db.blankDB(db.genDeviceId(), 'vaani', 'DEMO-VAANI');
  DB.school = {
    name: 'Vaani Public School',
    addr: '123, Govind Nagar, Kanpur, Uttar Pradesh 208006',
    udise: '09250100213',
    board: 'U.P. Board',
    session: db.defaultSession(),
    phone: '+91 98765 43210',
  };
  const names = db.DEFAULT_CLASSES.slice(0, 11);
  DB.classes = names.map((n) => ({ name: n, sections: n === 'Nursery' ? ['A'] : ['A', 'B'] }));
  names.forEach((n) => {
    DB.fees[n] = {
      tuition: db.DEFAULT_FEES[n][0],
      exam: db.DEFAULT_FEES[n][1],
      other: db.DEFAULT_FEES[n][2],
    };
    DB.subjects[n] = db.defaultSubjects(n);
  });
  DB.transportFee = 900;
  DB.terms = 3;
  DB.backupCheck = await db.hashPassword('backup123');
  const pw = await db.hashPassword('vidya123');
  const mk = (username, name, role, classes) => ({
    id: db.uid('u'),
    username,
    name,
    role,
    mobile: '98765' + db.pad(Math.floor(Math.random() * 1e5), 5),
    classes,
    ...pw,
    mustChange: false,
    failed: 0,
    locked: false,
    active: true,
    createdAt: db.nowISO(),
    lastLogin: null,
  });
  DB.users.push(
    mk('sunita', 'Sunita Mishra', 'principal', []),
    mk('sierra', "Sierra D'Souza", 'teacher', ['V-A', 'V-B']),
    mk('rakesh', 'Rakesh Verma', 'teacher', ['VI-A', 'VI-B', 'VII-A']),
    mk('anita', 'Anita Gupta', 'accountant', []),
  );
  db.state.DB = DB;

  // Students (deterministic pseudo-random).
  let r = 7;
  const rnd = (n) => {
    r = (r * 9301 + 49297) % 233280;
    return Math.floor((r / 233280) * n);
  };
  const FM = [
    'Aarav',
    'Rohit',
    'Aman',
    'Vivek',
    'Saurabh',
    'Ankit',
    'Nikhil',
    'Deepak',
    'Rahul',
    'Shivam',
    'Harsh',
    'Kunal',
    'Mohit',
    'Piyush',
    'Gaurav',
  ];
  const FF = [
    'Anjali',
    'Pooja',
    'Neha',
    'Priya',
    'Kavita',
    'Sneha',
    'Riya',
    'Divya',
    'Shreya',
    'Nisha',
    'Aarti',
    'Meera',
    'Sakshi',
    'Payal',
    'Khushi',
  ];
  const LN = [
    'Sharma',
    'Verma',
    'Gupta',
    'Yadav',
    'Singh',
    'Tiwari',
    'Pandey',
    'Kushwaha',
    'Nigam',
    'Dixit',
    'Srivastava',
    'Maurya',
    'Sahu',
  ];
  const FA = [
    'Ramesh',
    'Suresh',
    'Mahesh',
    'Dinesh',
    'Rajesh',
    'Vinod',
    'Santosh',
    'Pramod',
    'Manoj',
    'Ashok',
    'Sunil',
    'Anil',
  ];
  const plan = {
    Nursery: 10,
    LKG: 12,
    UKG: 12,
    I: 14,
    II: 14,
    III: 14,
    IV: 12,
    V: 14,
    VI: 12,
    VII: 12,
    VIII: 10,
  };
  names.forEach((cls) => {
    const secs = DB.classes.find((c) => c.name === cls).sections;
    const roll = {};
    for (let i = 0; i < plan[cls]; i++) {
      const boy = rnd(2) === 0;
      const ln = LN[rnd(LN.length)];
      const sec = secs[i % secs.length];
      roll[sec] = (roll[sec] || 0) + 1;
      const cat = ['General', 'OBC', 'OBC', 'SC', 'OBC', 'General', 'ST', 'SC'][rnd(8)];
      DB.students.push({
        adm: 'ADM/' + db.pad(DB.counters.adm++, 4),
        name: (boy ? FM[rnd(FM.length)] : FF[rnd(FF.length)]) + ' ' + ln,
        cls,
        sec,
        roll: roll[sec],
        gender: boy ? 'Male' : 'Female',
        dob: 2021 - names.indexOf(cls) + '-0' + (rnd(9) + 1) + '-' + (10 + rnd(18)),
        father: FA[rnd(FA.length)] + ' ' + ln,
        mother: FF[rnd(FF.length)] + ' ' + ln,
        mobile: '9' + (rnd(9) + 1) + String(10000000 + rnd(89999999)),
        cat,
        rte: cat !== 'General' && rnd(7) === 0,
        transport: rnd(3) === 0,
        village: ['Govind Nagar', 'Kidwai Nagar', 'Barra', 'Naubasta'][rnd(4)],
        concession: 0,
        aadhaar: rnd(10) > 1,
        apaar: rnd(10) > 3,
        status: 'active',
        admittedOn: db.todayKey(),
        leftOn: null,
        leftReason: '',
      });
    }
  });

  // Receipts.
  const acc = DB.users[3];
  const today = new Date();
  DB.students
    .filter((s) => !s.rte)
    .forEach((s) => {
      const k = rnd(8);
      if (k < 3) return;
      const amt = k >= 6 ? db.feeDue(s) : Math.round(db.feeDue(s) / 3 / 100) * 100;
      const d = new Date(today);
      d.setDate(d.getDate() - rnd(20));
      const pre = 'PC';
      const n = (DB.counters.rcpt[pre] = (DB.counters.rcpt[pre] || 0) + 1);
      DB.receipts.push({
        no: pre + '-' + db.pad(n, 4),
        adm: s.adm,
        amount: amt,
        mode: ['Cash', 'Cash', 'UPI', 'Cheque'][rnd(4)],
        ref: '',
        note: '',
        date: db.dateKey(d),
        at: d.toISOString(),
        by: acc.id,
        byName: acc.name,
        device: 'PC',
        balanceAfter: 0,
        cancelled: null,
      });
    });
  db.state._paid = null;

  // Attendance for the last 6 weekdays before today.
  let back = 1;
  let days = 0;
  while (days < 6) {
    const d = new Date(today);
    d.setDate(d.getDate() - back++);
    if (d.getDay() === 0) continue;
    const k = db.dateKey(d);
    DB.attendance[k] = {};
    db.allCK().forEach((ck) => {
      const marks = {};
      db.inCK(ck).forEach(
        (s) => (marks[s.adm] = ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P', 'A', 'L'][rnd(10)]),
      );
      DB.attendance[k][ck] = { marks, by: DB.users[0].id, byName: 'Sunita Mishra', at: d.toISOString() };
    });
    days++;
  }

  // Unit Test 1 marks for all classes.
  DB.marks.ut1 = {};
  DB.marksMeta.ut1 = {};
  DB.students.forEach((s) => {
    DB.marks.ut1[s.adm] = {};
    db.subjectsFor(s.cls).forEach((sub) => (DB.marks.ut1[s.adm][sub] = 8 + rnd(18)));
  });
  db.allCK().forEach(
    (ck) => (DB.marksMeta.ut1[ck] = { by: DB.users[0].id, byName: 'Sunita Mishra', at: db.nowISO() }),
  );
  db.commit('setup', 'Sample school loaded for testing');

  return {
    logins: [
      { role: 'principal', username: 'sunita@vaani' },
      { role: 'teacher', username: 'sierra@vaani', classes: ['V-A', 'V-B'] },
      { role: 'teacher', username: 'rakesh@vaani', classes: ['VI-A', 'VI-B', 'VII-A'] },
      { role: 'accountant', username: 'anita@vaani' },
    ],
    password: 'vidya123',
    backupPassword: 'backup123',
  };
}
