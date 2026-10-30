// CSV export with spreadsheet formula-injection protection (OWASP "CSV injection").
// Any cell a spreadsheet could interpret as a formula — leading = + - @, tab or
// carriage return (also after leading whitespace) — is prefixed with a single quote
// so it is shown as text. Every cell is then RFC 4180 quoted.

const DANGEROUS = /^\s*[=+\-@\t\r]/;

export function csvCell(value: unknown): string {
  let s: string;
  if (value === null || value === undefined) s = "";
  else if (value instanceof Date) s = value.toISOString();
  else if (Array.isArray(value)) s = value.join(" ");
  else s = String(value);
  if (DANGEROUS.test(s)) s = `'${s}`;
  return `"${s.replace(/"/g, '""')}"`;
}

export function toCsv(header: readonly string[], rows: readonly (readonly unknown[])[]): string {
  const lines = [header.map(csvCell).join(",")];
  for (const r of rows) lines.push(r.map(csvCell).join(","));
  return lines.join("\r\n") + "\r\n";
}
