// Home dashboards (one per role). Ported from the prototype.
import * as db from './db.js';
import { feeTotals } from './fees.js';

function todayAttendance() {
  const rec = db.state.DB.attendance[db.todayKey()] || {};
  let p = 0;
  let a = 0;
  let l = 0;
  Object.values(rec).forEach((r) =>
    Object.values(r.marks).forEach((m) => {
      if (m === 'P') p++;
      else if (m === 'A') a++;
      else l++;
    }),
  );
  return { p, a, l, done: Object.keys(rec).length };
}
function activityBrief(c) {
  return { seq: c.seq, at: c.at, kind: c.kind, text: c.text, who: c.who, device: c.device };
}
function receiptBrief(r) {
  const s = db.stuBy(r.adm) || {};
  return {
    no: r.no,
    date: r.date,
    mode: r.mode,
    amount: r.amount,
    byName: r.byName,
    name: s.name || '(deleted)',
    ck: s.cls ? db.ckOf(s) : '',
    cancelled: !!r.cancelled,
  };
}

export function homePrincipal() {
  db.need('reports.view');
  const st = todayAttendance();
  const ft = feeTotals();
  const cks = db.allCK();
  const today = db.todayKey();
  const pendingCK = cks.filter((ck) => !(db.state.DB.attendance[today] || {})[ck] && db.inCK(ck).length);
  const todayRc = db.state.DB.receipts.filter((r) => r.date === today && !r.cancelled);
  return {
    name: db.state.session.user.name,
    students: db.activeStudents().length,
    classes: db.state.DB.classes.length,
    sections: cks.length,
    attendance: st,
    fees: ft,
    backupOverdue: db.backupOverdue(),
    lastBackupAt: db.state.DB.meta.lastBackupAt,
    pending: pendingCK.map((ck) => ({
      ck,
      count: db.inCK(ck).length,
      teachers: db.teachersOf(ck).map((u) => u.name),
    })),
    collectedToday: db.sum(todayRc, (r) => r.amount),
    receiptsToday: todayRc.length,
    recentActivity: db.state.DB.changes
      .filter((c) => c.kind !== 'auth')
      .slice(0, 6)
      .map(activityBrief),
  };
}

export function homeAccountant() {
  db.need('fees.view');
  const ft = feeTotals();
  const today = db.state.DB.receipts.filter((r) => r.date === db.todayKey() && !r.cancelled);
  const byMode = (m) =>
    db.sum(
      today.filter((r) => r.mode === m),
      (r) => r.amount,
    );
  const topDue = db
    .activeStudents()
    .filter((s) => !s.rte && db.balanceOf(s) > 0)
    .sort((a, b) => db.balanceOf(b) - db.balanceOf(a))
    .slice(0, 8)
    .map((s) => ({ adm: s.adm, name: s.name, ck: db.ckOf(s), father: s.father, balance: db.balanceOf(s) }));
  return {
    name: db.state.session.user.name,
    fees: ft,
    collectedToday: db.sum(today, (r) => r.amount),
    receiptsToday: today.length,
    byMode: { Cash: byMode('Cash'), UPI: byMode('UPI'), Cheque: byMode('Cheque') },
    devicePrefix: db.state.DB.meta.deviceCode,
    topDue,
    today: today.slice().reverse().slice(0, 10).map(receiptBrief),
  };
}

export function homeTeacher() {
  const my = db.visibleCK();
  const rec = db.state.DB.attendance[db.todayKey()] || {};
  return {
    name: db.state.session.user.name,
    classes: my.map((ck) => {
      const list = db.inCK(ck);
      const r = rec[ck];
      return {
        ck,
        count: list.length,
        marked: !!r,
        present: r ? Object.values(r.marks).filter((x) => x === 'P').length : 0,
        exams: db.state.DB.exams.map((ex) => ({
          name: ex.name,
          done: list.filter(
            (s) =>
              db.state.DB.marks[ex.id] &&
              db.state.DB.marks[ex.id][s.adm] &&
              Object.keys(db.state.DB.marks[ex.id][s.adm]).length,
          ).length,
          total: list.length,
        })),
      };
    }),
    pending: my.filter((ck) => !rec[ck]),
  };
}
