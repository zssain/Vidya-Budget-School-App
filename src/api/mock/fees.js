// Fees, receipts and day book. Ported from the prototype.
import * as db from './db.js';

function receiptDto(r) {
  const s = db.stuBy(r.adm) || {};
  return {
    no: r.no,
    id: r.no,
    adm: r.adm,
    studentName: s.name || '(deleted)',
    ck: s.cls ? db.ckOf(s) : '',
    roll: s.roll,
    father: s.father,
    amount: r.amount,
    mode: r.mode,
    ref: r.ref,
    note: r.note,
    date: r.date,
    at: r.at,
    byName: r.byName,
    device: r.device,
    balanceAfter: r.balanceAfter,
    amountWords: db.inWords(r.amount),
    school: { ...db.state.DB.school },
    cancelled: r.cancelled
      ? { byName: r.cancelled.byName, at: r.cancelled.at, reason: r.cancelled.reason }
      : null,
  };
}

export function feeTotals() {
  const list = db.activeStudents().filter((s) => !s.rte);
  const due = db.sum(list, db.feeDue);
  const paid = db.sum(list, (s) => Math.min(db.paidOf(s.adm), db.feeDue(s)));
  return {
    due,
    paid,
    pending: Math.max(0, due - paid),
    unpaid: list.filter((s) => db.feeState(s) === 'due').length,
    part: list.filter((s) => db.feeState(s) === 'part').length,
    payingCount: list.length,
    pctCollected: due ? Math.round((paid / due) * 100) : 0,
  };
}

export function feeRegister(filter = {}) {
  db.need('fees.view');
  const st = filter.state || 'due';
  const q = String(filter.q || '').toLowerCase();
  const list = db
    .activeStudents()
    .filter((s) => {
      if (filter.sectionId && filter.sectionId !== 'All' && db.ckOf(s) !== filter.sectionId) return false;
      const fs = db.feeState(s);
      if (st === 'due' && !(fs === 'due' || fs === 'part')) return false;
      if (st === 'paid' && fs !== 'paid') return false;
      if (st === 'rte' && fs !== 'rte') return false;
      return (
        !q ||
        s.name.toLowerCase().includes(q) ||
        s.adm.toLowerCase().includes(q) ||
        s.father.toLowerCase().includes(q) ||
        s.mobile.includes(q)
      );
    })
    .sort((a, b) => db.allCK().indexOf(db.ckOf(a)) - db.allCK().indexOf(db.ckOf(b)) || a.roll - b.roll)
    .map((s) => ({
      id: s.adm,
      adm: s.adm,
      name: s.name,
      ck: db.ckOf(s),
      father: s.father,
      mobile: s.mobile,
      rte: s.rte,
      due: db.feeDue(s),
      paid: db.paidOf(s.adm),
      balance: db.balanceOf(s),
      feeState: db.feeState(s),
    }));
  const today = db.state.DB.receipts.filter((r) => r.date === db.todayKey());
  return {
    totals: feeTotals(),
    collectedToday: db.sum(
      today.filter((r) => !r.cancelled),
      (r) => r.amount,
    ),
    receiptsToday: today.filter((r) => !r.cancelled).length,
    session: db.state.DB.school.session,
    terms: db.state.DB.terms,
    devicePrefix: db.state.DB.meta.deviceCode,
    list,
    today: today.slice().reverse().map(receiptDto),
  };
}

export function getFeeAccount({ studentId }) {
  db.need('fees.view');
  const s = db.stuBy(studentId);
  if (!s) throw db.notFound('Student not found.');
  return {
    studentId: s.adm,
    name: s.name,
    ck: db.ckOf(s),
    roll: s.roll,
    rte: s.rte,
    status: s.status,
    due: db.feeDue(s),
    paid: db.paidOf(s.adm),
    balance: db.balanceOf(s),
    termFee: db.termFee(s.cls),
    terms: db.state.DB.terms,
    transport: s.transport,
    transportFee: db.state.DB.transportFee,
    concession: s.concession || 0,
    oneTerm: db.termFee(s.cls) + (s.transport ? db.state.DB.transportFee : 0),
    oneTermPayable: Math.min(
      db.balanceOf(s),
      db.termFee(s.cls) + (s.transport ? db.state.DB.transportFee : 0),
    ),
    receipts: db.state.DB.receipts
      .filter((r) => r.adm === s.adm)
      .slice()
      .reverse()
      .map(receiptDto),
    previousSessionDues: 0,
  };
}

