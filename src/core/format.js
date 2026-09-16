// Display formatting for the UI. Pure functions; the current time is always
// passed in (see docs/UI_GUIDE.md "Wording rules"). These mirror the prototype.

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Indian digit grouping: 12345678 -> "1,23,45,678". */
function groupIndian(digits) {
  if (digits.length <= 3) return digits;
  const last3 = digits.slice(-3);
  const rest = digits.slice(0, -3);
  return rest.replace(/\B(?=(\d{2})+(?!\d))/g, ',') + ',' + last3;
}

/** Whole rupees with the ₹ sign and Indian grouping. */
export function formatRupees(n) {
  const v = Math.round(Number(n) || 0);
  const sign = v < 0 ? '-' : '';
  return '₹' + sign + groupIndian(String(Math.abs(v)));
}

function pad2(n) {
  return String(n).padStart(2, '0');
}

/** "2026-09-15" -> "15 Sep 2026". */
export function formatDate(key) {
  if (!key) return '—';
  const [y, m, d] = String(key).split('-');
  return `${+d} ${MONTHS[+m - 1]} ${y}`;
}

/** Local time as "2:30 pm". Accepts an ISO string or a Date. */
export function formatTime(value) {
  if (!value) return '—';
  const d = value instanceof Date ? value : new Date(value);
  let h = d.getHours();
  const m = d.getMinutes();
  const ap = h < 12 ? 'am' : 'pm';
  h = h % 12;
  if (h === 0) h = 12;
  return `${h}:${pad2(m)} ${ap}`;
}

/** "15 Sep 2026, 2:30 pm". Accepts an ISO string or a Date. */
export function formatDateTime(value) {
  if (!value) return '—';
  const d = value instanceof Date ? value : new Date(value);
  const key = `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
  return `${formatDate(key)}, ${formatTime(d)}`;
}

/** Relative time. `now` is passed in (ms epoch or Date) so the function is pure. */
export function timeAgo(iso, now) {
  if (!iso) return 'never';
  const nowMs = now instanceof Date ? now.getTime() : Number(now);
  const s = Math.round((nowMs - new Date(iso).getTime()) / 1000);
  if (s < 60) return 'just now';
  if (s < 3600) return Math.round(s / 60) + ' min ago';
  if (s < 86400) return Math.round(s / 3600) + ' hr ago';
  const d = Math.round(s / 86400);
  return d + (d === 1 ? ' day ago' : ' days ago');
}
