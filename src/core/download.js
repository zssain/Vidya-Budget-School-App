export function toCsv(rows) {
  return `\ufeff${rows
    .map((row) =>
      row
        .map((value) => {
          let text = String(value ?? '');
          if (/^[=+\-@]/.test(text)) text = `'${text}`;
          return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
        })
        .join(','),
    )
    .join('\r\n')}`;
}
export function downloadFile(name, content, mime = 'application/octet-stream') {
  const link = document.createElement('a');
  link.href = URL.createObjectURL(new Blob([content], { type: mime }));
  link.download = name;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 4000);
}
export function downloadCommandResult(result) {
  if (result?.rows && result?.filename) downloadFile(result.filename, toCsv(result.rows), 'text/csv');
  else if (result?.content && result?.filename) downloadFile(result.filename, result.content);
  return result?.path || result?.filename;
}
