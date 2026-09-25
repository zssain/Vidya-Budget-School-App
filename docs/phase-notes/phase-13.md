# Phase 13 handoff — solid foundation: calendar, guardians, ledger, numbering, approval/message/print engines, custom fields, roles-as-data, school_id, Telugu, privacy

## Start state / environment
- Branch `v2/p13`, cut from `v2/p12` @ `464cb98`. HEAD at write time `653e8b6`.
- `git status`: clean; all work committed on `v2/p13` (no push/merge/tag).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**.
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives
  here; prompts in `prompts/`). The `run` skill points at `docs/PROGRESS.md` and a
  doubled prompt path that do not exist — the P13 prompt + `docs/` specs are the
  authority (as in P11/P12).

## Scope — all 12 foundation items built
Every one of the twelve §8/§9 foundation items exists in vidya-core, the database
and (where relevant) the sync scopes, each with tests. Built "step-by-step, commit
each"; full suite green after every step. Three items carry a documented, scoped
follow-up (marked ▸ below) — none blocks the foundation.

One commit per step:
| Commit | Step |
|---|---|
| `2c37c09` | 1 — School calendar backbone |
| `58f0e49` | 2 — Guardians |
| `b1f2db4` | 3 — General ledger + backfill |
| `1a09106` | 4 — Numbering engine |
| `57d1bb4` | 5 — Approval engine (registry) |
| `f0e36f3` | 6 — Messaging engine |
| `f09f246` | 7 — Print engine (shared printKit) |
| `6b02c2c` | 8 — Custom fields |
| `ae1ad18` | 9 — Roles as data |
| `668c654` | 10 — `school_id` everywhere |
| `6b2fb69` | 12 — Privacy (DPDP) |
| `653e8b6` | 11 — Telugu |

### Scoped follow-ups (documented, non-blocking)
- ▸ **Step 7 print**: `printKit` (pageCss A4/A5/80mm + letterhead + toolbar) is
  shared and receipts/report cards moved onto it with identical DOM; the **pixel
  screenshot compare** is only assertable on the canonical macOS baseline machine
  (P11 precedent — `tests/e2e/__screens__` PNGs are gitignored). Regen+assert there.
- ▸ **Step 8 custom fields**: value storage + validation + Settings/profile UI done;
  **CSV import/export columns (header = key)** are the remaining wire-up on the CSV
  commands.
- ▸ **Step 11 Telugu**: `amount_in_words_te` (real Telugu, tested), the Noto Sans
  Telugu font, the `te` language plumbing/pickers and the receipt Telugu amount are
  done; the **full ~800-key UI bundle translation** (and extending `check-i18n` to
  enforce `te`) is a native-speaker task — `te` is selectable now with an English
  fallback for untranslated keys.
- ▸ **Step 12 privacy**: consent storage/commands + Settings→Privacy + profile
  Consent/export/erase UI done; **admission-form consent capture + CSV
  `consent_signed_form=yes`** are the remaining wire-up.

## Doc/code discrepancy flagged (Standing Rule 3)
The prompt says visible strings live in `en.json`/`hi.json`. The repo actually uses
per-screen **TypeScript bundles** `src/lib/i18n/strings/*.ts` (`{ en, hi }`), gated
by `scripts/check-i18n.mjs`. Followed the real code (added `calendar.ts`,
`guardians.ts` bundles). Step 11 (Telugu) must add a `te` map to every bundle, add
`te` to `Lang`/`setLang`/`loadLang`, and extend `check-i18n.mjs` to a third language.

## Tables added (migrations 0008–0017, all additive, tested on a v1+P12 DB copy)
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
- **0014** `custom_field` (entity, key, label/`_hi`/`_te`, type, options_json, required,
  active, sort_order) UNIQUE(entity,key) · `custom_value` UNIQUE(entity_id,field_id).
- **0015** `role` (key, name, built_in; 3 built-ins seeded) · `role_permission`
  (role_id, action, effect allow|request; seeded in Rust from `default_permissions`).
- **0016** `school_id` added + backfilled on 25 mutable domain tables + indexes.
- **0017** `consent` · `incident_log` · `privacy_action` + indexes.
- All P13 tables carry `school_id` from the start (so 0016 only touches pre-P13 tables).

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
- `custom_fields.rs` — `Entity`, `FieldType`, `validate_key`, `validate_field_def`,
  `validate_value` (number by shape — no floats).
- `permissions.rs` (roles-as-data) — `Effect`, `Action::ALL`/`as_key`/`from_key`,
  `default_permissions()` (matrix DERIVED from `can`), `effect_of(set, …)`. `can`
  UNCHANGED — the exhaustive `role_matrix` test still passes.
- `words.rs` — `amount_in_words_te` (Indian system; DRAFT, native review pending).
- `audience.rs` — new tables mapped: calendar → `admin`; guardian/student_guardian/
  consent → `finance`; ledger tables → `finance`; message/message_template/custom_field/
  custom_value → `admin` (P14 refines message audience per recipient).

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
- `roles.rs` repo — `seed_role_permissions` (idempotent, from `default_permissions`,
  run at startup) + `load_permission_set`.
- `printKit.tsx` (frontend) — one print module: `pageCss(A4/A5/80mm)` + letterhead
  blocks (`PrintLogo`/`PrintSchoolName`/`PrintAddress`) + `PrintToolbar`; receipts +
  report cards compose it (identical DOM).
