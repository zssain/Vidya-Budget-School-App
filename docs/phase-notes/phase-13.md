# Phase 13 handoff — solid foundation (calendar, guardians, ledger, numbering, approval & message engines) + partial

## Start state / environment
- Branch `v2/p13`, cut from `v2/p12` @ `464cb98`. HEAD at write time `f0e36f3`.
- `git status`: clean; all work committed on `v2/p13` (no push/merge/tag).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**.
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives
  here; prompts in `prompts/`). The `run` skill points at `docs/PROGRESS.md` and a
  doubled prompt path that do not exist — the P13 prompt + `docs/` specs are the
  authority (as in P11/P12).

## Scope actually built this session (Steps 1–6 of 12)
Phase 13 is twelve substantial foundation items. With the owner's "step-by-step,
commit each" pacing, this session delivered the **six pure-backend foundation
steps** — the architectural core every later module plugs into — each committed
with its own tests, migrations and full suite green. **Steps 7–12 remain** (print
engine, custom fields, roles-as-data, `school_id` everywhere, full Telugu, privacy)
and are specced below for a follow-up session.

One commit per step:
| Commit | Step |
|---|---|
| `2c37c09` | 1 — School calendar backbone |
| `58f0e49` | 2 — Guardians |
| `b1f2db4` | 3 — General ledger + backfill |
| `1a09106` | 4 — Numbering engine |
| `57d1bb4` | 5 — Approval engine (registry) |
| `f0e36f3` | 6 — Messaging engine |

## Doc/code discrepancy flagged (Standing Rule 3)
The prompt says visible strings live in `en.json`/`hi.json`. The repo actually uses
per-screen **TypeScript bundles** `src/lib/i18n/strings/*.ts` (`{ en, hi }`), gated
by `scripts/check-i18n.mjs`. Followed the real code (added `calendar.ts`,
`guardians.ts` bundles). Step 11 (Telugu) must add a `te` map to every bundle, add
`te` to `Lang`/`setLang`/`loadLang`, and extend `check-i18n.mjs` to a third language.

## Tables added (migrations 0008–0013, all additive, tested on a v1+P12 DB copy)
- **0008** `school_week` (id-keyed, `weekday` UNIQUE 1–7, `is_working`) · `calendar_event`
  (`session_id`, `starts_on`/`ends_on` inclusive, `kind`, `title`/`_hi`/`_te`,
  `is_non_working`, `circular_id`) + indexes.
- **0009** `guardian` (name, relation, mobile, email, language, whatsapp_ok, school_id)
  · `student_guardian` (is_primary) + indexes. **Backfill** in the migration.
- **0010** `ledger_account` (name/`_hi`/`_te`, kind, system, active) · `voucher`
  (APPEND-ONLY) · `ledger_entry` (APPEND-ONLY) + triggers + indexes; seeds 13 system accounts.
- **0011** `number_series` (kind, series, prefix, last_seq) — device-local, not synced.
- **0012** recreated `request` with the widened `type` CHECK (adds `leave`,
  `attendance_duty`, `class_notice`). Standard table-rebuild (copy → drop → rename).
- **0013** `message` (outbox) · `message_template` (id-keyed, UNIQUE(key,language)) +
  indexes; seeds 5 templates × en/hi/te (15 rows).
- All new tables carry `school_id` from the start (so Step 10 only touches pre-P13 tables).

## vidya-core modules & public functions added
- `calendar.rs` — `SchoolWeek` (default Mon–Sat working, Sun off), `CalendarEvent`,
  `is_working_day`, `working_days`, `next_working_day`, `parse_date`, `weekday_index/_iso`.
- `guardians.rs` — `validate_language` (en/hi/te), `validate_guardian`, `dedup_key`,
  `has_guardian_data`, `MAX_GUARDIANS_PER_STUDENT = 2`.
- `ledger.rs` — `system_accounts()` (13), `Entry`, `validate_balanced` (→ new
  `CoreError::LedgerUnbalanced` / `LEDGER_UNBALANCED`), `voucher_for_payment` /
  `_reversal` / `_expense`, `money_account_for_mode`, account-id consts.
- `numbering.rs` — `NumberKind` (receipt/voucher/store/circular/hall_ticket),
  `format_number` (receipt format IDENTICAL — cross-checked); `receipts::next_receipt_no`
  now delegates here.
- `requests.rs` — approval **registry**: `RequestSpec`, `spec(type)`, `can_raise`.
  `types::RequestType` gains `Leave`/`AttendanceDuty`/`ClassNotice` + `as_key`/`from_key`/`ALL`.
- `messages.rs` — `Channel`, `MessageStatus`, `TEMPLATE_KEYS`, `allowed_placeholders`,
  `placeholders_in`, `validate_template`, `render`, `can_message`.
