// Settings (principal only). Ported from the prototype.
import * as db from './db.js';

export function getSettings() {
  db.need('settings.view');
  const DB = db.state.DB;
  return {
    school: { ...DB.school },
    classes: DB.classes.map((c) => ({
      name: c.name,
      sections: c.sections.length,
      sectionList: c.sections,
      tuition: (DB.fees[c.name] || {}).tuition || 0,
      exam: (DB.fees[c.name] || {}).exam || 0,
      other: (DB.fees[c.name] || {}).other || 0,
      termFee: db.termFee(c.name),
      subjects: db.subjectsFor(c.name),
    })),
    transportFee: DB.transportFee,
    terms: DB.terms,
    exams: DB.exams.map((e) => ({ id: e.id, name: e.name, max: e.max })),
    license: {
      schoolCode: DB.license.schoolCode,
      activationCode: DB.license.activationCode,
      type: DB.license.type,
      maxUsers: DB.license.maxUsers,
    },
    deviceId: DB.meta.deviceId,
    deviceCode: DB.meta.deviceCode,
    version: db.APP_VERSION,
    session: DB.school.session,
  };
}

export function saveSchool(input) {
  db.need('settings.edit');
  const name = String(input.name || '').trim();
  const udise = String(input.udise || '').trim();
  if (!name) throw db.userError('School name cannot be empty.', 'name');
  if (udise && !/^\d{11}$/.test(udise)) throw db.userError('A UDISE code has exactly 11 digits.', 'udise');
  Object.assign(db.state.DB.school, {
    name,
    addr: input.addr || '',
    udise,
    board: input.board || 'State Board',
    session: input.session || '',
    phone: input.phone || '',
  });
  db.commit('settings', 'School details updated');
  return getSettings();
}

export function saveClasses({ classes, transportFee, terms }) {
  db.need('settings.edit');
  const DB = db.state.DB;
  const seen = new Set();
  for (const c of classes) {
    const nm = String(c.name).trim();
    if (!/^[A-Za-z0-9 ]{1,12}$/.test(nm))
      throw db.userError('Class names use letters, numbers and spaces only.');
    if (seen.has(nm.toLowerCase())) throw db.userError(`The class ${nm} is listed twice.`);
    seen.add(nm.toLowerCase());
    for (const k of ['tuition', 'exam', 'other'])
      if (!/^\d+$/.test(String(c[k])))
        throw db.userError(`Fee for class ${nm} must be a whole number of rupees.`);
    const subs = Array.isArray(c.subjects)
      ? c.subjects
      : String(c.subjects || '')
          .split(',')
          .map((x) => x.trim())
          .filter(Boolean);
    if (!subs.length) throw db.userError(`Class ${nm} needs at least one subject.`);
    c._name = nm;
    c._subs = [...new Set(subs)];
    c._secs = Array.from({ length: +c.sections }, (_, k) => String.fromCharCode(65 + k));
  }
  if (!/^\d+$/.test(String(transportFee))) throw db.userError('Bus fee must be a whole number of rupees.');
  for (const c of classes) {
    const existing = DB.classes.find((x) => x.name === c._name);
    if (!existing) continue;
    for (const sec of existing.sections.filter((x) => !c._secs.includes(x))) {
      if (DB.students.some((s) => s.status === 'active' && s.cls === c._name && s.sec === sec))
        throw db.userError(
          `Class ${c._name}-${sec} still has students. Move them before removing the section.`,
        );
      if (DB.users.some((u) => u.active && (u.classes || []).includes(c._name + '-' + sec)))
        throw db.userError(`A teacher is assigned to ${c._name}-${sec}. Change their classes first.`);
    }
  }
  DB.classes = classes.map((c) => ({ name: c._name, sections: c._secs }));
  DB.fees = {};
  DB.subjects = {};
  classes.forEach((c) => {
    DB.fees[c._name] = { tuition: +c.tuition, exam: +c.exam, other: +c.other };
    DB.subjects[c._name] = c._subs;
  });
  DB.transportFee = +transportFee;
  DB.terms = +terms;
  db.commit('settings', 'Classes, fees and subjects updated');
  return getSettings();
}

export function saveExams({ exams }) {
  db.need('settings.edit');
  for (const ex of exams) {
    if (!String(ex.name || '').trim()) throw db.userError('Every exam needs a name.');
    if (!/^\d+$/.test(String(ex.max)) || +ex.max < 1 || +ex.max > 500)
      throw db.userError(`Maximum marks for ${ex.name} must be between 1 and 500.`);
    const existing = db.state.DB.exams.find((e) => e.id === ex.id);
    if (existing && +ex.max < +existing.max) {
      const over = Object.values(db.state.DB.marks[existing.id] || {}).some((o) =>
        Object.values(o).some((v) => v !== 'AB' && v > +ex.max),
      );
      if (over) throw db.userError(`Some ${existing.name} marks are above ${ex.max}. Fix them first.`);
    }
  }
  db.state.DB.exams = exams.map((ex) => ({
    id: ex.id || db.uid('ex'),
    name: String(ex.name).trim(),
    max: +ex.max,
  }));
  db.commit('settings', 'Exams updated');
  return getSettings();
}

export function saveDeviceCode({ code }) {
  db.need('settings.edit');
  const c = String(code || '').toUpperCase();
  if (!/^[A-Z0-9]{2,4}$/.test(c))
    throw db.userError('Use 2 to 4 capital letters or digits, for example PC or T1.', 'code');
  const old = db.state.DB.meta.deviceCode;
  db.state.DB.meta.deviceCode = c;
  db.commit('settings', `Receipt prefix changed from ${old} to ${c}`);
  return getSettings();
}
