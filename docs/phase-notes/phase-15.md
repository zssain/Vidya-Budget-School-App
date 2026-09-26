# Phase 15 handoff — Fees instalments & School accounts (expenses, cash book, profit, salary register, School store)

## Start state / environment
- Branch `v2/p15`, cut from `v2/p14` @ `e62f3f4`. HEAD at write time `c8d3043`.
- `git status`: clean; all work committed on `v2/p15` (no push/merge/tag).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**.
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives here;
  prompts in `prompts/`). The `run` skill points at `docs/PROGRESS.md` + a doubled prompt path
  that do not exist — the P15 prompt + `docs/` specs are the authority (as in P11–P14).

## Dependencies check (Standing Rule 4)
No new crates or npm packages. `npm run check:deps` → **31 npm + 34 cargo direct deps, all
within §13**. All money crypto reuses existing crates (`sha2`, `hmac`, `chacha20poly1305`,
`sync::seal`); `cargo tree -i aws-lc-rs` is empty (ring only).

## Commits (one per step; Step 4/5 also carry their screens)
| Commit | Step |
|---|---|
| `c3f00ba` | 1 — fee instalments (plan, dues per instalment, plan-change preview) |
| `6102f95` | 3 — expenses (append-only, voucher on save, Principal-only reversal) + all P15 permissions |
| `2684123` | 4 — Accounts screen: cash book · day book · expenses · profit · opening balance |
| `bd2d863` | 5 — salary register (structures, advances, deduction, pay, slips) |
| `b86b68c` | 6 — optional School store (items, stock, sales, module gating) |
| `f96249a` | 2 — encrypted attachment store for bill photos |
| `317784c` | 7 — reports (expenses/salary/store/instalment dues) + CSV |
| `c8d3043` | 8 — test gaps + opening voucher dated at session start |

## Tables added (migrations 0019–0022, additive & numbered; tested on a v1+P14 DB copy)
- **0019** `fee_head.instalments_json` (TEXT, an `InstalmentPlan`); `fee_due.instalment_no`
  (INTEGER DEFAULT 1) + `fee_due.due_date` (TEXT). Backfill: existing dues get
  `instalment_no=1`, `due_date` = matching term start, else month-first, else the current
  session start. Index on `fee_due(due_date)`.
- **0020** `expense` (voucher_id, category_account_id, amount_paise, paid_via cash|upi|bank,
  details, vendor, **bill_attachment** sha256, spent_on, …) + `expense_reversal`
  (expense_id, voucher_id, reason, approved_by, applied_at). Both **APPEND-ONLY** (triggers,
  like payment/reversal).
- **0021** `salary_structure` (staff_id, monthly_paise, effective_from) · `staff_advance`
  (amount_paise, voucher_id, recover_per_month_paise, recovered_paise, given_on) ·
  `salary_run` (month UNIQUE, status draft|finalised, finalised_by) · `salary_line`
  (run_id, staff_id, monthly/working/present/unpaid/deduction/advance_recovery/net, paid_voucher_id).
- **0022** `store_item` (name/_hi/_te, price_paise, stock, low_stock_at, active) · `store_sale`
  (receipt_no S-…, student_id?, guardian_id?, items_json, total_paise, mode, voucher_id) **APPEND-ONLY** ·
  `stock_move` (item_id, qty signed, reason sale|purchase|adjust, ref_id, by, at).
- `migrations_apply_and_are_idempotent` stays green (now 22 migrations).

