import { useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { StatCard } from '../../components/StatCard.jsx';
import { formatDate, formatRupees, formatTime } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { usePrint } from '../../core/print.jsx';
import { useRouter } from '../../core/router.jsx';
import { useQuery } from '../../core/useCommand.js';
import { useToast } from '../../core/ui.jsx';
import { downloadCommandResult } from '../../core/download.js';
import * as commands from '../../api/commands.js';
const today = () => new Date().toISOString().slice(0, 10);
function DayBookDocument({ book }) {
  const t = useT();
  return (
    <article className="p-doc">
      <h2>{book.school.name}</h2>
      <h3>{t('fees.dayBookOn', { date: formatDate(book.date) })}</h3>
      <table>
        <thead>
          <tr>
            <th>{t('doc.receiptNo')}</th>
            <th>{t('doc.time')}</th>
            <th>{t('doc.student')}</th>
            <th>{t('doc.class')}</th>
            <th>{t('fees.mode')}</th>
            <th>{t('fees.reference')}</th>
            <th>{t('doc.by')}</th>
            <th>{t('doc.amount')}</th>
          </tr>
        </thead>
        <tbody>
          {book.receipts.map((receipt) => (
            <tr className={receipt.cancelled ? 'cancelled' : ''} key={receipt.no}>
              <td>{receipt.no}</td>
              <td>{formatTime(receipt.at)}</td>
              <td>{receipt.studentName}</td>
              <td>{receipt.ck}</td>
              <td>{receipt.mode}</td>
              <td>{receipt.ref}</td>
              <td>{receipt.byName}</td>
              <td>{formatRupees(receipt.amount)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </article>
  );
}
export function DayBook() {
  const t = useT();
  const router = useRouter();
  const { printElement } = usePrint();
  const { toast } = useToast();
  const [date, setDate] = useState(today);
  const query = useQuery(() => commands.dayBook({ date }), [date]);
  const book = query.data;
  const exportBook = async () => {
    try {
      downloadCommandResult(await commands.exportDaybookXlsx({ date }));
      toast(t('reports.downloaded'), { kind: 'ok' });
    } catch (error) {
      toast(error.message, { kind: 'error' });
    }
  };
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t('fees.dayBookTitle')}</h1>
            {book && <p className="mut">{formatDate(book.date)}</p>}
          </div>
          <Button kind="quiet" onClick={router.back}>
            {t('common.back')}
          </Button>
        </div>
        <input
          className="inp"
          aria-label={t('doc.date')}
          type="date"
          max={today()}
          value={date}
          onChange={(event) => setDate(event.target.value)}
        />
        {book && (
          <>
            <div className="stats">
              {book.modes.map((mode) => (
                <StatCard
                  key={mode.mode}
                  label={mode.mode}
                  value={formatRupees(mode.total)}
                  note={t('fees.nReceipts', { n: mode.count })}
                />
              ))}
              <StatCard
                label={t('fees.total')}
                value={formatRupees(book.total)}
                note={t('fees.nReceipts', { n: book.count })}
              />
            </div>
            <div className="card">
              {book.receipts.length ? (
                book.receipts.map((receipt) => (
                  <button
                    className="feerow"
                    key={receipt.no}
                    onClick={() => router.go('receipt', { receiptId: receipt.id })}
                  >
                    <span className={`grow ${receipt.cancelled ? 'cancelled' : ''}`}>
                      <span className="b">{receipt.studentName}</span>
                      <small>
                        {receipt.no} · {receipt.mode} · {receipt.byName}
                      </small>
                    </span>
                    <strong>{formatRupees(receipt.amount)}</strong>
                  </button>
                ))
              ) : (
                <div className="empty">{t('fees.noReceiptsToday')}</div>
              )}
            </div>
            <div className="row g8 end">
              <Button kind="outline" onClick={exportBook}>
                {t('reports.export')}
              </Button>
              <Button onClick={() => printElement(<DayBookDocument book={book} />)}>
                {t('common.print')}
              </Button>
            </div>
          </>
        )}
      </div>
    </section>
  );
}
