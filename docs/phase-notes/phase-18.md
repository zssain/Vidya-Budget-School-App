# Phase 18 handoff — v2 hardening, upgrade safety, docs and the v2.0.0 release prep

## Start state / environment
- Branch `v2/p18`, cut from `v2/p17` @ `d682f41` (the P17 handoff tip).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**, pnpm 10.31.0.
- `git status` at start: clean. All work committed on `v2/p18` (no push/merge/tag).
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives here;
  prompts in `prompts/`).

## The one thing to read before releasing (Standing Rule 3 + Rule 10)
The P18 prompt opens with *"Phases 1–18 are built and v2.0.0 is released."* **Neither was true at
the start of this phase** and the second is still not achievable as a *multi-device* product:

1. The repo was at **Phase 17**; Phase 18 had not been run and there were **no git tags** (no
   release). This phase is that P18 run.
2. **The live networked half of v2 was never built.** Since P12/P14 the following are
   *specced-but-not-built* because they need the owner's real Google/Meta accounts and an Android
   toolchain — none of which exist in the CI/dev sandbox:
   - the **live Google Drive** `drive.file` client + the sign-in UI, and the live LAN→Drive→queue
     loop + `epoch.json` on the live import loop (gated on the one-time Google spike,
     `docs/phase-notes/phase-12-spike.md`, **never run**);
   - the **Android** app build and the in-repo `plugins/vidya-android` native plugin (share sheet,
     camera/gallery compression, AndroidKeyStore wrapping, Google sign-in bridge) — the plugin
     **does not exist** (only Tauri's `gen/android/` scaffolding);
   - the **live Gmail send** client + timer, and the **automatic-WhatsApp** live client + setup UI.

   Everything above has working *offline* foundations (the `DriveApi`/`GmailSender`/`WaSender`
   traits + fakes, sealing/epoch crypto, message outbox, LAN sync) and is tested against fakes, but
   the live glue is not wired.

**Consequence:** v2.0.0 can ship **now as a single-PC desktop release** (Windows/macOS — the full
offline record with every module), but the cross-device Drive sync, Android app, email and
automatic WhatsApp are **not functional** until the owner runs the Google spike and completes the
live clients on a real machine. This is the same deferral chain carried since P12/P14; P18 did not
change it (this phase adds no features — Rule "no new features"). The CHANGELOG `[2.0.0]` entry and
`docs/RELEASE-CHECKLIST.md` both state this build-state plainly.

The live-Drive spike + client is also the shared prerequisite for **Phase 19** (the iPhone PWA
syncs through the same Drive route), so it is the natural next unit of owner-hardware work.

## What this phase did (all committed on `v2/p18`)

### WORK 2 — upgrade & migration safety  (commit `a266067`)
- **Fixed a real v1→v2 upgrade-breaking bug** that P13 had flagged and deferred to P18. Migration
  0012 rebuilds the `request` table with `DROP TABLE request`; on a production v1 DB that holds an
  applied payment reversal, `reversal.request_id → request(id)` (a NO-ACTION FK) makes the DROP's
  implicit DELETE fail with foreign keys on, so the whole upgrade aborts. Fix is in the **migration
  runner** (`src-tauri/src/db/mod.rs::run_migrations`): FK enforcement is turned **off for the
  migration run** (PRAGMA `foreign_keys` is a no-op inside a transaction, so it must be toggled
  around the per-migration transactions) and **on again afterward**, then **`foreign_key_check`**
  runs so a migration that leaves a genuinely dangling reference becomes a hard error instead of
  silent corruption. **No committed migration was edited** (Rule 9). Each migration is still its own
  transaction, so an interrupted run leaves `schema_version` at the last fully-applied version.
- **New `src-tauri/tests/upgrade.rs`** (3 tests, all green) — DONE-MEANS #1:
  - `v1_database_upgrades_with_money_audit_and_history_intact`: builds a realistic finished v1 DB
    (siblings sharing a guardian, legacy `L` marks, confirmed payments across two days and two
    modes, **an applied reversal linked to its approval request** — the exact FK case above) from
    the frozen v1 schema (migrations 0001–0004), captures invariants, runs the v2 chain, and asserts
    **per-day/per-mode money totals identical to the paisa**, the **audit chain still verifies with
    an unchanged head**, **legacy L preserved**, **guardians backfilled + siblings de-duped**,
    **licence still active**, and **balanced + idempotent ledger vouchers** (one receipt voucher per
    confirmed payment). This test fails without the runner fix.
  - `interrupted_upgrade_resumes_from_the_last_committed_migration`: a partial upgrade (killed after
    migration 15) resumes to the latest on next start, data intact.
  - `a_failed_migration_rolls_back_and_leaves_the_schema_version_intact`: an uncommitted mid-migration
    change rolls back on reopen; `schema_version` unchanged; a fresh run then completes.
- The 9 existing `db::tests` (idempotent migrations, the P12 v1-upgrade test, guardian backfill,
  request rebuild, school_id backfill, append-only triggers, FTS, encryption) all still pass.

### WORK 2c — deprecated columns: **owner-deferred**
The prompt asks to drop `staff.google_email` and `student.guardian_name`/`guardian_mobile` "after
checking no code reads them." They are **still read and written** by ~8 sites plus the `student_fts`
search index (+ its 3 triggers) and the join wire protocol, so dropping them is a real refactor, not
a one-line migration. The data already lives fully in the `guardian` table, so the columns are
harmless dead weight. Per the owner's decision this phase, the physical drop is **deferred** to a
post-2.0 cleanup (OWNER-DECISIONS #16) — the right call for a safety-only release phase.

### WORK 8 — version + release machinery  (commit `600ec6c`)
- **Version single-sourced to 2.0.0** (`node scripts/set-version.mjs 2.0.0`): package.json,
  src-tauri/Cargo.toml, crates/vidya-core/Cargo.toml, tauri.conf.json, and (at build) Android
  `tauri.properties` → versionCode 20000. `check:version` OK.
- Verified the release machinery is already v2-correct from P12 (nothing to change): `release.yml`
  has **no `LICENCE_API`**, `RELAY_URL` optional, requires `LICENCE_PUBLIC_KEY` + the two Google
  client IDs, refuses the dev licence key, enforces the size gates, and writes `SHA256SUMS.txt` +
  `site/releases.json`. `write-release-config.sh` and `src-tauri/build.rs` are the hard backstops.
- **Rewrote `docs/RELEASE-CHECKLIST.md` for v2** (it was stale v1.0.0 — it still told the owner to
  deploy a Fly licence server and set `LICENCE_API`, both retired in v2). The v2 checklist uses
  offline licence files (`tools/licence-maker init`), makes the relay optional, states the
  single-PC-desktop build-state, and lists the clean-machine runs.

### WORK 7 — docs  (this commit)
- `CHANGELOG.md`: an honest `[2.0.0]` **draft** entry (Added / Changed / Removed / Fixed) with a
  prominent build-state note. `release.yml` pipes this into the draft release notes.
- `docs/OWNER-DECISIONS.md`: added #16 (deprecated-column deferral) and a P18 note listing every
  still-open decision as a release-notes limitation + the single-PC-desktop build-state.
- `docs/RELEASE-CHECKLIST.md`: rewritten for v2 (above).

### WORK 3 — security & privacy review of the new v2 surfaces
Every surface the prompt lists was reviewed against the real code. The automated gates pass:
`check:logs` OK (no tokens/keys/PINs/recovery keys/full mobiles in logs) and `check:release-clean`
OK (no dev routes/seed/fixtures/keys in the built frontend).

**OK (verified, test coverage present):**
- **Email sender** — the sync-account OAuth refresh token is stored encrypted-at-rest only
  (`drive_account.token_enc`); the `DriveAccount` DTO omits it and `disconnect` clears it; no body
  or token is logged; consent is re-checked at send time (`email.rs` `row_consented`).
- **WhatsApp token** — kept in SQLCipher `settings_json.wa_auto.token`; `get_wa_auto_config` returns
  only `token_set: bool`; the audit `after_json` explicitly excludes the token; never logged (test
  `wa_auto_config_save_read_and_token_never_leaks`).
- **Attachment blobs** — `nonce ‖ ChaCha20-Poly1305`, ≤ 2 MB cap, AEAD + SHA-256 re-hash tamper
  check on read, key = HMAC-SHA256(db_key, label) (distinct from the DB key). Tests cover
  roundtrip/tamper/oversize/wrong-key.
- **Notes folder** — only `homework_note` attachment *metadata* targets the unencrypted `notes/`
  path (migration says "study material only — no student photos or marks"); DB/audit/personal data
  never routed there.
- **Licence-maker never in the app** — `tools/licence-maker` is workspace-`exclude`d and has its own
  `[workspace]`; the app is **verify-only** (`verify_strict`, no non-test `SigningKey`/`sign` for
  licences); it holds only the public key from build-config; `.gitignore` blocks the private
  key/register/`.vlic`. The only shipped `sign_*` is the school server signing its own per-school
  epoch key (legitimate, §8.9).
- **Epoch signatures** — `verify_epoch` checks the ed25519 signature over the raw payload bytes
  before parsing; wrong-key/tamper/garbage → `EPOCH_INVALID` (tested).
- **PIN / DB key / recovery key** — Argon2id hash only / 32 random bytes in the OS keychain /
  Argon2id-derived backup key never persisted in plaintext; none logged.

**Fixed this phase (commit — WORK 3):**
- **Server module-off enforcement (§8.4)** — `sync/apply::apply_op` gated the module switch *and*
  the permission re-check inside `if let Some(action) = action_for(...)`, but `action_for` maps only
  ~5 optional-module tables, so a sync op for **any other** optional-module table (`expense`,
  `salary_*`, `staff_advance`, `store_*`, `circular*`, `period`, `timetable_slot`, `substitution`,
  `report_lock`, `exam_*`, …) **bypassed the module switch and was written even when the module was
  off**. Fixed: a table-based module gate now runs for **every** op using the authoritative
  `scope::module_of_table`, rejecting a disabled module's op with `MODULE_OFF` before any write.
  New tests `module_off_op_is_rejected_server_side_even_for_unmapped_tables` and
  `module_on_op_passes_the_module_gate` (a `store_item` op — an un-mapped table — is rejected while
  store is off, applied when on).

**Also fixed this phase (the security-review follow-ups — commit — WORK 3):**
- **[HIGH] Server permission re-check gap (Rule 7) — FIXED.** The same `action_for`-only structure
  meant the server did **not** re-validate *permission* for ops on the un-mapped module tables. Now
  `action_for` maps `expense`→RecordExpense, `expense_reversal`→ReverseExpense,
  `salary_*`/`staff_advance`→ManageSalary, `store_item`/`stock_move`→ManageStore,
  `store_sale`→RecordStoreSale, `circular`→ManageCirculars, `period`/`timetable_slot`→ManageTimetable,
  `substitution`→ManageSubstitutes, `exam_*`→ManageExamSeating — each with the same target the
  command layer uses (updated `target_for`), so legitimate ops from the right role pass and forged
  ops from the wrong role are rejected. Tests: a teacher's forged `expense` op and the accountant's
  forged `salary_structure` op are both `Rejected` with `FORBIDDEN`; the Principal's `store_item` op
  still applies. Left intentionally un-mapped (module-gated only, or Core reference/server-generated,
  lower risk): `circular_read` (any staff marks read), `report_template`/`report_lock`, `fee_due`,
  `guardian`/`student_guardian`, `consent`, `message`/`message_template` — noted in `apply.rs`.
- **[DPDP] Erase message/notification cache — FIXED (and FTS confirmed clean).** Verified the FTS
  search index **is** correctly refreshed on erase (the `student` UPDATE fires `student_au`; the
  erased student drops out of search — other same-name students are unaffected; the review was
  right). Added: erase now tombstones the erased student's `message` rows (`subject`/`to_address`
  NULL, `body`=`(erased)`) and clears `notification.vars_json` for notifications deep-linking the
  student. Regression test asserts the message is scrubbed and the erased student is no longer
  searchable by id.
- **[MEDIUM] Server-side consent gap — FIXED.** `record_message_logic` now rejects a student-linked
  guardian message (`related_table='student'`) without the student's active `messages` consent
  (mirrors `email::row_consented`, now `pub(crate)` and reused), and `send_queued_wa_auto` re-checks
  consent at send time like the email drain. Tests updated/added.

**Remaining (documented, low priority):**
- **[LOW] Defense-in-depth:** add a release-binary `strings` scan (or port a `no_key_leak` test into
  the app workspace) so licence-maker isolation doesn't rest solely on the Cargo `exclude` +
  `build.rs` dev-key rejection. No leak was found.

### WORK 4 — performance & size
- The Phase 9 dashboard-performance budget **still holds with the full v2 schema** (30 migrations).
  `cargo test -p vidya --test benchmark -- --ignored` at **1,500 students / 60 classes / 20 days
  attendance / 1,500 payments** (seeded in 649 ms):
  | Aggregate | Time |
  |---|---|
  | active students | 0.66 ms |
  | collected today | 0.52 ms |
  | attendance today by mark | 36.06 ms |
  | attendance by class today | 2.06 ms |
  | fee collection last 6 days | 1.42 ms |
  | **Principal Home total** | **≈ 41 ms** (budget ≤ 150 ms) |
- **Carried forward (needs a new bench harness):** the expanded large-school seed the prompt
  describes (60 staff, a *full year* of attendance, instalment fees, 2,000 expenses, 12 salary runs,
  500 circulars, 2 exams with seating) and timing the newer module screens (Accounts→Profit, Dues
  list, Timetable, seating generation, report-card batch print). The current bench covers the P09
  Principal-Home aggregates only; extending it is a focused follow-up.
- **Size:** the `size-report.mjs` gate (≤ 40 MB download / ≤ 50 MB installed, decimal) is unchanged
  and runs in every CI/release job. Per-platform installer measurement (Windows NSIS, Android APKs)
  needs those toolchains and is **owner-hardware**; the macOS DMG/.app can be measured on a Mac.
  The one runtime size add since v1 is the Telugu font subset (`@fontsource/noto-sans-telugu`,
  ~30–60 KB gz across 3 weights), far inside budget.

### WORK 5 — accessibility & languages
- `check:contrast` passes for every token pair (WCAG AA), including the new-screen pairs (gate).
- `check:i18n` green: **30 modules, 1473 keys, en/hi in sync, no TODO-HI**. The gate enforces
  **en/hi**; **Telugu is selectable with an English fallback** (font, pickers, amount-in-words done
  in P13) but the ~1473-key bundle is **not translated** — that is a native-speaker task
  (OWNER-DECISIONS #12; I must not invent copy, Rule 5). "Three languages complete / check-i18n
  3-way" is therefore **deferred to the native review**, not a code gap.
- Device-only a11y checks (Android 130% font scale, TalkBack labels on P/A + check-in + share,
  Telugu/Hindi layout at 360 px) require a running app on a device and are **owner-side verification**
  (the screens are built from the mock tokens; contrast passes programmatically). Listed below.

### WORK 6 — demo seed
The demo seed (`src-tauri/src/seed.rs`) already reproduces the mock/prototype headline numbers
(Saraswati Public School, Meena Iyer, the cash-book day, the salary month, the September calendar,
attendance 91.3%, receipts `R-A2-…`, etc.; extended each module phase P15–P17). Extending it so
**every** prototype state renders identically (e.g. seed guardian rows so the Dues table shows
names — a gap P14 noted) is **carried forward** — it pairs with the Playwright prototype-fidelity
baselines that can only be asserted on the canonical macOS machine (see below), so it is best done
in the same pass on that machine.

## WORK 1 — known issues / deferrals from phase notes 11–17 (the register)
Everything below is **either fixed this phase or an accepted owner deferral**. Items marked
**[owner-hardware]** cannot be built or verified in the CI/dev sandbox (no Android toolchain, no
live Google/Meta, no canonical baseline machine); they are the same deferrals every v2 phase
carried. Items marked **[feature, post-2.0]** are new-feature completions the "no new features"
release rule keeps out of P18.

**Fixed in P18**
- v1→v2 request-rebuild FK break on production DBs with applied reversals (P13 caveat) — fixed +
  tested (WORK 2).

**Owner-hardware (build + verify on the owner's machine; blocks a true multi-device v2.0.0)**
- Google `drive.file` spike + live Drive client + Drive sync UI + live route/timings/epoch loop
  (P12). **The single biggest blocker; also the P19 prerequisite.**
- Android app build + `plugins/vidya-android` native plugin: share sheet (P14), photo compression
  (P15), Google sign-in bridge, AndroidKeyStore wrapping (P04), print (P07).
- Live Gmail send client + timer (P14 Step 3); automatic-WhatsApp live client + setup UI (P14 Step 7).
- Attachment blob transport over LAN/Drive + backup inclusion (P15); live Drive note upload to
  `notes/<class>/<yyyy-mm>/` + retention cleanup (P16).
- Live LAN route detection on a real phone for staff check-in (P17).
- Playwright **prototype-fidelity baselines** for every new screen — `tests/e2e/__screens__` PNGs
  are gitignored and only assertable on the canonical macOS baseline machine (P11/P13/P14/P15/P16/P17).
- Clean-machine acceptance runs (fresh Windows/Android/Mac) and per-platform **installer size**
  measurement (Windows NSIS + Android APKs need those toolchains; the macOS DMG/.app can be measured
  on a Mac). The `size-report.mjs` gate and CI enforce the ≤40 MB download / ≤50 MB installed limits
  on every build.

**Native-language review (OWNER-DECISIONS #12)**
- Full Telugu UI translation (~1473 keys) + extending `check-i18n` to enforce `te`; native review of
  `amount_in_words_te`, the ledger `name_te`, the message/report template `hi`/`te` drafts, and the
  P14–P17 strings + leave-type names.

**Feature completions deferred to post-2.0 (no new features in P18)**
- Teacher class-notice approval flow, circular send-time channel fan-out, per-staff read list +
  "Remind", printed notice with tear-off slip, "Add to calendar", the staff phone **Inbox** screen
  (P14 Step 6).
- Desktop-register variant of absence alerts + "who may send absence alerts" setting (P14 Step 4).
- Custom-field CSV columns; admission-form consent capture + `consent_signed_form=yes` on import (P13).
- Instalment plan re-dating on session rollover (P15); Accountant-view of the salary register (P15).
- Teacher phone remark entry; manual seating pairs + schedule-editor UI; teacher phone calendar
  (read-only) (P16).
- Server-side recompute of `clock_warning`; cross-month unpaid-leave attribution; accountant desktop
  check-in card (P17).
- The **activate** screen visual redesign to the prototype (pixel-exact mock contract — needs owner
  fidelity approval + baseline regen) (P12).
- Reports screen's final home in the Principal nav (P11 open item).
- Deprecated-column physical drop (OWNER-DECISIONS #16, deferred this phase).

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
#1 app identifier · #2 price · #6 Telugu logo · #7 salary formula (default built) · #8 leave quotas
(default built) · #9 remote check-in (default built) · #10 retention · #11 UPI QR image · #12 native
Hindi/Telugu review · #13 gmail.send production verification · #14 code signing · #15 GST.
**Decided this phase:** #16 defer the deprecated-column drop.

## Exact commands to re-verify / rebuild
```
# full test + lint + gates (all green on this branch)
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo tree -i aws-lc-rs          # must print nothing (ring only)
npm run verify                   # tsc + hex + i18n + deps + version + contrast + logs + vitest
npm run check:release-clean
node scripts/set-version.mjs --check

# just the upgrade-safety suite
cargo test -p vidya --test upgrade

# release (owner): see docs/RELEASE-CHECKLIST.md — mint the licence key, set the GitHub
# Variables, then:  git tag v2.0.0 && git push --tags   (starts .github/workflows/release.yml)
```

## STOP conditions (prompt) — status
- "Any upgrade test that changes money or audit data" — the upgrade test **proves the opposite**
  (money identical to the paisa, audit chain unchanged). No STOP.
- "Any size limit exceeded" — the size gates are unchanged and pass on the buildable target; the
  per-platform installer measurement is owner-hardware. No known breach.
- "Any feature that would need a company server" — none; v2 remains zero-cost (offline licences,
  the school's own Drive, an optional relay).
- **Reported, not a silent pass:** v2.0.0 is not a working *multi-device* product until the
  owner-hardware live-Drive/Android/Gmail work is done (see the top of this file). This is a
  build-state disclosure, not a regression.
