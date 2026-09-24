# PHASE 15 — FEES INSTALMENTS AND SCHOOL ACCOUNTS: EXPENSES, CASH BOOK, PROFIT, SALARY REGISTER, SCHOOL STORE

## ROLE
You are a senior Rust + React engineer with accounting-software experience, continuing **Vidya Budget School** (v2).

## STANDING RULES (same in every v2 phase)
1. Read IN FULL before anything else: `docs/00-SYSTEM-CONTEXT.md`, `docs/01-MOCK-SPEC.md`,
   `docs/02-V2-CHANGES.md` (the decision record; merged into 00 in Phase 11),
   `docs/03-PROTOTYPE-SPEC.md`, then every file in `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. You are extending a built product. **Read the actual code before changing it.** Never rebuild
   something that exists; extend it. If code and docs disagree, report it and follow the docs;
   if the docs are silent, STOP and ask.
4. Dependencies: only context §13 plus 02-V2-CHANGES §12. Anything else → STOP and ask.
5. Never invent features, rules, copy, numbers, library APIs or Google/Meta/NPCI behaviour.
   Check official docs or installed source. Unsure → STOP.
6. **The mock and the prototype win.** New screens match `design/prototype/VidyaPrototype.jsx`
   exactly, built from the app's existing components and tokens.
7. Business rules only in `crates/vidya-core`; commands thin; the server re-validates every op.
8. Every write = one transaction: row + audit + op (+ voucher and ledger entries for money).
9. Data safety: migrations are additive and numbered, tested on a copy of a v1 database (demo
   seed + fixtures). Never edit or delete a committed migration. Never delete financial or
   audit rows.
10. No fake success; statuses are honest.
11. Never delete or weaken tests. P01–P10 suites must still pass; where 02-V2-CHANGES changes
    behaviour, update the expected values and list every such change in the handoff.
12. Every visible string via `t()` in `en.json`, `hi.json` and (from Phase 13) `te.json`.
13. Zero monthly cost: nothing may require a company server.
14. Run every command you mention and paste real output. Branch `v2/p15`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-15.md`.


## OBJECTIVE
Turn Vidya into the school's complete money book: fee instalments with due dates, expenses with
vouchers and bill photos, a daily cash book, a monthly profit summary, a salary register with
advances and slips, and the optional school store — all on the Phase 13 ledger, all append-only.

## DONE MEANS
- Fee structure with instalments (prototype `feesadmin` state 1); dues generated per instalment;
  preview before changes; paid dues never change.
- Record expense (prototype `accounts` state 2) → voucher V-… → appears in the cash book (prototype
  `accounts` state 1) with correct running balances.
- Profit summary (prototype `accounts` state 3) equals the sum of ledger income − expense accounts.
- Salary register (prototype `salary`) computes net pay with the owner-default formula; Pay creates
  vouchers; slips print.
- School store (prototype `store`) works when the module is on and is invisible when off.
- For every day in the demo seed: cash book "money in" = day book total; debits = credits.

## STEPS

### Step 1 — Instalments
- `fee_head` gains `instalments_json` ([{no, amount_paise, due_date}] or monthly rule for `month`
  frequency). `fee_due` gains `instalment_no`, `due_date` (migration; existing dues get
  `instalment_no = 1`, `due_date` = term start).
- vidya-core `fees.rs`: instalment amounts must sum to the head's yearly amount; due dates inside the
  session; dues generation per instalment; allocation oldest `due_date` first (unchanged principle);
  plan change → `preview_plan_change` lists affected **unpaid** dues only.
- Fees → Fee structure (prototype `feesadmin` state 1): table with instalment chips and dates, "Add fee
  head" sheet, change preview dialog. Dues screen (Phase 14) now shows real instalment labels
  ("Tuition · 2 of 3") and due dates.

### Step 2 — Attachments store (for bill photos)
Local folder `<AppData>/Vidya/attachments/<sha256>` encrypted with a key derived from the DB key;
max 2 MB after compression (photos compressed on device: Android via the plugin, desktop via
canvas; JPEG q70, long edge ≤ 1600 px). Referenced by hash from rows. Travel to the school PC as
separate sealed blobs (finance audience) over LAN (`POST /v1/attachments`, dedup by hash) or Drive
(`exchange/ops-<device>/blobs/<hash>.vbl`). Included in backups. Tests: dedup, tamper, missing blob →
row shows "photo not yet received".