## Ledger mappings per voucher kind (all balanced; `ledger_imbalance()` = 0)
The chart of accounts is unchanged (P13's 13 system accounts). paid_via/mode → money account:
cash → `cash`; upi/bank → `bank`; cheque → `cheques`.

| Voucher kind | source_table | Debit | Credit |
|---|---|---|---|
| `receipt` (P13) | payment | money account | `fee_income` |
| `reversal` (fee, P13) | reversal | `fee_income` | money account |
| `opening` (P15) | — | `cash` (+ `bank`) | `opening_equity` |
| `expense` (P15) | expense | category expense account | money account (paid_via) |
| `reversal` (expense, P15) | expense_reversal | money account | category account |
| `advance` (P15) | staff_advance | `staff_advances` | money account |
| `salary` (P15) | salary_line | `salary_expense` (= net + recovery) | `staff_advances` (recovery) + money account (net) |
| `store_sale` (P15) | store_sale | money account (mode) | `store_income` |

Vouchers are only posted for **confirmed** money (Server mode inline, else the server's
idempotent `backfill_vouchers`); `on_device` payments/expenses carry no voucher yet — so
cash-book "money in" = the day book total and Σ debits = Σ credits (test
`cash_book_money_in_equals_day_book_total_and_book_balances_every_day`). The P13 backfill test
now asserts `vouchers == confirmed payments` (was all payments) — the one changed expectation.

## Formulas + owner defaults
- **Instalments (§10.2):** a plan is `List{ instalments:[{no,amount_paise,due_date}] }` or
  `Monthly{ monthly_amount_paise, day_of_month }`. `validate_instalment_plan`: List amounts sum
  to the head total, `no` contiguous `1..=n`, each due date inside the session; Monthly amount
  > 0 and `day_of_month ∈ 1..=28`. Allocation stays **oldest `due_date` first**. `preview_plan_change`
  lists only **unpaid** dues that change/add/remove — **paid dues never change** (enforced;
  applying a plan cancels unpaid dues and regenerates, skipping paid instalments).
- **Expense cash warning [OWNER default]:** a cash expense that would drive cash-in-hand
  negative returns a **warning** (`cash_warning: true`), never a block. Amount > 0; category must
  be an active expense account.
- **Salary deduction [OWNER default, #7]:** `deduction = monthly ÷ working days × unpaid days`,
  **rounded half-up to the whole rupee**. Anchor: **R. Nair ₹17,000, 26 working days, 2 unpaid →
  ₹1,308** (net ₹15,692). Advance recovery ≤ remaining advance AND ≤ earned; net ≥ 0. Working days
  come from the school calendar; days present entered by the Principal until Staff HR (P17).
  Salary is **Principal-only** (Accountant-view is a Principal-granted setting, default off — a
  documented follow-up).
- **Store (§10.3):** stock can never go negative (a bigger sale is blocked); the price is
  snapshotted onto the sale line; low-stock when `stock ≤ low_stock_at`.
- **Opening balance:** once per session (`kind='opening'` exists → rejected). Dated at the
  session start so it is the cash-book opening balance, not "money in today".

## Permissions (§5) — 8 new actions (ALL 44 → 52)
`RecordExpense`, `ReverseExpense`, `ViewAccounts`, `ViewProfit`, `OpeningBalance`, `ManageSalary`
(module `accounts`); `ManageStore`, `RecordStoreSale` (module `store`). Principal: all.
Accountant: `RecordExpense`, `ViewAccounts`, `RecordStoreSale` (record + cash book/day book/
expenses + store sales) — **not** profit, reversal, opening, salary, store management. Teacher:
none. Wired through `Action`/`ALL`/`as_key`/`from_key`/`target_kind_for`/`module_for` and both
`can()` role handlers; `role_matrix` + `every_action_maps_to_a_module` still pass;
`action_all_covers…` updated 44 → **52** (the one changed permission expectation).

## Attachment store design (Step 2)
`src-tauri/attachments.rs`: content-addressed by **SHA-256 of the plaintext**; each blob is
`nonce(12) ‖ ChaCha20-Poly1305(key, plaintext)` (reuses `sync::seal`) at
`<AppData>/Vidya/attachments/<sha256>`. The key is `HMAC-SHA256(db_key, "vidya-attachments-v1")`
so blobs are unreadable without the DB key. `put` (dedup by hash, atomic temp+rename, 2 MB cap),
`has`, `get` (AEAD **and** content-hash verify → tamper detection; missing → `None` so the row
shows "photo not yet received"). Commands `save_attachment` / `read_attachment` (base64,
finance-gated); the expense DTO gains `bill_received` (set from the store). Desktop compression:
`src/lib/image.ts` (canvas → JPEG q70, long edge ≤ 1600) wired into the Record-expense sheet.

**Deferred (cannot build/verify in this sandbox — no Android toolchain, no live LAN/Drive):**
- Android-native compression via the in-repo `vidya-android` plugin (Bitmap → JPEG q70) — the
  plugin still does not exist (Rule-3 discrepancy carried from P14; only `gen/android/` scaffolding).
- Blob transport: LAN `POST /v1/attachments` (dedup by hash) and Drive
  `exchange/ops-<device>/blobs/<hash>.vbl`, sealed for the **finance** audience; and inclusion in
  the daily backup. Designed here; wire + verify on real devices (as P14 deferred live Gmail/WhatsApp).

## Screens (prototype-matched, built from existing components/tokens)
- **Fees → Fee structure** (`FeesScreen` structure tab, prototype `feesadmin` 1): instalment chips
  + due dates per head; the head editor adds a plan mode (none / instalments list / monthly rule)
  with a live change **preview** (affected/added/removed unpaid dues + delta). **Dues** screen now
  shows "Head · N of M" + the due date.
- **Finance → Accounts** (`AccountsScreen`, module `accounts`; nav reuses the `daybook` icon):
  tabs **Cash book · Day book · Expenses · Profit** (+ **Salaries** for Principal). Cash book
  (prototype `accounts` 1): strip (opening / in / out / cash+bank in hand) + running-balance table
  + day picker + print + opening-balance banner. Record expense sheet (prototype `accounts` 2):
  category chips, amount, paid-by, details, vendor, bill photo, "Voucher V-… on save", cash
  warning. Profit (prototype `accounts` 3): strip + CSS bar chart (income vs expense per month,
  `vGrow`) + per-month table. Accountant sees Cash book / Day book / Expenses only.
- **Accounts → Salaries** (`SalaryRegister`, prototype `salary`): strip (total / advances /
  deducted / net) + editable days-present + Pay N pending (cash/bank) + Give advance sheet +
  Print salary slips (A5 `SalarySlipsDoc`).
- **Finance → School store** (`SchoolStoreScreen`, prototype `store`, module `store`): items grid
  with stock + low-stock warning, cart + sale sheet (mode, receipt no.), Principal add-item +
  stock purchase/adjust. **Hidden entirely from the sidebar when the module is off** (AppShell
  filters nav items with a `module` key against `list_modules`); a direct visit shows a
  module-off notice.
- New icons: none in the mock set for accounts/store — reused `daybook` (Accounts) and `receipts`
  (Store), documented like P14's `inbox`-for-circulars (Rule 6).

## Reports (Step 7) — printable + CSV, scoped by role
Reports gains tabs: **Expenses** (by category, date range), **Salary register** (month),
**Store** (stock + sales, module-gated), **Instalment dues** (by due date). CSV via the existing
`export_csv` command with new kinds `expenses` (arg `from|to`), `salary` (arg month), `store_sales`,
`store_stock`, `instalment_dues`, each permission- and module-gated (test
`report_csv_exports_are_scoped_and_written`).

## Sync / modules / audience
`sync/scope::module_of_table`: `expense`/`expense_reversal`/`salary_*` → `accounts`;
`store_item`/`store_sale`/`stock_move` → `store` (default OFF → excluded). All P15 tables added to
`FEE_TABLES` (teachers never receive them) and to the Principal/Accountant snapshots (salary is
Principal-only). Test `store_rows_sync_only_when_the_module_is_on` proves "module off → no store
rows synced". `ledger::backfill_vouchers` now covers expenses, expense-reversals and advances
(confirmed-only).

## Demo seed additions
4 expenses (electricity ₹6,850 · repairs ₹1,200 · stationery ₹4,300 · rent ₹18,000); 4 salary
structures (Anita ₹20,000 · Meena ₹18,000 · Nair ₹17,000 · Suresh ₹16,000) + Meena's ₹2,000
advance; 6 store items (one low-stock). Store items are inert until the `store` module is turned on.

## Tests (all real, this phase)
- `cargo test --workspace --locked` → green: **vidya-core 381 lib + 7 role_matrix**, **app 259 lib**
  (+17 this phase), integration bins green.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo tree -i aws-lc-rs` → empty (ring only). `npm run verify` → green: typecheck; check-hex
  (42 tokens, no new colours); **check-i18n 25 modules, 1210 keys, en/hi in sync**; **check-deps 31
  npm + 34 cargo within §13**; contrast; logs; **vitest 38** (api.ts ↔ commands.json parity).
- Sample-number tests: instalment sum validation (₹4,000×3 = ₹12,000; a ₹11,500 sum rejected);
  plan-change preview (paid inst1 untouched, inst2 400k→500k, inst3 added, inst4-removed, Δ = 0);
  expense voucher balanced (Dr electricity 685000, Cr cash 685000) + Principal-only reversal;
  cash-book money-in = day-book total for every seeded day + Σ debit = Σ credit; profit
  income/expense = ledger account sums; **R. Nair deduction = ₹1,308, net ₹15,692**; advance
  recovery capped; store sale ₹2,450 + 2×₹350, stock 18→17, store income ₹3,150, over-stock blocked;
  module off → no store rows synced; offline expense + photo → confirmed voucher + decryptable blob;
  attachment dedup/tamper/missing/wrong-key; every voucher kind produced and balanced;
  report CSVs scoped by role.

### Changed test expectations (Standing Rule 11)
1. `permissions::…action_all_covers…` `ALL.len()` **44 → 52** (8 new P15 actions).
2. `ledger::backfill_is_balanced_idempotent_and_preserves_money_totals` — asserts **one receipt
   voucher per *confirmed* payment** (the seed's one `on_device` payment now has no voucher; the
   ledger holds confirmed money only). No other existing test value changed.

## Prototype fidelity (deferred, sandbox precedent)
Playwright pixel fidelity for `feesadmin`(1), `accounts`(1–3), `salary`, `store` **cannot be
asserted here** — `tests/e2e/__screens__` baselines are gitignored and only assertable on the
canonical macOS machine (P11/P13/P14 precedent). The screens are built from the mock's tokens
(check-hex passes) and the `data-hl` regions are preserved (`instalments`, `cashstrip`, `addexp`,
`category`, `amount`, `bill`, `chart`, `items`, `cart`, `record`, `deduction`, `pay`). Regenerate
+ assert on the baseline machine.

## Follow-ups (documented, non-blocking)
1. Attachment blob transport (LAN `POST /v1/attachments`, Drive `blobs/<hash>.vbl`, backup
   inclusion) + Android-native photo compression (`vidya-android` plugin) — verify on devices.
2. Instalment plans on **session rollover**: a plan's due dates belong to one session; rollover
   currently regenerates plain term dues (`session.rs`), leaving per-session date regeneration to
   the Fee-structure screen. Add automatic re-dating on rollover.
3. Accountant-view of the salary register (Principal-granted setting, default off).
4. Playwright prototype-fidelity baselines (canonical macOS machine).
5. Hindi/Telugu review of the new `accounts`/`store`/`salary` strings (OWNER-DECISIONS #12); `te`
   falls back to English for the new bundles.

## Stop conditions (Standing Rule / prompt) — none hit
No accounting rule beyond the prompt was invented: no GST/TDS/PF/ESI, no depreciation, no bank
reconciliation. No existing payment/receipt number changed (receipts still `R-…`; new series are
`V-…` vouchers and `S-…` store sales via the existing numbering engine).

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
Unchanged list; P15 builds the stated defaults for **#7 salary formula** (monthly ÷ working days ×
unpaid days, half-up to rupee) and the **cash-expense-negative = warning** default. Also open:
#1 app identifier · #2 price · #6 Telugu logo · #8 leave types · #9 remote check-in · #10 retention ·
#11 UPI QR · #12 native Hindi/Telugu review (now also the P15 strings) · #13 gmail.send · #14 code
signing · #15 GST.
