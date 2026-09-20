import { useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { Field } from '../../components/Field.jsx';
import { ReceiptCopy } from '../../components/ReceiptCopy.jsx';
import { formatDateTime, formatRupees } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { usePrint } from '../../core/print.jsx';
import { useRouter } from '../../core/router.jsx';
import { useCurrentUser } from '../../core/session.jsx';
import { useMutation, useQuery } from '../../core/useCommand.js';
import { useToast } from '../../core/ui.jsx';
import * as commands from '../../api/commands.js';
function Copies({ receipt }) {
  const t = useT();
  return (
    <div>
      <ReceiptCopy receipt={receipt} copyLabel={t('fees.parentCopy')} />
      <div className="p-cut">{t('fees.cutHere')}</div>
      <ReceiptCopy receipt={receipt} copyLabel={t('fees.schoolCopy')} />
    </div>
  );
}
export function Receipt({ receiptId }) {
  const t = useT();
  const router = useRouter();
  const { can } = useCurrentUser();
  const { printElement } = usePrint();
  const { toast } = useToast();
  const [cancelling, setCancelling] = useState(false);
  const [reason, setReason] = useState('');
  const query = useQuery(() => commands.getReceipt({ receiptId }), [receiptId]);
  const mutation = useMutation(commands.cancelReceipt);
  const receipt = query.data;
  const cancel = async () => {
    try {
      await mutation.run({ receiptId, reason });
      toast(t('fees.receiptCancelled'), { kind: 'ok' });
      setCancelling(false);
      await query.reload();
    } catch {
      // The mutation hook exposes the cancellation error beside the field.
    }
  };
  if (query.loading && !receipt) return null;
  if (query.error)
    return (
      <section className="view">
        <div className="note n-red">{query.error.message}</div>
      </section>
    );
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <div>
            <h1>{t('fees.receiptTitle', { no: receipt.no })}</h1>
            <p className="mut">{formatDateTime(receipt.at)}</p>
          </div>
          <Button kind="quiet" onClick={router.back}>
            {t('common.back')}
          </Button>
        </div>
        {receipt.cancelled && (
          <div className="note n-red">
            {t('fees.cancelledBy', {
              name: receipt.cancelled.byName,
              at: formatDateTime(receipt.cancelled.at),
              reason: receipt.cancelled.reason,
            })}
          </div>
        )}
        <div className="receipt card">
          <div className="rline">
            <span>{t('fees.student')}</span>
            <b>
              {receipt.studentName} ({receipt.ck})
            </b>
          </div>
          <div className="rline">
            <span>{t('fees.paidBy')}</span>
            <span>
              {receipt.mode}
              {receipt.ref ? ` · ${receipt.ref}` : ''}
            </span>
          </div>
          <div className="rline">
            <span>{t('fees.receivedBy')}</span>
            <span>
              {receipt.byName} · {receipt.device}
            </span>
          </div>
          <div className="rline">
            <span>{t('fees.balanceAfter')}</span>
            <span>{formatRupees(receipt.balanceAfter)}</span>
          </div>
          <div className="rtot">
            <span>{t('fees.amount')}</span>
            <strong>{formatRupees(receipt.amount)}</strong>
          </div>
          <p>{t('fees.rupeesWords', { words: receipt.amountWords })}</p>
        </div>
        {cancelling && (
          <div className="card cb stack">
            <Field
              label={t('fees.reasonLabel')}
              placeholder={t('fees.reasonPlaceholder')}
              value={reason}
              onChange={(event) => setReason(event.target.value)}
              error={mutation.fieldError('reason')}
            />
            <div className="row g8">
              <Button kind="outline" onClick={() => setCancelling(false)}>
                {t('common.back')}
              </Button>
              <Button kind="warn" onClick={cancel}>
                {t('fees.cancelReceipt')}
              </Button>
            </div>
          </div>
        )}
        <div className="row g8 end">
          {can('fees.cancel_receipt') && !receipt.cancelled && !cancelling && (
            <Button kind="quiet" onClick={() => setCancelling(true)}>
              {t('fees.cancelReceipt')}
            </Button>
          )}
          <Button onClick={() => printElement(<Copies receipt={receipt} />)}>{t('fees.print2')}</Button>
        </div>
      </div>
    </section>
  );
}