- `audience.rs` — new tables mapped: calendar → `admin`; guardian/student_guardian →
  `finance`; ledger tables → `finance`; message/message_template → `admin` (P14 refines).

## src-tauri modules & wiring
- `calendar.rs` repo (`load_week`/`load_events`/`is_working_day`); `dash::attendance_pending`
  returns `[]` on non-working days.
- `guardians.rs` repo (`ensure_primary_guardian` find-or-create by mobile+name,
  `list_for_student`, `count_for_student`); `create_student` + `import_students_commit`
  create the primary guardian inline (siblings dedup); legacy `student.guardian_*`
  columns still written for read-only compatibility (dropped in P18).
- `ledger.rs` repo (`post_payment_voucher`/`post_reversal_voucher`/`backfill_vouchers`/
  `ledger_imbalance`); `record_payment` posts the balanced receipt voucher in the same
  transaction (server/confirmed); approving a `payment_reversal` posts the opposite
  voucher; startup + demo seed run the idempotent backfill.
- `numbering.rs` repo (`next_no` — reserves the sequence atomically, seeds `last_seq`
  from existing rows for continuity); receipts + vouchers use it (receipt still R-A2-0419).
- Commands added: `get_calendar`, `set_weekly_offs`, `add/update/delete_calendar_event`,
  `add/update/set_primary/remove_guardian`. `get_student_profile` now returns `guardians[]`.
  `create_request` validates the kind + enforces `can_raise` for the new types;
  `decide_request` reads `spec().apply_available`.
