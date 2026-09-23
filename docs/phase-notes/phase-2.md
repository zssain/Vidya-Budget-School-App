# Phase 2 handoff (in progress) — business rules + schema landed; DB/security remain

Branch: `rebuild/p02` (from `rebuild/p01`). HEAD at handoff: `d4373a7`.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, node v25.3.0.

## Status summary
**DONE this block (Steps 1–2 + the schema for Step 3):** all `vidya-core` business
rules, fully tested and clippy-clean, plus the validated `0001_init.sql`.
**REMAINING (Steps 3–7):** wiring the encrypted DB in Rust, the audit chain, the
`with_write` helper, keys/PIN/recovery, spike removal + size re-measure, and the
benchmark. These are scoped below so the next block can pick them up directly.

## Step 1 — vidya-core: fresh-start (no port)
Owner decision in Phase 1 was **fresh start**, so nothing was ported from the old
edition; every module is new. `vidya-core` depends ONLY on serde, serde_json,
thiserror, time, uuid, sha2, base64, ed25519-dalek. No IO/async/clock/randomness/floats
(a `tests/no_floats.rs` guard greps `src/` for `f32`/`f64` and fails if found).

## Step 2 — modules (all in `crates/vidya-core/src/`, 5,881 LOC)
Test counts in parentheses. Total **235 unit + no-floats guard + 10 doc tests green**;
`cargo clippy -p vidya-core --all-targets -- -D warnings` clean.

- **errors.rs** — `CoreError` (all codes from the prompt) + `code()` + `i18n_key()`
  (`error.<CODE>`) (2).
- **money.rs** — `Paise(i64)`, checked add/sub (overflow→INTERNAL), `sub_floor_zero`, no floats (4).
- **types.rs** — shared enums: Role, StaffState, RequestType, PaymentMode, Mark, FeeFrequency, SyncState.
- **words.rs** — `amount_in_words_en/hi(Paise)` Indian system (22). **Hindi words flagged — see below.**
- **permissions.rs** — `can(actor, action, target) -> Allow{audit_reason_required}|Deny{reason}|NeedsRequest(type)`;
  40-variant `Action`; exhaustive 3×40×2 table (22).
- **attendance.rs** — `validate_sheet_date`, `can_edit_draft`, `can_submit` (IncompleteSheet{remaining}),
  `edit_after_submit`, `percent_present` (integer tenths, half-up), `merge_marks` (20).
- **marks.rs** — `validate_entry`, `can_edit`, `edit_after_submit`, `apply_patch` (patch semantics; NULL≠0) (17).
- **grades.rs** — `grade_for`, `default_scale`, `subject_percent`, `grade_subject`, `grade_report`
  (AB excluded, incomplete→Incomplete) (19).
- **fees.rs** — `generate_dues`, `allocate` (oldest-first + advance_credit), `validate_collection`,
  `apply_synced_payment` (always accepts + excess flag), `validate_reference`, `duplicate_flag` (20).
- **receipts.rs** — `next_receipt_no`/`parse_receipt_no` (R-A2-0419, grows past 9999) (7).
- **admissions.rs** — `provisional_no`, `next_admission_no`, `validate_admission_no` (8).
- **requests.rs** — `create`/`approve`(REQUEST_STALE)/`reject`/`return_for_edits`/`cancel`/
  `resubmit_returned`(new revision)/`mark_applied`/`mark_failed` (23).
- **hlc.rs** — `Hlc{wall_ms,counter,device_id}` total order, `now` (monotonic), `receive` (canonical),
  fixed-width `to_string_form`/`parse`, `skew_flag` (>10 min) (15).
- **conflicts.rs** — `detect` → Apply|Merge|Conflict; payments never conflict (8).
- **licence.rs** — `verify` (ed25519 verify_strict over raw bytes), `status` (perpetual), `check_limits` (11).
- **validation.rs** — name (1–120 Unicode scalars), mobile `^[6-9]\d{9}$`, session label (EN dash),
  date_in_session, PIN 4–6 (17).
- **csv.rs** — RFC 4180 read/write, Excel BOM, `escape_formula` (= + - @ TAB CR) (11).
- **lease.rs** — `lease_status`/`require_active`, `DEFAULT_LEASE_DAYS=30` (6).
- **epoch.rs** — `check_epoch` (EpochOld) (3).

### Error codes → i18n keys
Every `CoreError` variant → `error.<CODE>` via `CoreError::i18n_key()`. Codes: LOCKED,
FORBIDDEN, NOT_FOUND, VALIDATION, AMOUNT_EXCEEDS_DUE, NO_DUES, SHEET_LOCKED,
INCOMPLETE_SHEET, REQUEST_ALREADY_PENDING, REQUEST_STALE, DUPLICATE_ADMISSION_NO,
LICENCE_INVALID, LICENCE_REVOKED, LICENCE_MOVED, LICENCE_LIMIT, PIN_WRONG, PIN_LOCKED,
SESSION_READ_ONLY, LEASE_EXPIRED, EPOCH_OLD, INTERNAL. (These keys need adding to
`src/lib/i18n` when the UI surfaces errors — Phase 3.)

