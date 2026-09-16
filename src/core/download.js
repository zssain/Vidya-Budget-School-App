// Browser file downloads (CSV exports show as .csv the school can open in Excel).
// The real desktop app writes files through a Tauri command instead (P3.7).

export function toCsv(rows) {
  return (
    '﻿' +
    rows
      .map((r) =>
        r
          .map((v) => {
            const s = String(v ?? '');
            return /[",\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s;
          })
          .join(','),
      )
      .join('\r\n')
  );
}

export function downloadFile(name, content, mime = 'application/octet-stream') {
  const blob = new Blob([content], { type: mime });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 4000);
}

export function downloadCsv(name, rows) {
  downloadFile(name, toCsv(rows), 'text/csv');
}