- `sync/scope.rs`: calendar in every role's scope; guardian/student_guardian scoped
  (teachers see their students' guardians, name+mobile only); ledger tables finance-only
  (added to `FEE_TABLES` so teachers never receive them). Messaging tables NOT yet in the
  device scope — P14 wires the communication module's tables + scope (as P11 planned).

## Migration reports (counts / money totals)
- Guardian backfill (0009 + Rust `ensure_primary_guardian`): one guardian per distinct
  `(mobile, name)`; siblings share the row; each backfilled link is `is_primary`.
  Test `guardian_backfill_dedupes_siblings` (v1 rows: 2 siblings + 2 others + 1 with no
  guardian → 3 guardians, 4 primary links, 0 for the guardian-less student). Legacy
  columns untouched.
- Ledger backfill (`ledger::backfill_vouchers`, idempotent, derived-only): one receipt
  voucher per payment, one reversal voucher per reversal. Test
  `backfill_is_balanced_idempotent_and_preserves_money_totals`: **day+mode money totals
  identical before/after** (they are computed from `payment`/`reversal`, which the ledger
  never touches); whole-book `Σ(debit−credit) = 0`; re-run creates 0.
- Request rebuild (0012): preserves existing request rows, accepts the 3 new types, still
  rejects unknown types. **Production caveat:** the recreate's implicit row-copy DROP fails
  if `reversal.request_id` references a `request` row (reversal is append-only, can't be
  nulled). The demo/v1 test DB has no `reversal` rows, so it applies cleanly; a production
  DB with applied reversals needs the recreate run with `foreign_keys=OFF` at the app layer
  (one-time). **Flagged for P18 / owner.**

## Ledger account list (13, seeded en + hi; te in Step 11)
cash `1001` · bank (UPI/Bank) `1002` · cheques `1003` · staff_advances `1004` ·
fee_income `4001` · store_income `4002` · salary_expense `5001` · electricity `5002` ·
rent `5003` · repairs `5004` · stationery `5005` · other_expense `5099` ·
opening_equity `3001`.

## Templates seeded (message_template, 5 × en/hi/te = 15 rows)
`absence_alert`, `fee_reminder`, `receipt_share`, `circular`, `homework`. English is
authoritative; **Hindi and Telugu bodies are DRAFTS pending native-speaker review**
(OWNER-DECISIONS #12). Every seeded body validates against
`vidya_core::messages::allowed_placeholders` (test `seeded_message_templates_validate`).

## Telugu status
Only two P13 UI bundles gained strings (`calendar.ts`, `guardians.ts`) in en/hi. Telugu
across the whole app is **Step 11 (not started)** — see below. Ledger account `name_te`
and template `te` are the only Telugu committed so far (drafts, for review).

## Tests (all real, this session)
- `cargo test --workspace --locked` → **592 passed / 0 failed** (was 550 at P12 tip +42).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo tree -i aws-lc-rs` → empty (ring only).
- `npm run verify` → green: typecheck; check-hex (42 tokens); **check-i18n 20 modules,
  866 keys, en/hi in sync**; **check-deps 30 npm + 34 cargo, all within §13 — NO new
  dependencies**; contrast; logs; vitest 38.
- New tests of note: calendar working-day rules + non-working dashboard; guardian dedup
  migration + create/add/set-primary/remove + max-2 + teacher-forbidden; ledger balance
  invariants + backfill money-total equality + append-only; numbering format-identity +
  receipt continuity; approval registry + request rebuild + new-type create/forbid;
  messaging placeholder validation/render + seeded-template validation.

### Changed test expectations
None. No existing test expected value changed. Existing behaviour preserved:
`record_payment` still numbers `R-A2-0419`; `decide_request` auto-applies exactly the
same types (now via `spec().apply_available`); the permission matrix test is untouched
(roles-as-data is Step 9, not yet built).

## REMAINING — Steps 7–12 (for the next session), with notes
Build in order; each = core rules + tests → migration → repo → commands → minimal UI.

- **7 — Print engine.** Extract one print module: `pageCss(size)` for **A4/A5/80 mm**,
  a props-configurable `<Letterhead>` (logo/name/address; the 3 existing docs differ —
  receipt 140×46 centered, report card 120×40 left+exam title, day book its own), and a
  `no-print` toolbar. Move `ReceiptDoc` + `ReportCardDoc` onto it with **identical markup**.
  Pixel/screenshot fidelity is **only assertable on the canonical macOS baseline machine**
  (P11 precedent: `tests/e2e/__screens__` PNGs are gitignored) — regen + assert there.
- **8 — Custom fields.** `custom_field` (entity student|staff, key, label/`_hi`/`_te`,
  type text|number|date|choice, options_json, required, active) + `custom_value`. Core
  type validation (a `custom_fields.rs` in vidya-core). Settings → Custom fields
  (Principal); render in student/staff profile forms; CSV import/export with header = key.
- **9 — Roles as data.** `role` + `role_permission(role_id, action, effect allow|request)`
  seeded from the CURRENT matrix in `permissions.rs`. `permissions::can` keeps taking the
  permission set as input (pure) — load the set from the DB and pass it in. The exhaustive
  `role_matrix.rs` test MUST pass unchanged. No custom-role UI.
- **10 — `school_id` everywhere.** Migration adds `school_id` (default the single
  `school.id`) to every pre-P13 table missing it + indexes where queries filter by school.
  No behaviour change. All P13 tables already have it.
- **11 — Telugu (largest).** `@fontsource/noto-sans-telugu` 400/500/600 (add to §13 deps
  list + every font stack after Noto Sans Devanagari; `:root[lang="te"]` heading swap like
  Hindi). Add a `te` map to ALL ~20 `strings/*.ts` bundles (866 keys), add `te` to
  `Lang`/`setLang`/`loadLang`/index merge, and teach `check-i18n.mjs` the 3rd language.
  `vidya_core::words::amount_in_words_te` (Indian system) + tests; list unsure number
  words. Language pickers: Settings → Languages & modules (school print language), Profile
  (per-staff UI language), guardian form (already has a language field). Teacher Home in
  Telugu = prototype `thomete` fidelity. Logo: English lockup in Telugu until owner
  supplies one (OWNER #6). Record font size delta. Fill ledger `name_te` + confirm template
  `te` drafts.
- **12 — Privacy (DPDP).** `consent` (student, guardian, purpose school_records|messages,
  method signed_form|in_person, recorded_by/at, withdrawn_at) — profile Consent section,
  admission asks both purposes, CSV `consent_signed_form=yes`. `can_message` (already in
  core) must read this table. Student export (JSON + readable print) + erase-on-request
  (tombstone personal fields; keep financial/academic totals + audit; Principal only;
  confirmation screen). Retention setting (default keep). Settings → Privacy: incident log
  (date, description, action, reported to board, reported on) + 72-hour reminder text; list
  of export/erase actions.

## What Phase 14 (Communication) needs from P13
- Numbering engine ready: use `numbering::next_no(NumberKind::Circular, <session>)` for
  circulars (server-only) and `Store`/`HallTicket` later.
- Messaging engine ready: `message`/`message_template` tables, `messages::render` +
  `validate_template` + `can_message`. P14 adds channels (email via sync-account Gmail,
  tap-to-WhatsApp, optional Cloud API), wires `message` audience per recipient, and adds
  the messaging tables to `sync/scope` under the communication module (the `MODULE_OFF`
  gate + scope exclusion are already in place from P11).
- Guardians table is the messaging recipient source (`to_guardian_id`); Step 12 consent
  gates sends via `can_message`.

## Owner decisions still open (see docs/OWNER-DECISIONS.md)
Unchanged from P12: #1 app identifier, #2 price, #6 Telugu logo, #7 salary formula,
#8 leave types/quotas, #9 remote check-in, #10 retention, #11 UPI QR, #12 native review of
Hindi/**Telugu** (now also the P13 template + ledger drafts), #13 gmail.send verification,
#14 code signing, #15 GST. **New this phase:** the request-table rebuild (0012) needs
`foreign_keys=OFF` on production DBs that hold applied reversals — confirm the one-time
migration path before a v1→v2 upgrade ships.
