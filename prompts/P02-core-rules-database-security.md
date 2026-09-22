# PHASE 2 of 10 — BUSINESS RULES (vidya-core), ENCRYPTED DATABASE, AUDIT CHAIN, SECURITY

## ROLE
You are a senior Rust engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies with the listed features. Anything else → STOP and ask.
4. Never invent features, rules, copy, numbers, API fields, library APIs or config keys.
   Check installed crate source/docs for every API. Unsure → STOP and ask.
5. The mock wins; no visual changes in this phase.
6. Business rules only in `crates/vidya-core`. Commands stay thin.
7. No fake success anywhere.
8. Never delete/weaken tests. Never edit an earlier phase's migration.
9. Run every command you mention; paste real output. Separate environment vs code failures.
10. Work on branch `rebuild/p02`; small commits; no push/merge/tag unless asked.
11. Stop conditions are real.
12. Finish with the handoff file.

## OBJECTIVE
All business rules as pure, fully tested Rust; the encrypted SQLite database with the full
schema; the tamper-evident audit chain; key storage, PIN and recovery-key primitives. No UI
changes in this phase (Phase 3 wires the UI).

## DONE MEANS
- `cargo test --workspace` green; every rule and edge case below has a named test.
- `cargo clippy --workspace -- -D warnings` clean.
- A test proves the database file is unreadable without the key.
- Tests prove UPDATE/DELETE on `audit_log`, `payment`, `reversal` raise.
- Tampering one audit row makes `verify_chain` fail and name the first bad `seq`.
- `spike.rs` removed; size re-measured and recorded.

## CURRENT STATE
Phase 1: UI shell with fixtures; size spike done; `crates/vidya-core` is either new or the
ported old crate.

## STEPS

### Step 1 — Port or create vidya-core
If old modules exist (money, fees, marks, dates, validation, hlc, permissions, roles):
keep their passing tests, then change them to match THIS context (paise not rupees if
different, action names, roles). Every behaviour change needs a test and a line in the
handoff ("changed X because context §Y"). `vidya-core` may depend ONLY on `serde`,
`serde_json`, `thiserror`, `time`, `uuid`, `sha2`, `base64`, `ed25519-dalek`. No IO, no
async, no clock reads (time is passed in), no randomness (ids passed in).

### Step 2 — Modules and exact rules (`crates/vidya-core/src/`)
Each function takes snapshots + input and returns a `Decision`/`Result`. Write tests first
for every bullet.

**errors.rs** — `CoreError` with codes: `LOCKED, FORBIDDEN{reason}, NOT_FOUND, VALIDATION{field, rule},
AMOUNT_EXCEEDS_DUE, NO_DUES, SHEET_LOCKED, INCOMPLETE_SHEET, REQUEST_ALREADY_PENDING,
REQUEST_STALE, DUPLICATE_ADMISSION_NO, LICENCE_INVALID, LICENCE_REVOKED, LICENCE_MOVED,
LICENCE_LIMIT{what}, PIN_WRONG{remaining}, PIN_LOCKED{until}, SESSION_READ_ONLY,
LEASE_EXPIRED, EPOCH_OLD, INTERNAL{id}`. Each maps to an i18n key `error.<code>`.

**money.rs** — `Paise(i64)` newtype; checked add/sub (overflow → error); no floats
anywhere in the crate (add a test that greps the crate source for `f32`/`f64` and fails).
Rupee formatting is the UI's job; vidya-core only returns paise.

**words.rs** — `amount_in_words_en(Paise)` and `amount_in_words_hi(Paise)` in the Indian
system (thousand, lakh, crore): 310000 paise → "Three thousand one hundred rupees only";
include paise ("… and fifty paise only"). Hindi uses standard Hindi number words (e.g.
"तीन हज़ार एक सौ रुपये मात्र"). Test 0, 1, 99, 100, 1,000, 1,00,000, 1,00,00,000, and
values with paise. If you are not sure of a Hindi number word, list it in the handoff for
owner review — do not guess silently.

**permissions.rs** — `can(actor, action, target) -> Allow | Deny(reason) |
NeedsRequest(type)`. `actor = {staff_id, role, state, class_teacher_of: [class_id],
class_subjects: [class_subject_id]}`. Encode context §5 exactly, including: suspended/
removed → Deny; teacher sees no fee data; guardian address only if class teacher;
Principal direct edit of locked data → Allow + `audit_reason_required`. Write ONE
exhaustive test table: every action × every role × (own / not own target).

