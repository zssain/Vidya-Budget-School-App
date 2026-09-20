import { useT } from '../core/i18n.jsx';
import { formatDate, formatRupees } from '../core/format.js';
export function ReceiptCopy({ receipt = {}, copyLabel }) {
  const t = useT();
  return (
    <article className="p-copy p-doc">
      <h2>{receipt.school?.name}</h2>
      <h3>{t('doc.feeReceipt')}</h3>
      <p>{copyLabel}</p>
      <p>
        {t('doc.receiptNo')}: {receipt.no}
      </p>
      <p>
        {t('doc.date')}: {formatDate(receipt.date)}
      </p>
      <p>
        {t('doc.student')}: {receipt.studentName}
      </p>
      <strong>{formatRupees(receipt.amount)}</strong>
    </article>
  );
}
