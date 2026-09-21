import { useState } from 'react';
import { Button } from '../../components/Button.jsx';
import { ChipBar } from '../../components/ChipBar.jsx';
import { Field } from '../../components/Field.jsx';
import { formatRupees } from '../../core/format.js';
import { useT } from '../../core/i18n.jsx';
import { useRouter } from '../../core/router.jsx';
import { useMutation, useQuery } from '../../core/useCommand.js';
import * as commands from '../../api/commands.js';
function AccountForm({ account }) {
  const t = useT();
  const router = useRouter();
  const [amount, setAmount] = useState(String(account.balance));
  const [mode, setMode] = useState('Cash');
  const [reference, setReference] = useState('');
  const [note, setNote] = useState('');
  const mutation = useMutation(commands.collectFee);
  const submit = async (event) => {
    event.preventDefault();
    try {
      const receipt = await mutation.run({
        studentId: account.adm || account.studentId,
        amount: Number(amount),
        mode,
        reference,
        note,
      });
      router.go('receipt', { receiptId: receipt.id });
    } catch {
      // The mutation hook exposes field and form errors below.
    }
  };
  return (
    <form className="card cb stack" onSubmit={submit}>
      <h2>{t('fees.collectSubtitle', { name: account.name, ck: account.ck, roll: account.roll })}</h2>
      <div className="pl">
        <span>{t('fees.totalDue')}</span>
        <b>{formatRupees(account.due)}</b>
      </div>
      <div className="pl">
        <span>{t('fees.alreadyPaid')}</span>
        <b>{formatRupees(account.paid)}</b>
      </div>
      <div className="pl">
        <span>{t('fees.balance')}</span>
        <b>{formatRupees(account.balance)}</b>
      </div>
      <div className="row g8">
        <Button type="button" kind="outline small" onClick={() => setAmount(String(account.balance))}>
          {t('fees.fullBalance')}
        </Button>
        {account.terms > 1 && (
          <Button
            type="button"
            kind="outline small"
            onClick={() => setAmount(String(account.oneTermPayable))}
          >
            {t('fees.oneTerm')}
          </Button>
        )}
      </div>
      <Field
        label={t('fees.amountReceived')}
        inputMode="numeric"
        value={amount}
        onChange={(event) => setAmount(event.target.value)}
        error={mutation.fieldError('amount')}
      />
      <ChipBar
        items={['Cash', 'UPI', 'Cheque'].map((value) => ({ value, label: value }))}
        value={mode}
        onChange={setMode}
      />
      {mode !== 'Cash' && (
        <Field
          label={mode === 'UPI' ? t('fees.upiLabel') : t('fees.chequeLabel')}
          value={reference}
          onChange={(event) => setReference(event.target.value)}
          error={mutation.fieldError('reference')}
        />
      )}
      <Field
        label={t('fees.noteOptional')}
        value={note}
        maxLength="80"
        onChange={(event) => setNote(event.target.value)}
      />
      {mutation.error && !mutation.error.field && (
        <div className="err" role="alert">
          {mutation.error.message}
        </div>
      )}
      <div className="row g8 end">
        <Button type="button" kind="outline" onClick={router.back}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" disabled={mutation.pending}>
          {t('fees.saveAndPrint')}
        </Button>
      </div>
    </form>
  );
}
export function CollectFee({ studentId }) {
  const t = useT();
  const router = useRouter();
  const [selected, setSelected] = useState(studentId || '');
  const [q, setQ] = useState('');
  const picker = useQuery(
    () => (selected ? Promise.resolve(null) : commands.feeRegister({ q, state: 'all' })),
    [selected, q],
  );
  const account = useQuery(
    () => (selected ? commands.getFeeAccount({ studentId: selected }) : Promise.resolve(null)),
    [selected],
  );
  return (
    <section className="view">
      <div className="inner stack">
        <div className="spread">
          <h1>{t('common.collectFee')}</h1>
          <Button kind="quiet" onClick={router.back}>
            {t('common.back')}
          </Button>
        </div>
        {!selected && (
          <div className="card cb stack">
            <Field
              label={t('fees.findStudent')}
              placeholder={t('fees.searchStudent')}
              value={q}
              onChange={(event) => setQ(event.target.value)}
            />
            {picker.data?.list
              .filter((row) => !row.rte)
              .sort((a, b) => b.balance - a.balance)
              .slice(0, 30)
              .map((row) => (
                <button className="btn outline spread" key={row.id} onClick={() => setSelected(row.id)}>
                  <span>
                    <span className="b">{row.name}</span>
                    <br />
                    <small>
                      {row.ck} · {row.adm} · {row.father}
                    </small>
                  </span>
                  <span>{row.balance ? formatRupees(row.balance) : t('fees.pill.paid')}</span>
                </button>
              ))}
          </div>
        )}
        {account.error && <div className="note n-red">{account.error.message}</div>}
        {account.data &&
          (account.data.rte ? (
            <div className="note n-red">{t('fees.rteNoFee')}</div>
          ) : account.data.balance <= 0 ? (
            <div className="note n-green">{t('fees.noBalanceToast')}</div>
          ) : (
            <AccountForm key={selected} account={account.data} />
          ))}
      </div>
    </section>
  );
}