**attendance.rs** — one sheet per class per date; no future dates (today passed in);
drafts editable by class teacher / Principal; submit requires every ACTIVE enrolled student
on that date to be marked (`INCOMPLETE_SHEET` with count); after submit: teacher change →
`NeedsRequest(attendance_correction)`, Principal → Allow + reason; percent present =
P ÷ (P + A + L) **[OWNER default]**, 1 decimal, half-up; merge rule for two submissions of
the same sheet (context §8.5).

**marks.rs** — `0 ≤ marks ≤ max_marks` or `absent`; NULL = not entered (never zero);
sheet lock PER exam_subject; teacher only own class_subject; after submit → request;
saving a subset of rows never touches other rows (patch semantics: only listed students
change; explicit `clear` is a separate action).

**grades.rs** — `grade_for(pct, scale)`; subject % and overall % rounded half-up to 1
decimal; absent → "AB", excluded from totals and max; incomplete (any NULL) → result
"Incomplete", no grade. Default scale **[OWNER default, editable]**: A1 91–100 (10), A2
81–90 (9), B1 71–80 (8), B2 61–70 (7), C1 51–60 (6), C2 41–50 (5), D 33–40 (4), E 0–32 (—).
No pass/fail or promotion decisions here (not specified).

**fees.rs** — dues generation: term heads once per term; monthly heads per month for
matching students (e.g. transport only if `student.transport`); once heads at admission;
never regenerate a due that has any allocation. Allocation: oldest due first; excess →
`advance_credit`. Two validation modes:
- `validate_collection(amount, total_due_now)` → `0 < amount ≤ due` else
  `AMOUNT_EXCEEDS_DUE` / `NO_DUES` (used on the collecting device only).
- `apply_synced_payment(payment, dues_now)` → ALWAYS accepts; returns allocations +
  optional `excess_payment` flag (context §8.6).
Reference rules: UPI 6–30 alphanumerics required; cheque ≤ 60 chars required; cash none.
Duplicate detection (context §8.6) → flags, never rejection.

**receipts.rs** — `next_receipt_no(series, last_seq)` → `R-A2-0419`; 4 digits, grows to 5
after 9999; never reused.

**admissions.rs** — `provisional_no(series, seq)` → `P-A2-0003`;
`next_admission_no(year, last)` → `2026/0142`; validation of `YYYY/NNNN`.

**requests.rs** — create with `before_json`, `after_json`, `base_version`, `reason`
(1–300 chars); one pending request per target per type (`REQUEST_ALREADY_PENDING`);
types from context §7 incl. `access_change`, `device_replacement`; decide approve /
reject / return (note ≥ 5 chars for reject/return); cancel by requester while pending;
returned → requester edits → NEW revision row linked by `parent_request_id` (old revision
immutable). Approve checks: approver currently allowed; target `version == base_version`
else `REQUEST_STALE` (Principal must reopen and review current values). Approve returns
the exact change set to apply; `apply_state` moves `not_applied → applied | failed`.

**hlc.rs** — `Hlc{wall_ms, counter, device_id}`; `now(prev, wall)` monotonic even if the
wall clock goes backwards; `receive(local, remote, wall)`; `Ord` = total order; string
form fixed-width and sortable; skew detection (> 10 min → warning flag).

**conflicts.rs** — `detect(op, current_row, fields_changed_since(base_version))` →
`Apply | Merge(fields) | Conflict(fields)`; payments never conflict.

**licence.rs** — `verify(licence_json, signature, public_key)` → `Licence`;
`status(last_check_result)` → `Active | Revoked | Moved` (perpetual: no expiry, no grace);
limits check when `max_*` is non-null.

**validation.rs** — names 1–120 chars, any Unicode incl. Devanagari, trimmed; mobile
`^[6-9]\d{9}$`; admission no; dates within the session; session label "2026–27"; PIN 4–6
digits.

**csv.rs** — pure CSV writer/reader (RFC 4180 quoting, UTF-8 with BOM for Excel) and
`escape_formula(cell)`: prefix `'` when a cell starts with `= + - @` TAB or CR. Tested.

**lease.rs / epoch.rs** — lease expiry check (context §8.8), epoch comparison (§8.9).

### Step 3 — Database (`src-tauri/src/db/`)
1. Open with `rusqlite` + SQLCipher: `PRAGMA key = "x'<64 hex>'"` as the FIRST statement,
   then `PRAGMA cipher_compatibility`/defaults as documented by SQLCipher, `journal_mode =
   WAL`, `foreign_keys = ON`, `busy_timeout = 5000`.
