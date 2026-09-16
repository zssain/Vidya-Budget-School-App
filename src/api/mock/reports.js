// Reports. Ported from the prototype.
import * as db from './db.js';

export function reportsSummary() {
  db.need('reports.view');
  const act = db.activeStudents();
  const boys = act.filter((s) => s.gender === 'Male').length;
  const from = db.dateKey(new Date(Date.now() - 30 * 86400e3));
  let p = 0;
  let tt = 0;
  Object.entries(db.state.DB.attendance).forEach(([d, r]) => {
    if (d < from) return;
    Object.values(r).forEach((x) =>
      Object.values(x.marks).forEach((m) => {
        tt++;
        if (m === 'P') p++;
      }),
    );
  });
  const byClass = db.state.DB.classes.map((c) => {
    const l = act.filter((s) => s.cls === c.name);
    const paying = l.filter((s) => !s.rte);
    const due = db.sum(paying, db.feeDue);
    const col = db.sum(paying, (s) => Math.min(db.paidOf(s.adm), db.feeDue(s)));
    return {
      c: c.name,
      n: l.length,
      boys: l.filter((s) => s.gender === 'Male').length,
      girls: l.filter((s) => s.gender === 'Female').length,
      rte: l.filter((s) => s.rte).length,
      due,
      col,
    };
  });
  return {
    enrolment: act.length,
    boys,
    girls: act.length - boys,
    rte: act.filter((s) => s.rte).length,
    aadhaarPct: act.length ? Math.round((act.filter((s) => s.aadhaar).length / act.length) * 100) : null,
    aadhaarPending: act.filter((s) => !s.aadhaar).length,
    attendance30: tt ? Math.round((p / tt) * 100) : null,
    attendanceMarks: tt,
    total: act.length,
    byClass,
    cats: ['General', 'OBC', 'SC', 'ST'].map((c) => ({ c, n: act.filter((s) => s.cat === c).length })),
    feeStatus: ['paid', 'part', 'due', 'rte'].map((k) => ({
      k,
      n: act.filter((s) => db.feeState(s) === k).length,
    })),
  };
}

export function exportClassSummaryXlsx() {
  db.need('reports.view');
  const rows = [['Class', 'Students', 'Boys', 'Girls', 'RTE', 'Fee due', 'Collected', 'Collection %']];
  reportsSummary().byClass.forEach((r) =>
    rows.push([
      r.c,
      r.n,
      r.boys,
      r.girls,
      r.rte,
      r.due,
      r.col,
      r.due ? Math.round((r.col / r.due) * 100) : 100,
    ]),
  );
  return { filename: `class-summary-${db.CODE()}-${db.todayKey()}.csv`, rows };
}