## Step 3 — schema (SQL landed; Rust wiring remains)
`src-tauri/src/db/migrations/0001_init.sql` — the full §7 schema: every table, CHECK for
every enum, `amount_paise > 0` on payment / `>= 0` on dues/heads, the partial unique index
on `student.admission_no`, all requested indexes, the FTS5 `student_fts` (+ sync triggers,
Devanagari-ready), and the append-only `RAISE(ABORT,'append-only')` triggers on audit_log/
payment/reversal. **Validated**: loads clean into sqlite3 3.51 with FTS5. **Remaining**:
the Rust `db/mod.rs` (open with `PRAGMA key='x''<64 hex>'''` first, cipher defaults,
WAL, foreign_keys ON, busy_timeout 5000; a migration runner writing `schema_version` in one
transaction), the repository functions (parameterised only), and the tests (DB unreadable
without key; UPDATE/DELETE on the append-only tables raise).

## Steps 4–7 — REMAINING (scoped)
- **Step 4 audit chain** `src-tauri/src/security/audit.rs`: `append(tx, entry)` with
  `hash = SHA-256(prev_hash ‖ canonical_json(entry))` (sorted keys, no whitespace);
  `verify_chain()` returns the first bad seq; head accessor. Test: tamper one row → names the seq.
- **Step 5 `with_write`**: one transaction = domain change + audit append + outbox/op_log
  append + version/hlc/updated_* bump; rollback on any error. Test: inject failure after row
  write → no row, no audit, no op.
- **Step 6 keys/PIN/recovery** `src-tauri/src/security/`: DB key 32 bytes (desktop `keyring`
  service `in.vidyabudget.app` account `db-key`; Android `KeyStore` trait + TEMP file impl);
  `DB_KEY_MISSING` when a DB exists but the key doesn't; PIN Argon2id (m=19MiB,t=2,p=1) +
  lockout persisted; recovery key 30 Crockford base32 from 150 bits (6×5); backup key =
  Argon2id(recovery, salt, m=64MiB,t=3,p=1); DeviceMode Server|Client write paths.
- **Step 7**: delete `src-tauri/src/spike.rs` + its handler (Phase-2 real code now uses the
  native crates), re-run desktop build, re-record sizes (Phase 1 baseline: dmg 5.58 MB / .app
  9.86 MB — expect slightly smaller).
- **Benchmark**: seed 1,500 students / 60 classes / one term; dashboard aggregates ≤ 150 ms.

## [OWNER] defaults used
- Attendance % denominator = **P + A + L** (Leave counts), 1 decimal half-up (integer tenths).
- Default grade scale: A1 91–100 (10) … E 0–32 (—), editable — encoded in `grades::default_scale()`.
- Offline lease = **30 days** (`lease::DEFAULT_LEASE_DAYS`).
- Licence limits use strict `>` (count == cap is allowed).

## Hindi number words needing owner review (words.rs)
A native-Hindi reviewer should confirm before shipping. High-confidence (asserted in tests):
शून्य, एक–दस, round tens (बीस/तीस/…/नब्बे), सौ, हज़ार, लाख, करोड़, पचास. **Needs review:**
छह/छः (6); 16–19 (सोलह, सत्रह, अठारह, उन्नीस); the irregular bands 21–29/31–39/…/91–99
(इक्कीस, उनतीस, इकतीस …, निन्यानवे) — spellings vary (nukta, chandrabindu vs anusvara); and
हज़ार/करोड़ vs हजार/करोड़ house style. English is fully verified.

## Notes / small deviations flagged by the module work
- `requests::create` takes a leading `id` param (ids are passed in — no randomness in the crate).
- `licence` test fixtures use `SigningKey::from_bytes([seed;32])` (the `rand_core`/`generate`
  feature isn't enabled and can't be added) — deterministic, valid keypair.
- `Validation` rule strings are free-form (e.g. "pattern","required","future") — if the i18n
  error table wants specific rule tokens, pin them in Phase 3.
- `attendance/marks::edit_after_submit(Accountant)` returns NeedsRequest; the real "may touch
  at all" gate is `permissions::can` (flagged in case Forbidden is preferred there).

## Questions for the owner
1. Confirm the Hindi number words above (or supply corrections).
2. Grade scale / attendance-denominator defaults OK to keep?
3. Proceed with Steps 3–7 next (DB + audit + security + benchmark) — ideally a fresh session
   per the README workflow, since it's a large independent block.
