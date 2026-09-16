// Marks and report cards. Ported from the prototype.
import * as db from './db.js';

function parseMark(v, max) {
  const s = String(v).trim().toUpperCase();
  if (s === '') return { ok: true, v: undefined };
  if (s === 'AB') return { ok: true, v: 'AB' };
  if (!/^\d{1,3}$/.test(s) || +s > max) return { ok: false };
  return { ok: true, v: +s };
}

export function getMarksSheet({ examId, sectionId }) {
  if (!db.visibleCK().includes(sectionId)) throw db.permissionError();
  const ex = db.state.DB.exams.find((e) => e.id === examId) || db.state.DB.exams[0];
  const subjects = db.subjectsFor(db.splitCK(sectionId)[0]);
  const meta = (db.state.DB.marksMeta[ex.id] || {})[sectionId];
  const students = db.inCK(sectionId).map((s) => {
    const m = (db.state.DB.marks[ex.id] || {})[s.adm] || {};
    const marks = {};
    subjects.forEach((sub) => {
      if (m[sub] !== undefined) marks[sub] = String(m[sub]);
    });
    const r = db.examResult(s, ex);
    return {
      adm: s.adm,
      roll: s.roll,
      name: s.name,
      marks,
      total: r.entered ? `${r.got}/${r.max}` : '—',
      grade: r.entered ? r.grade : '—',
    };
  });
  return {
    exam: { id: ex.id, name: ex.name, max: ex.max },
    exams: db.state.DB.exams.map((e) => ({ id: e.id, name: e.name })),
    sectionId,
    subjects,
    students,
    savedBy: meta ? meta.byName : undefined,
    savedAt: meta ? meta.at : undefined,
    editable: db.can('marks.enter', sectionId),
  };
}

export function saveMarks({ examId, sectionId, entries }) {
  db.need('marks.enter', sectionId);
  const ex = db.state.DB.exams.find((e) => e.id === examId);
  if (!ex) throw db.notFound('Exam not found.');
  const list = db.inCK(sectionId);
  const subs = db.subjectsFor(db.splitCK(sectionId)[0]);
  // entries: [{ studentId, subjectId, value }]
  const byStudent = {};
  for (const e of entries) {
    (byStudent[e.studentId] = byStudent[e.studentId] || {})[e.subjectId] = e.value;
  }
  const bad = [];
  list.forEach((s) => {
    const d = byStudent[s.adm] || {};
    subs.forEach((sub) => {
      if (d[sub] !== undefined && !parseMark(d[sub], +ex.max).ok) bad.push(`${s.name} (${sub})`);
    });
  });
  if (bad.length)
    throw db.userError(
      `Fix these marks. They must be 0 to ${ex.max} or AB: ${bad.slice(0, 4).join(', ')}${bad.length > 4 ? '…' : ''}`,
    );
  db.state.DB.marks[ex.id] = db.state.DB.marks[ex.id] || {};
  let filled = 0;
  list.forEach((s) => {
    const d = byStudent[s.adm] || {};
    const out = {};
    subs.forEach((sub) => {
      const p = parseMark(d[sub] !== undefined ? d[sub] : '', +ex.max);
      if (p.v !== undefined) {
        out[sub] = p.v;
        filled++;
      }
    });
    const old = db.state.DB.marks[ex.id][s.adm] || {};
    Object.keys(old).forEach((sub) => {
      if (!subs.includes(sub)) out[sub] = old[sub];
    });
    db.state.DB.marks[ex.id][s.adm] = out;
  });
  (db.state.DB.marksMeta[ex.id] = db.state.DB.marksMeta[ex.id] || {})[sectionId] = {
    by: db.state.session.user.id,
    byName: db.state.session.user.name,
    at: db.nowISO(),
  };
  db.commit('mark', `Saved ${ex.name} marks for ${sectionId} (${filled} entries)`);
  return getMarksSheet({ examId, sectionId });
}

export function reportCardData(s) {
  const subjects = db.subjectsFor(s.cls);
  return {
    school: db.state.DB.school,
    student: { name: s.name, ck: db.ckOf(s), roll: s.roll, adm: s.adm, father: s.father, dob: s.dob },
    subjects,
    exams: db.state.DB.exams.map((ex) => ({ id: ex.id, name: ex.name, max: ex.max })),
    rows: subjects.map((sub) => ({
      subject: sub,
      byExam: db.state.DB.exams.map((ex) => {
        const v = ((db.state.DB.marks[ex.id] || {})[s.adm] || {})[sub];
        return v === undefined ? '—' : v;
      }),
    })),
    totals: db.state.DB.exams.map((ex) => {
      const r = db.examResult(s, ex);
      return { got: r.got, max: r.max, entered: r.entered, pct: r.pct, grade: r.grade };
    }),
    attendance: db.attendanceOf(s.adm),
  };
}

export function getReportCard({ studentId }) {
  const s = db.stuBy(studentId);
  if (!s || !db.can('reportcard.view', db.ckOf(s))) throw db.permissionError();
  return reportCardData(s);
}

export function getClassReportCards({ sectionId }) {
  if (!db.can('reportcard.view', sectionId)) throw db.permissionError();
  return { sectionId, cards: db.inCK(sectionId).map((s) => reportCardData(s)) };
}