- Commands added (with wrappers, `COMMANDS`, both invoke_handler lists, `commands.json`,
  `api.ts`): calendar (5), guardians (4), custom fields (6), privacy (10). `record_payment`
  posts the receipt voucher; `decide_request` reads `spec().apply_available`;
  `get_student_profile` returns `guardians[]`; the receipt DTO gains `amount_words_te`.
- `sync/scope.rs`: calendar in every role's scope; guardian/student_guardian/consent
  scoped (teachers see their students' guardians, name+mobile only); ledger tables
  finance-only (in `FEE_TABLES` so teachers never receive them). Messaging + custom-field
  + role tables are NOT yet in the device incremental scope — P14 wires the communication
  module's tables + scope (as P11 planned).

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
- `school_id` (0016): added + backfilled to the single school's id on 25 mutable domain
  tables; test `school_id_backfilled_on_domain_tables`. Append-only (payment/reversal/
  audit) + per-install bookkeeping are excluded by design (triggers block the UPDATE;
  documented in the migration). No behaviour change (every existing test unchanged).
- Roles (0015): `role_permission` seeded from `default_permissions()` and read back;
  test `seeds_and_loads_the_built_in_matrix` (seed count == matrix, idempotent, load
  reproduces it). Erase (0017): `export_then_erase_keeps_money_tombstones_personal`
  (payments kept, name → `(erased)`, 2 privacy actions logged).

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

## Telugu status + phrases for native review (OWNER-DECISIONS #12)
Done: `@fontsource/noto-sans-telugu` 400/500/600 (imported; added to every font stack
after Noto Sans Devanagari; `:root[lang="te"]` heading swap = Noto Sans Telugu 500);
`te` is a selectable `Lang` (Settings + guardian pickers, receipt Telugu amount);
`vidya_core::words::amount_in_words_te`.

**DRAFT / review needed:**
- **amount_in_words_te** uses simple space-separated composition and nominative-plural
  scale words (వందలు/వేలు/లక్షలు/కోట్లు). Formal Telugu contracts some compounds (sandhi,
  e.g. "ఇరవైఒకటి") and uses **oblique** scale forms before more digits ("మూడు వేల" not
  "మూడు వేలు"). Review: the tens (ఇరవై/ముప్పై/నలభై/యాభై/అరవై/డెబ్బై/ఎనభై/తొంభై), the 11–19
  forms, and the scale/oblique choices.
- **Ledger `name_te`** was left NULL (fill in review) and **template `te` bodies** are drafts.
- **Full UI translation NOT done**: the ~800 legacy-bundle keys have no `te` yet; `t()`
  falls back to English for them. `check-i18n` stays at en/hi (extend to `te` once the
  bundles are translated). The 4 new P13 bundles are also en/hi only for now.
- **Size delta**: `@fontsource/noto-sans-telugu` adds the Telugu woff2 subset (~30–60 KB
  gz across 400/500/600) — well within §13 limits; it is lazy per weight like the other
  Fontsource subsets. Measure the exact bundle delta on the release build.

## Tests (all real)
- `cargo test --workspace --locked` → **614 passed / 0 failed** (was 550 at P12 tip, +64).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo tree -i aws-lc-rs` → empty (ring only).
- `npm run verify` → green: typecheck; check-hex (42 tokens); **check-i18n 22 modules,
  932 keys, en/hi in sync**; **check-deps 31 npm + 34 cargo, all within §13** (the one new
  dep is the §13-allowed Telugu font); contrast; logs; vitest 38.
- New tests of note (per step): calendar working-day + non-working dashboard; guardian
  dedup migration + CRUD + max-2 + teacher-forbidden; ledger balance invariants + backfill
  money-total equality + append-only; numbering format-identity + receipt continuity;
  approval registry + request rebuild + new-type create/forbid; messaging placeholder
  validation + seeded-template validation; print (DOM); custom-field type validation +
  CRUD + Principal-only; roles-as-data matrix consistency + seed/load; `school_id` backfill;
  privacy consent/export/erase/retention/incident; Telugu `amount_in_words_te`.

### Changed test expectations
None. No existing test's expected value changed. Existing behaviour preserved end-to-end:
`record_payment` still numbers `R-A2-0419` (now via the numbering engine); `decide_request`
auto-applies exactly the same types (now via `spec().apply_available`); the exhaustive
permission-matrix test is untouched (roles-as-data derives its data from `can`); `school_id`
is unused so nothing reads it.

## Remaining sub-items to finish later (all non-blocking; foundation is complete)
1. **Print screenshots** (Step 7): regenerate + assert the receipt/report-card fidelity
   baselines on the canonical macOS machine (they're gitignored; can't run in this sandbox).
2. **Custom-field CSV columns** (Step 8): add each field's `key` as a CSV column on
   student import/export (the value storage/validation are ready).
3. **Full Telugu UI translation** (Step 11): add a `te` map to every `strings/*.ts` bundle
   (~800 keys) and extend `check-i18n.mjs` to enforce `te`; native-speaker review of the
   amount-in-words words, ledger `name_te`, and template `te` drafts.
4. **Admission-form consent + CSV** (Step 12): capture both consent purposes on the
   admission form and honour `consent_signed_form=yes` on CSV import (commands are ready).
5. **Request-table rebuild on production** (Step 5 / migration 0012): on a v1 DB that holds
   applied reversals, run the recreate with `foreign_keys=OFF` (one-time). Confirm before a
   v1→v2 upgrade ships.

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
