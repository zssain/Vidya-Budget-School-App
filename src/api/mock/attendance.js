// Attendance. Ported from the prototype.
import * as db from './db.js';

function readOnlyReason(sectionId, date) {
  if (date > db.todayKey()) return 'You cannot mark attendance for a future date.';
  if (!db.can('attendance.mark', sectionId)) return 'You can only mark attendance for your own classes.';
  if (db.state.session.user.role === 'teacher' && date !== db.todayKey())
    return "Teachers can change only today's attendance. Ask the principal to correct past days.";
  return '';
}

export function getAttendance({ sectionId, date }) {
  if (!db.visibleCK().includes(sectionId)) throw db.permissionError();
  const saved = (db.state.DB.attendance[date] || {})[sectionId];
  const students = db.inCK(sectionId).map((s) => ({ adm: s.adm, roll: s.roll, name: s.name }));
  return {
    sectionId,
    date,
    students,
    marks: saved ? { ...saved.marks } : {},
    savedBy: saved ? saved.byName : undefined,
    savedAt: saved ? saved.at : undefined,
    readOnlyReason: readOnlyReason(sectionId, date) || undefined,
  };
}

export function saveAttendance({ sectionId, date, marks }) {
  db.need('attendance.mark', sectionId);
  const ro = readOnlyReason(sectionId, date);
  if (ro) throw db.userError(ro);
  const list = db.inCK(sectionId);
  const missing = list.filter((s) => !marks[s.adm]);
  if (missing.length)
    throw db.userError(
      `${missing.length} student${missing.length > 1 ? 's are' : ' is'} not marked yet: ${missing
        .slice(0, 3)
        .map((s) => s.name)
        .join(', ')}${missing.length > 3 ? '…' : ''}`,
    );
  const clean = {};
  list.forEach((s) => (clean[s.adm] = marks[s.adm]));
  const existed = !!(db.state.DB.attendance[date] || {})[sectionId];
  (db.state.DB.attendance[date] = db.state.DB.attendance[date] || {})[sectionId] = {
    marks: clean,
    by: db.state.session.user.id,
    byName: db.state.session.user.name,
    at: db.nowISO(),
  };
  const c = (k) => Object.values(clean).filter((x) => x === k).length;
  db.commit(
    'att',
    `${existed ? 'Corrected' : 'Saved'} attendance ${sectionId} for ${date}: ${c('P')} present, ${c('A')} absent, ${c('L')} leave`,
  );
  return getAttendance({ sectionId, date });
}

export function attendanceRegister({ sectionId, month }) {
  if (!db.can('attendance.mark', sectionId) && db.state.session.user.role !== 'principal')
    throw db.permissionError();
  const [y, m] = month.split('-').map(Number);
  const days = new Date(y, m, 0).getDate();
  const list = db.inCK(sectionId);
  const rows = list.map((s) => {
    let present = 0;
    const cells = [];
    for (let i = 1; i <= days; i++) {
      const k = `${y}-${db.pad(m)}-${db.pad(i)}`;
      const v = (((db.state.DB.attendance[k] || {})[sectionId] || {}).marks || {})[s.adm] || '';
      if (v === 'P') present++;
      cells.push(v);
    }
    return { roll: s.roll, name: s.name, cells, present };
  });
  return {
    sectionId,
    year: y,
    month: m,
    monthName: db.MONTHS[m - 1],
    days,
    rows,
    school: db.state.DB.school,
  };
}
