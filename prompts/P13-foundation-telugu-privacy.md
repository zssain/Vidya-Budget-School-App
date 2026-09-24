# PHASE 13 — SOLID FOUNDATION: CALENDAR, GUARDIANS, LEDGER, NUMBERING, APPROVAL & MESSAGE ENGINES, PRINT ENGINE, CUSTOM FIELDS, ROLES, TELUGU, PRIVACY

## ROLE
You are a senior Rust engineer (data modelling and domain rules) continuing **Vidya Budget School** (v2).

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
14. Run every command you mention and paste real output. Branch `v2/p13`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-13.md`.


## OBJECTIVE
Build the shared foundations every new module plugs into, so later phases add features without
bending the core. Plus full Telugu and the privacy (DPDP) features. Existing features keep working
and move onto the new foundations.

## DONE MEANS
- All 12 foundation items of 02-V2-CHANGES §8 exist in vidya-core, the database and the sync scopes,
  each with tests.
- Existing payments and reversals have backfilled vouchers; debits = credits for every voucher; the
  day book and Home totals are unchanged to the paisa.
- Existing guardian data migrated; messages later read guardians only from the new table.
- The whole app works in Telugu (UI, receipts, report cards, reports) with amount in words; Teacher
  Home in Telugu matches prototype `thomete`.
- Consent records, student export/erase and the incident log work.
- P01–P12 suites green.

## STEPS (build in this order; each step = vidya-core rules + tests → migration → repository →
commands → minimal UI where stated)

### Step 1 — Calendar backbone
- Tables: `school_week(weekday, is_working)` (default Mon–Sat working, Sun off) and
  `calendar_event(id, session_id, starts_on, ends_on, kind holiday|exam|event, title, title_hi,
  title_te, is_non_working bool, circular_id NULL)`.
- vidya-core `calendar.rs`: `is_working_day(date, week, events)`, `working_days(from, to, …)`.
- Use it in: attendance (no sheets required on non-working days; dashboards ignore them), fee due
  dates (display "due on next working day" only as a note, never move money dates), salary days
  (Phase 15/17), leave day counts (Phase 17).
- UI now: Settings → Session & terms gets "Weekly off days" and a simple holidays/events list
  (add/edit/delete, audited). The full Calendar screen comes in Phase 16.

### Step 2 — Guardians
- Tables: `guardian(id, name, relation, mobile, email NULL, language en|hi|te, whatsapp_ok bool,
  school_id, …sync columns)` and `student_guardian(student_id, guardian_id, is_primary)`.
- Migration: one guardian per student from the existing columns; students sharing the same
  guardian mobile + name → the same guardian row (report counts). Old columns stay read-only.
- Student profile and admission form edit guardians (up to 2 per student; one primary). Scopes:
  teachers see guardian name + mobile of their students; address only if class teacher (unchanged).

### Step 3 — General ledger
- Tables: `ledger_account(id, code, name, name_hi, name_te, kind asset|liability|income|expense,
  system bool, active)`, `voucher(id, voucher_no, kind receipt|reversal|expense|salary|advance|
  store_sale|opening|transfer, date, narration, source_table, source_id, created_by, device_id)`
  APPEND-ONLY, `ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise)` APPEND-ONLY
  (triggers like payment/audit).
- Seed system accounts: Cash in hand, UPI/Bank, Cheques to deposit, Fee income, Store income,
  Staff advances, Salary expense, Electricity, Rent, Repairs & maintenance, Stationery, Other
  expense, Opening balance equity.
- vidya-core `ledger.rs`: `voucher_for_payment`, `voucher_for_reversal`, `voucher_for_expense`, …;
  every voucher balanced (sum debits = sum credits, else `LEDGER_UNBALANCED`).
- Wire `record_payment` and reversal apply to create vouchers **in the same transaction**. Server
  applying synced payments creates vouchers there (vouchers are official records; devices show
  provisional ones marked "waiting for school").
- Backfill migration for existing payments/reversals (derived rows only). Test: totals per day and
  per mode identical before/after.

### Step 4 — Numbering engine
`number_series(kind, series, prefix, last_seq)`; vidya-core `numbering.rs` generalising
`receipts.rs` (keep receipt output identical): kinds `receipt R-`, `voucher V-`, `store S-`,
`circular CIR/<session>/NNN` (server-only), `hall_ticket`. Device series reused (A2 …).

### Step 5 — Approval engine
Refactor `requests.rs` into a registry: each request type declares payload schema, who may raise,
who decides, the stale check and the apply function. Existing types keep identical behaviour
(tests). Add types (apply functions implemented in their phases; now they validate and store):
`leave`, `attendance_duty`, `class_notice`.

### Step 6 — Messaging engine
- Tables: `message(id, kind, channel email|wa_tap|wa_auto|app, template_key, language, to_guardian_id
  NULL, to_staff_id NULL, to_address, subject, body, attachments_json, status draft|queued|sent|
  tapped|failed|read, error, related_table, related_id, created_by, created_at, sent_at,
  provider_ref)` and `message_template(key, language, subject, body, updated_by)`.
- vidya-core `messages.rs`: placeholder syntax `{student_name}`, `{amount}`, …; validation against
  a per-template allowed list; rendering with Indian formats; `can_message(guardian)` requires
  `messages` consent (Step 11). Seed templates in en/hi/te for: absence alert, fee reminder, receipt
  share, circular, homework (Hindi/Telugu drafts marked for review).
- No sending yet (Phase 14 adds channels).

### Step 7 — Print engine
One print module (hidden route + template components + @page sizes) with letterhead (logo, school
name, address), language, paper (A4/A5/80 mm). Move receipts and report cards onto it with identical
output (screenshot compare). Later phases add hall tickets, seating charts, salary slips, notices.

### Step 8 — Custom fields
`custom_field(id, entity student|staff, key, label, label_hi, label_te, type text|number|date|choice,
options_json, required, active)` + `custom_value(entity_id, field_id, value)`. vidya-core validates
types. Settings → Custom fields (Principal); fields appear in student/staff profile forms and CSV
import/export (with header = key).

### Step 9 — Roles as data
`role(id, key, name, built_in)`, `role_permission(role_id, action, effect allow|request)` seeded from
the current matrix; `permissions::can` reads the loaded set (pure function still takes the set as
input). The exhaustive matrix test must pass unchanged. No custom-role UI.

### Step 10 — `school_id` everywhere
Add `school_id` (default the single school's id) to every table missing it; indexes where queries
filter by school. No behaviour change.

### Step 11 — Telugu
1. `@fontsource/noto-sans-telugu` 400/500/600, Telugu subset only; add to every font stack after
   Noto Sans Devanagari; `:root[lang="te"]` heading swap like Hindi (Noto Sans Telugu 500, same size,
   no italics).
2. `te.json` with every key (natural, simple Telugu as used in schools; keep ₹, receipt/admission
   numbers, class names like VII-B). List phrases you're unsure of. `check-i18n` covers 3 languages.
3. `vidya-core::words::amount_in_words_te` (Indian system) with tests; unsure number words listed.
4. Language picker: Settings → Languages & modules (prototype `settings` state 3: English, हिंदी,
   తెలుగు) for the school's print language; Profile for each staff member's UI language; guardian
   language on the guardian form.
5. Teacher Home in Telugu matches prototype `thomete` (fidelity test). Logo: English lockup until the
   owner supplies a Telugu one (OWNER-DECISIONS).
6. Measure size impact (fonts) and record.

### Step 12 — Privacy features (02-V2-CHANGES §9)
- `consent(id, student_id, guardian_id, purpose school_records|messages, method signed_form|in_person,
  recorded_by, recorded_at, withdrawn_at)`; student profile → Consent section; admission form asks
  for both purposes; CSV import can mark `consent_signed_form=yes`.
- Student data export (JSON + readable print) and erase-on-request (tombstone personal fields; keep
  financial, academic totals and audit; audited; Principal only; confirmation screen explains what
  stays).
- Retention setting (default "keep"; no automatic deletion).
- Settings → Privacy: incident log (date, description, action, reported to board, reported on) with
  the 72-hour reminder text; list of export/erase actions.

## TESTS
vidya-core unit tests for every new module; migration tests on a v1+P12 database copy (row counts,
money totals, guardian mapping); sync scope tests for new tables per role; print screenshots before/
after for receipts and report cards; i18n completeness; Telugu words.

## STOP CONDITIONS
Any backfill changing a money total. A migration that can't be applied on the v1 test database. A
Telugu term you can't confirm (list it; don't block).

## HANDOFF → `docs/phase-notes/phase-13.md`
Tables added · vidya-core modules and public functions · migration reports (counts, totals) ·
ledger account list · templates seeded · Telugu status + phrases for review + size delta · privacy
features · tests · what Phase 14 needs.