### Step 3 — Expenses
- `expense(id, voucher_id, category_account_id, amount_paise, paid_via cash|upi|bank, details,
  vendor NULL, bill_attachment NULL, created_by, …)` append-only; mistakes → `expense_reversal`
  (reversal voucher) with reason, Principal only.
- vidya-core: amount > 0; category must be an active expense account; cash expense cannot make cash
  in hand negative → warning (not a block) **[OWNER default]**.
- Record expense sheet (prototype `accounts` state 2): category chips (from expense accounts),
  amount input, paid by, details, bill photo, "Voucher V-… is given when you save".
- Permissions: Principal and Accountant record; only Principal reverses.

### Step 4 — Accounts screen: cash book, day book, profit
- Sidebar Finance → **Accounts** (module `accounts`): tabs **Cash book · Day book · Expenses ·
  Profit · Salaries**.
- Opening balance: once per session, Principal enters cash and bank opening amounts → opening voucher.
- Cash book (prototype `accounts` state 1): strip (opening, money in today, money out today, cash +
  bank in hand), table (time, receipt/voucher no., details, in, out, running balance), day picker,
  print.
- Day book: the existing day book moves here unchanged (plus expenses as "out").
- Profit (prototype `accounts` state 3): strip (income, expenses, surplus, fees still due), CSS bar
  chart income vs expenses per month (animation as prototype), table per month; year to date;
  print.

### Step 5 — Salary register
- Tables: `salary_structure(staff_id, monthly_paise, effective_from)`, `staff_advance(id, staff_id,
  amount_paise, voucher_id, recover_per_month_paise, recovered_paise)`, `salary_run(id, month,
  status draft|finalised, finalised_by)`, `salary_line(run_id, staff_id, monthly_paise,
  working_days, days_present, unpaid_leave_days, deduction_paise, advance_recovery_paise,
  net_paise, paid_voucher_id NULL)`.
- vidya-core `salary.rs`: working days from the calendar; deduction = monthly ÷ working days ×
  unpaid days, rounded half-up to the rupee **[OWNER default]**; advance recovery ≤ remaining
  advance; net ≥ 0. Days present come from staff attendance + approved leave once Phase 17 exists;
  until then the Principal enters days per staff (field shown only while HR is not active).
- Screen (prototype `salary`): strip (total salaries, advances recovered, unpaid-leave deducted,
  net to pay), table, "Pay N pending" (creates salary vouchers per staff, cash or bank), "Give
  advance" sheet, "Print salary slips" (print engine, one slip per staff).
- Permissions: Principal only (Accountant may view if the Principal allows — setting, default off).

### Step 6 — School store (optional module `store`)
- Tables: `store_item(id, name, name_hi, name_te, price_paise, stock, low_stock_at, active)`,
  `store_sale(id, receipt_no S-…, student_id NULL, guardian_id NULL, items_json, total_paise,
  mode, voucher_id)` append-only, `stock_move(id, item_id, qty, reason sale|purchase|adjust,
  ref_id, by, at)`.
- vidya-core: stock can't go negative (sale blocked with message); price snapshot on sale.
- Screen (prototype `store`): items grid with stock + low-stock warning, sale sheet (student
  optional), mode, receipt print, stock purchase/adjust (Principal). Ledger: Store income.
- Hidden entirely when the module is off (sidebar, commands, sync).

### Step 7 — Reports
Add to Reports: expenses by category (date range), salary register (month), store sales and stock,
instalment dues by due date. All printable and CSV-exportable (scoped by role).

### Step 8 — Tests
Ledger balance for every voucher kind; cash book running balance; profit = sum of accounts;
salary formula examples (incl. the prototype's R. Nair case: ₹17,000, 26 working days, 2 unpaid →
₹1,308); advance recovery; instalment sum validation; plan-change preview; store stock; module off →
no store rows synced; offline expense with photo → server confirms voucher and blob. Prototype
fidelity for `feesadmin`(1), `accounts`(1–3), `salary`, `store`.

## STOP CONDITIONS
Any accounting rule not written here (GST, TDS, PF/ESI, depreciation, bank reconciliation) → ask.
Any change to existing payment/receipt numbers.

## HANDOFF → `docs/phase-notes/phase-15.md`
Tables · ledger mappings per voucher kind · formulas + owner defaults · attachment store design ·
screens · reports · tests with sample numbers.