export function collectFee({ studentId, amount, mode, reference, note }) {
  db.need('fees.collect');
  const s = db.stuBy(studentId);
  if (!s || s.status !== 'active') throw db.userError('This student is not studying in the school.');
  if (s.rte) throw db.userError('This student has an RTE free seat. No fee is charged.');
  const bal = db.balanceOf(s);
  const amt = Number(amount);
  if (!Number.isInteger(amt) || amt <= 0) throw db.userError('Enter the amount in whole rupees.', 'amount');
  if (bal <= 0) throw db.userError('This student has no balance to pay.');
  if (amt > bal) throw db.userError(`That is more than the balance of ${db.rs(bal)}.`, 'amount');
  const ref = String(reference || '');
  if (mode === 'UPI' && !/^\d{12}$/.test(ref))
    throw db.userError('Enter the 12-digit UPI transaction ID from the payment screen.', 'reference');
  if (mode === 'Cheque' && ref.length < 4)
    throw db.userError('Enter the cheque number and bank.', 'reference');
  if (mode === 'UPI' && db.state.DB.receipts.some((r) => r.mode === 'UPI' && r.ref === ref && !r.cancelled))
    throw db.userError('This UPI transaction ID is already on another receipt.', 'reference');
  const pre = db.state.DB.meta.deviceCode;
  const n = (db.state.DB.counters.rcpt[pre] = (db.state.DB.counters.rcpt[pre] || 0) + 1);
  const r = {
    no: `${pre}-${db.pad(n, 4)}`,
    adm: studentId,
    amount: amt,
    mode,
    ref: mode === 'Cash' ? '' : ref,
    note: note || '',
    date: db.todayKey(),
    at: db.nowISO(),
    by: db.state.session.user.id,
    byName: db.state.session.user.name,
    device: pre,
    balanceAfter: bal - amt,
    cancelled: null,
  };
  db.state.DB.receipts.push(r);
  db.commit('fee', `Receipt ${r.no}: ${db.rs(amt)} ${mode} from ${s.name} (${db.ckOf(s)})`);
  return receiptDto(r);
}

export function getReceipt({ receiptId }) {
  db.need('fees.view');
  const r = db.state.DB.receipts.find((x) => x.no === receiptId);
  if (!r) throw db.notFound('Receipt not found.');
  return receiptDto(r);
}

export function cancelReceipt({ receiptId, reason }) {
  if (db.state.session.user.role !== 'principal')
    throw db.userError('Only the principal can cancel a receipt.');
  const r = db.state.DB.receipts.find((x) => x.no === receiptId);
  if (!r || r.cancelled) throw db.userError('This receipt is already cancelled.');
  if (String(reason || '').length < 4) throw db.userError('Write the reason for cancelling.', 'reason');
  r.cancelled = { by: db.state.session.user.id, byName: db.state.session.user.name, at: db.nowISO(), reason };
  const s = db.stuBy(r.adm) || {};
  db.commit('fee', `Cancelled receipt ${r.no} (${db.rs(r.amount)}, ${s.name}). Reason: ${reason}`);
  return receiptDto(r);
}

export function dayBook({ date } = {}) {
  db.need('daybook.view');
  date = date || db.todayKey();
  const list = db.state.DB.receipts.filter((r) => r.date === date);
  const okList = list.filter((r) => !r.cancelled);
  return {
    date,
    modes: ['Cash', 'UPI', 'Cheque'].map((m) => ({
      mode: m,
      total: db.sum(
        okList.filter((r) => r.mode === m),
        (r) => r.amount,
      ),
      count: okList.filter((r) => r.mode === m).length,
    })),
    total: db.sum(okList, (r) => r.amount),
    count: okList.length,
    cancelledCount: list.length - okList.length,
    receipts: list.map(receiptDto),
    school: db.state.DB.school,
  };
}

export function exportDuesXlsx() {
  db.need('fees.view');
  const rows = [
    ['Admission no', 'Name', 'Class', 'Roll', 'Father', 'Mobile', 'Total due', 'Paid', 'Balance'],
  ];
  db.activeStudents()
    .filter((s) => !s.rte && db.balanceOf(s) > 0)
    .sort((a, b) => db.allCK().indexOf(db.ckOf(a)) - db.allCK().indexOf(db.ckOf(b)) || a.roll - b.roll)
    .forEach((s) =>
      rows.push([
        s.adm,
        s.name,
        db.ckOf(s),
        s.roll,
        s.father,
        s.mobile,
        db.feeDue(s),
        db.paidOf(s.adm),
        db.balanceOf(s),
      ]),
    );
  return { filename: `fee-dues-${db.CODE()}-${db.todayKey()}.csv`, rows };
}

export function exportDaybookXlsx({ date } = {}) {
  db.need('daybook.view');
  const book = dayBook({ date });
  const rows = [['Receipt', 'Date', 'Student', 'Class', 'Mode', 'Reference', 'By', 'Amount', 'Status']];
  book.receipts.forEach((r) =>
    rows.push([
      r.no,
      r.date,
      r.studentName,
      r.ck,
      r.mode,
      r.ref,
      r.byName,
      r.amount,
      r.cancelled ? 'Cancelled' : 'OK',
    ]),
  );
  return { filename: `day-book-${db.CODE()}-${book.date}.csv`, rows };
}