2. Migrations: `migrations/0001_init.sql` … with a `schema_version` table; applied in one
   transaction each; a failed migration leaves the old version intact.
3. Full schema from context §7 with CHECK constraints for every enum and `amount_paise > 0`
   on payments; UNIQUE/indexes: `student.admission_no` (partial, NOT NULL), `enrollment`
   (class_id), (student_id), `attendance_sheet(class_id,date)`,
   `attendance_mark(sheet_id,student_id)`, `mark_entry(sheet_id,student_id)`,
   `fee_due(student_id)`, `payment(receipt_no)` UNIQUE, `payment(student_id)`,
   `payment(collected_at)`, `request(status)`, `op_log(server_seq)`, `outbox(hlc)`,
   `audit_log(seq)`, `applied_ops(op_id)` PRIMARY KEY.
4. FTS5 table `student_fts(name, admission_no, provisional_no, guardian_name,
   guardian_mobile)` with triggers; must work with Devanagari names (test).
5. Triggers `RAISE(ABORT, 'append-only')` on UPDATE/DELETE of `audit_log`, `payment`,
   `reversal`.
6. Repository functions: SQL only, parameterised only (no string-built SQL with values).

### Step 4 — Audit chain (`src-tauri/src/security/audit.rs`)
`append(tx, entry)` computes `hash = SHA-256(prev_hash ‖ canonical_json(entry))`
(canonical = sorted keys, no whitespace); `verify_chain()` walks all rows and returns the
first bad seq; head hash accessor.

### Step 5 — Write transaction helper
`with_write(ctx, |tx| { … })` that, in ONE transaction: runs the domain change, appends
the audit entry, appends the op to `outbox` (or `op_log` in server mode), bumps
`version`/`hlc`/`updated_*`, and rolls back everything on any error. Test: inject a
failure after the row write → no row, no audit, no op.

### Step 6 — Keys, PIN, recovery key (`src-tauri/src/security/`)
- DB key: 32 random bytes; desktop `keyring` (service `in.vidyabudget.app`, account
  `db-key`); Android: `KeyStore` trait with a TEMP private-file implementation marked
  `// TEMP: replaced by plugins/vidya-android in Phase 4`.
- Missing key but DB file present → error `DB_KEY_MISSING`; never create a new DB over an
  existing file.
- PIN: Argon2id (m=19 MiB, t=2, p=1); lockout counter + until time persisted in the DB.
- Recovery key: 30 Crockford base32 chars from 150 random bits, 6 groups of 5; backup key
  = Argon2id(recovery key, salt, m=64 MiB, t=3, p=1); salt stored in `school`.
- DeviceMode `Server | Client`: Server writes `confirmed` + `op_log(server_seq)`; Client
  writes `on_device` + `outbox`. Both paths tested now.

### Step 7 — Remove the spike
Delete `spike.rs` and its command; re-run the size builds; record the new sizes.

## EDGE-CASE TESTS (each a named test)
Crash mid-write → no partial rows · two simultaneous payments → unique receipts · amount ==
due → allowed, balance 0 · no dues → `NO_DUES` · synced payment over due → accepted +
excess flag · duplicate UPI reference → flag, both kept · class with 0 students → cannot
submit · student left → excluded from new sheets, history kept · section transfer →
enrollment closed/opened, old marks/attendance intact · wall clock earlier than last HLC
→ still monotonic + skew flag · Hindi names stored and FTS-searchable · approve after target
changed → `REQUEST_STALE` · second approval of the same request → error, applied once ·
returned request resubmitted → new revision, old unchanged · marks patch with 3 of 34
students → other 31 unchanged · licence with null limits → unlimited · CSV cell "=SUM(A1)"
→ escaped.

## BENCHMARK
Seed 1,500 students / 60 classes / one term of attendance and payments in a test DB;
dashboard-style aggregate queries ≤ 150 ms on your machine (record numbers).

## STOP CONDITIONS
A rule not in the context or this prompt is needed (late fines, concessions, discounts,
scholarships, promotion criteria) → stop and ask. SQLCipher cannot be built on a target.

## HANDOFF → `docs/phase-notes/phase-2.md`
Final DDL · module list with public functions · error codes → i18n keys · rules changed
while porting (with reasons) · test counts per module · benchmark numbers · sizes after
spike removal · [OWNER] defaults · Hindi number words needing review · questions.
