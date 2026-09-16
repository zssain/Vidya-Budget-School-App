// Collect-fee picker and modal. Shared by home, the student page and the fees
// register. Ported from the prototype.
import { html, render } from '../../core/html.js';
import { t } from '../../core/i18n.js';
import { formatRupees } from '../../core/format.js';
import { openModal, toast } from '../../core/ui.js';
import { rerender } from '../../core/router.js';
import * as commands from '../../api/commands.js';
import { toAppError } from '../../api/errors.js';
import { showReceipt } from './receipt.js';

export async function openCollectPicker(adm) {
  if (adm) return openCollect(adm);
  const { element } = openModal({
    title: t('common.collectFee'),
    subtitle: t('fees.findStudent'),
    body: html`<input class="inp" id="pickQ" placeholder="${t('fees.searchStudent')}" autofocus />
      <div id="pickList" style="margin-top:10px;max-height:52vh;overflow:auto"></div>`,
  });
  const listEl = element.querySelector('#pickList');
  async function refresh() {
    const q = element.querySelector('#pickQ').value;
    let data;
    try {
      data = await commands.feeRegister({ q, state: 'all' });
    } catch (e) {
      return toast(toAppError(e).message, { kind: 'error' });
    }
    const rows = data.list
      .filter((s) => !s.rte)
      .sort((a, b) => b.balance - a.balance)
      .slice(0, 30);
    render(
      listEl,
      rows.length
        ? html`${rows.map(
            (s) =>
              html`<button
                class="btn b-out"
                style="width:100%;justify-content:space-between;margin-bottom:6px;text-align:left"
                data-pick="${s.adm}"
              >
                <span
                  ><b>${s.name}</b><br /><span class="xs mut">${s.ck} | ${s.adm} | ${s.father}</span></span
                >
                <span class="num" style="color:${s.balance ? 'var(--red)' : 'var(--green)'}"
                  >${s.balance ? formatRupees(s.balance) + ' ' + t('fees.pill.due') : t('fees.pill.paid')}</span
                >
              </button>`,
          )}`
        : html`<div class="empty">${t('fees.noStudentFound')}</div>`,
    );
  }
  element.querySelector('#pickQ').addEventListener('input', refresh);
  element.addEventListener('click', (e) => {
    const b = e.target.closest && e.target.closest('[data-pick]');
    if (b) openCollect(b.getAttribute('data-pick'));
  });
  refresh();
}

export async function openCollect(adm) {
  let acc;
  try {
    acc = await commands.getFeeAccount({ studentId: adm });
  } catch (e) {
    return toast(toAppError(e).message, { kind: 'error' });
  }
  if (acc.rte) return toast(t('fees.rteNoFee'), { kind: 'error' });
  if (acc.balance <= 0) return toast(t('fees.noBalanceToast'), { kind: 'error' });
  let mode = 'Cash';
  const { element, close } = openModal({
    title: t('common.collectFee'),
    subtitle: t('fees.collectSubtitle', { name: acc.name, ck: acc.ck, roll: acc.roll }),
    body: html`<div class="pl">
        <span class="k">${t('fees.totalDue')}</span><span class="v num">${formatRupees(acc.due)}</span>
      </div>
      <div class="pl">
        <span class="k">${t('fees.alreadyPaid')}</span
        ><span class="v num" style="color:var(--green)">${formatRupees(acc.paid)}</span>
      </div>
      <div class="pl">
        <span class="k b">${t('fees.balance')}</span
        ><span class="v num" style="color:var(--red);font-size:17px">${formatRupees(acc.balance)}</span>
      </div>
      <div class="row g8 wrap" style="margin:12px 0 4px">
        <button class="btn b-out b-sm" data-ui="full">${t('fees.fullBalance')}</button>
        ${acc.terms > 1 ? html`<button class="btn b-out b-sm" data-ui="oneterm">${t('fees.oneTerm')}</button>` : ''}
      </div>
      <label class="lbl" style="margin-top:10px">${t('fees.amountReceived')}</label>
      <input
        class="inp num"
        id="payAmt"
        inputmode="numeric"
        value="${acc.balance}"
        style="font-size:22px;font-weight:750"
        autofocus
      />
      <label class="lbl" style="margin-top:12px">${t('fees.paidByLabel')}</label>
      <div class="chipbar" id="payModes">
        ${['Cash', 'UPI', 'Cheque'].map((m) => html`<button class="chip ${m === 'Cash' ? 'on' : ''}" data-ui="mode" data-mode="${m}">${m}</button>`)}
      </div>
      <div id="refBox" class="hide">
        <label class="lbl" style="margin-top:12px" id="refLbl">${t('fees.reference')}</label>
        <input class="inp mono" id="payRef" maxlength="30" />
      </div>
      <label class="lbl" style="margin-top:12px">${t('fees.noteOptional')}</label>
      <input class="inp" id="payNote" maxlength="80" placeholder="${t('fees.notePlaceholder')}" />
      <div class="err" id="payErr"></div>`,
    footer: html`<button class="btn b-quiet" data-ui="cancel">${t('common.cancel')}</button>
      <button class="btn b-ok" data-ui="save">${t('fees.saveAndPrint')}</button>`,
  });
  const q = (sel) => element.querySelector(sel);
  element.addEventListener('click', async (e) => {
    const b = e.target.closest && e.target.closest('[data-ui]');
    if (!b) return;
    const act = b.getAttribute('data-ui');
    if (act === 'cancel') return close();
    if (act === 'full') return void (q('#payAmt').value = acc.balance);
    if (act === 'oneterm') return void (q('#payAmt').value = Math.min(acc.balance, acc.oneTerm));
    if (act === 'mode') {
      mode = b.getAttribute('data-mode');
      element
        .querySelectorAll('#payModes .chip')
        .forEach((c) => c.classList.toggle('on', c.getAttribute('data-mode') === mode));
      q('#refBox').classList.toggle('hide', mode === 'Cash');
      q('#refLbl').textContent = mode === 'UPI' ? t('fees.upiLabel') : t('fees.chequeLabel');
      return;
    }
    if (act !== 'save') return;
    const amount = Number(String(q('#payAmt').value).replace(/[,₹\s]/g, ''));
    b.disabled = true;
    try {
      const r = await commands.collectFee({
        studentId: adm,
        amount,
        mode,
        reference: q('#payRef') ? q('#payRef').value : '',
        note: q('#payNote').value,
      });
      close();
      const me = await commands.currentUser().catch(() => null);
      await showReceipt(r.no, { fresh: true, me });
      rerender();
    } catch (ex) {
      b.disabled = false;
      q('#payErr').textContent = toAppError(ex).message;
    }
  });
}
