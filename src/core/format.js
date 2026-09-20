const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const pad = (n) => String(n).padStart(2, '0');

export function formatRupees(value) {
  return `₹${Math.round(Number(value) || 0).toLocaleString('en-IN')}`;
}
export function formatDate(value) {
  if (!value) return '—';
  const [year, month, day] = String(value).slice(0, 10).split('-');
  return `${Number(day)} ${MONTHS[Number(month) - 1]} ${year}`;
}
export function formatTime(value) {
  if (!value) return '—';
  const date = value instanceof Date ? value : new Date(value);
  const hour = date.getHours();
  return `${hour % 12 || 12}:${pad(date.getMinutes())} ${hour < 12 ? 'am' : 'pm'}`;
}
export function formatDateTime(value) {
  if (!value) return '—';
  const date = value instanceof Date ? value : new Date(value);
  return `${formatDate(`${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`)}, ${formatTime(date)}`;
}
export function timeAgo(value, now) {
  if (!value) return 'never';
  const seconds = Math.round(
    ((now instanceof Date ? now.getTime() : Number(now)) - new Date(value).getTime()) / 1000,
  );
  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.round(seconds / 60)} min ago`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)} hr ago`;
  const days = Math.round(seconds / 86400);
  return `${days} ${days === 1 ? 'day' : 'days'} ago`;
}
