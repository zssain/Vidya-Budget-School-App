# Phase 8 handoff — backups, restore, session rollover (core rules + backend) + full Hindi (Part F)

> **Update (second session):** Part F (full Hindi) is now complete — see the
> **Part F** section at the end. The rest of this document covers the first
> session (Parts B/C/D backend + rules).

Branch: `rebuild/p08` (from `rebuild/p07`). Start HEAD: `b64df2b` (P07 close-out).
`git status` at start: clean. Tools: rustc/cargo/clippy 1.98.1, node v25.3.0, npm 11.7.0.

## Scope of THIS session (owner-chosen focus)

Phase 8 is a whole phase (Parts A–F + 5 flows). Given the sandbox's hard blockers
(no Java/NDK/Android device, no Google accounts/Cloud project, no Tauri window —
carried over from P06/P07, still true), the owner chose **"Core rules + backend,
fully tested"** for this session. So this handoff delivers the parts that are
**provable offline now**, all with passing tests, and nothing faked:

- **Part B (Backups)** — the backup engine: SQLCipher export, verify, Drive
  upload, retention, scheduler decision.
- **Part C (Restore)** — the restore engine: summary, staged install with atomic
  swap + safety copy, fencing (`server_epoch + 1`, `needs_rejoin`).
- **Part D (New session)** — the rollover engine: promotion, carry-forward, new
  dues, in one transaction with one audit entry (tested at 1,500 students).
- The **pure rules** all three depend on, in `vidya-core` (rule 6/9: business
  rules only in vidya-core).

**Deferred (not built this session, no code faked):** Part A teacher phone screens
(React), Part E Settings + Storage screens (React), Part F Hindi translation +
`scripts/check-i18n.mjs`, the thin Tauri **commands** that wire these engines to
the UI, and the 5 end-to-end flows / Playwright screenshots. These need either the
React/UI pass or the blocked environment; see **What's left** below.

## The gate (all green, real output this branch)

- `cargo test --workspace` → **483 pass, 0 fail** (was 420 at P07; **+63**):
  vidya lib **159** (+23: 15 backup · 5 restore · 3 session), vidya-core **280**
  (+37), drive_e2e 8, e2e_flows 4, relay_e2e 5, sync_e2e 13, no_floats 1, doc **13**
  (+3). benchmark `#[ignore]`.
- `cargo clippy --workspace --all-targets -- -D warnings` → **clean**.
- Crypto gate: `cargo tree -i aws-lc-rs` → *not found* (absent); `ring` is the sole
  TLS provider — unchanged. No new dependencies added.
- `npx tsc --noEmit` → clean · `npx vitest run` → **38 pass** · `node
  scripts/check-hex.mjs` → OK (42 tokens). (No TS/CSS changed this phase.)

## What was built (files + what each proves)

### vidya-core (pure: no IO, no clock, no floats) — `crates/vidya-core/src/`
| Module | Key API | Tests |
|---|---|---|
| `backup.rs` | `select_for_deletion(runs, today)` (30 daily + 12 monthly **per destination**), `slugify`, `backup_filename`, `backup_outcome` (Verified/Partial/Failed) | 11 |
| `session.rs` | `next_class` ladder (Nursery→LKG→UKG→I…→XII→Passed out), `build_promotion` (overrides repeat/leave; sections kept; off-ladder→Unknown), `summarize`, `carry_forward` (Previous-balance dues, linked to old dues) | 12 |
| `restore.rs` | `validate_restore` (audit chain must verify), `staleness_warning` (older than 1 day), `recovery_lockout_seconds` (3 wrong → 30 s), `days_between` (Howard-Hinnant, leap-safe) | 14 |

Registered in `crates/vidya-core/src/lib.rs`.

### src-tauri backend (IO shell over the pure rules) — `src-tauri/src/`
- **`backup/mod.rs`** (15 tests):
  - `is_backup_due(last_verified_date, today, hour, on_quit)` — 06:00 daily + on
    quit if none today (pure decision, clock passed in).
  - `export_backup(src, dest, backup_key_hex)` — real SQLCipher `ATTACH … KEY` +
    `sqlcipher_export`; writes a temp sibling, **fsync**, atomic **rename**; leaves
    nothing behind on failure. **Verified it round-trips with our full schema
    (FTS5 + append-only triggers).**
  - `compute_expectations` + `verify_backup` — open read-only, `PRAGMA
    integrity_check`, domain row counts, audit-chain head. **Failure paths tested:**
    wrong key, corrupted file, row-count mismatch, chain-head mismatch.
  - `upload_and_verify(drive, folder, name, bytes)` — temp-name upload +
    verify-by-readback + rename over the P06 `DriveApi` trait (proven on the
    fault-injecting `FakeDrive`); quota-full surfaces cleanly.
  - `parse_backup_at` + `retain_local` / `retain_drive` — apply
    `select_for_deletion` to the actual files (local dir + Drive folder).
  - `run_backup` — export → local verify → optional Drive copy → `backup_outcome`.
    Tested: local-only → **Partial**; PC + Drive → **Verified**; Drive
    unavailable → **Partial** with the local copy still verified + kept.
- **`backup/restore.rs`** (5 tests):
  - `build_summary(vbak, backup_key)` — school, backup date (from filename),
    students, payments, last receipt no., `chain_ok` — read-only, never mutates
    the `.vbak`. Feeds vidya-core `validate_restore` / `staleness_warning`.
  - `install(vbak, backup_key, live_db, new_db_key, now)` — **staged**: re-encrypt
    the backup into a temp live DB under a **new DB key** (copying with
    `sqlcipher_export('main','bak')` so the `.vbak` is only read), verify it, apply
    fencing (`server_epoch + 1`, every `device.needs_rejoin`, one `restore` audit
    entry), then **swap atomically**, keeping the previous DB as a
    `*.pre-restore-<stamp>` **safety copy**.
- **`session.rs`** (3 tests): `rollover_commit(conn, params)` — one transaction:
  new session (current) + terms, old session read-only; promote/repeat/leave/pass-
  out per plan (creating destination classes on the fly, sections kept); carry each
  enrolled student's unpaid balance forward as one **Previous balance** due in new
  Term 1 (linked via `fee_due.carried_from_due_ids`, old dues kept); generate new
  term-head dues; **one** `session_rollover` audit entry. **Tested at 1,500
  students** (all promoted V→VI, 500 carry a balance, 3,000 new dues, chain valid).

### Migration
- `src-tauri/src/db/migrations/0004_p08.sql` — `ALTER TABLE fee_due ADD COLUMN
  carried_from_due_ids TEXT` (the carry-forward link). New migration only; no
  earlier migration edited (rule 9). `db::MIGRATIONS` updated (now 4); the
  idempotent-migration test still passes.
- `db::open_encrypted_readonly` added (verify/restore inspect a `.vbak` without
  switching it to WAL).

## Design decisions I made (please confirm) + [OWNER] questions

1. **Retention is count-based GFS** (the 30 most-recent backup-**days** + the 12
   most-recent backup-**months** per destination), not a fixed 30-day/12-month
   calendar window — so a school never loses its latest 30 days / 12 months even if
   backups are sparse. `today` drops future-dated runs. Confirm this reading of
   "30 daily + 12 monthly".
2. **`backup_outcome`** — `Verified` requires both the local copy AND a Drive copy
   to verify; local-only (Drive unavailable **or not configured**) → `Partial`.
   Confirm a school with **no Drive at all** should read `Partial` (vs a local-only
   `Verified`).
3. **Backup-key provisioning + salt (real crypto gap, flagged not invented).** The
   backup must be encrypted with the backup key = `Argon2id(recovery key, salt)`,
   but an unattended 06:00 run has no recovery key, and a restore on a fresh PC
   needs the salt *before* it can open the file. Every function here takes the
   backup key as a **parameter**, so the mechanics are done and tested; the
   provisioning is the open decision. **Recommended default:** derive the backup
   key at setup and cache it in the OS keychain (account `backup-key`), and write
   the non-secret salt + school id in a plaintext `<file>.vbak.meta` sidecar next to
   each backup so restore can derive-and-verify. Needs your OK before wiring.
4. **Rollover fee generation** — generates **term-head** dues for the new session's
   terms; **once** heads are not re-charged on promotion, and **month** heads are
   left to the monthly scheduler (consistent with the P07 admission-dues decision).
   Confirm.
5. **Carry-forward** = each enrolled student's total outstanding across all
   non-cancelled dues (amount − allocations) > 0 → one Previous-balance due for that
   sum, linked to those due ids. Left/passed-out students do **not** get a carry-
   forward due (no new enrollment); their old dues remain. Confirm.
6. Prior defaults kept (attendance % = P÷(P+A+L); lease 30 d; accent `#2F7479`;
   `[Company name]` placeholders). The P07 open questions (Reports placement, etc.)
   are unchanged by this session.

## What's left in Phase 8 (for the next session)

- **Part A — Teacher phone screens** (React): the 9 tiles (Attendance picker,
  Marks list, Report cards, My classes, Students, My requests, Inbox, Sync,
  Profile), computed due cards + pulse, Android back handling. UI pass.
- **Part E — Settings (all sections) + Storage** (React): School, Session & terms
  (+ the rollover wizard UI on top of `session::rollover_commit`), Classes &
  subjects, Grade scale, cut-off/threshold, Appearance, Language, Printing,
  Security, Drive, Connection, Licence, Storage, About.
- **Part F — Hindi**: real natural Hindi for every i18n key (`hi` is still
  `TODO-HI:` placeholders), the `:root[lang="hi"]` Newsreader→Noto swap + 164px→
  `min-width` fix, and a new `scripts/check-i18n.mjs` gate. Self-contained; fully
  verifiable next session.
- **Command wiring**: thin, audited Tauri commands that call the engines above
  (`backup_now`, `list_backups`, `restore_summary`, `restore_install`,
  `session_rollover_preview`, `session_rollover_commit`, the scheduler tick), plus
  the `commands.json` ↔ `COMMANDS` ↔ `api.ts` consistency entries. Decision #3 must
  be settled first for the backup/restore commands.
- **Environment-blocked (cannot run or honestly claim here):** the real Android
  APK + phone print plugin, real Google Drive OAuth transport, actual OS print
  dialogs, and Tauri-window Playwright screenshots (into
  `docs/phase-notes/phase-8-screens/`). Same blockers recorded in P06/P07.
- **The 5 end-to-end flows** depend on the UI + command layer above.

## Questions for the owner

1. **Approve design decision #3** (backup-key cache in keychain + salt sidecar) so
   the backup/restore commands + scheduler can be wired.
2. Confirm decisions #1, #2, #4, #5 above.
3. Next-session priority among the deferred items: **Part F (Hindi)** is the most
   self-contained and fully verifiable without the blocked environment; **Part A**
   (phone screens) and **Part E** (settings) are the largest UI surfaces. Which
   first?

---

# Part F — Full Hindi (second session)

Owner delegated the call ("you choose the best option"); Part F was the
blocker-free, fully-verifiable next slice, so it was done next.

## What was built

- **Every i18n string module translated to natural Hindi.** All 17 modules in
  `src/lib/i18n/strings/` now carry a real `hi` value for every key — **716 keys,
  0 `TODO-HI` left anywhere in `src/`**. Translations use everyday school Hindi
  with a consistent glossary (उपस्थिति/अंक/फ़ीस/रसीद/प्रधानाचार्य/शिक्षक/लेखाकार/
  विद्यार्थी/कक्षा…), preserving every `{placeholder}`, ₹, receipt numbers
  (`R-A2-0419`), admission numbers (`2026/0142`), class displays (`VII-B`),
  session labels (`2026–27`), and the product name **Vidya**. The auto-generated
  `Object.fromEntries(… TODO-HI …)` `hi` blocks were replaced with explicit maps.
- **`scripts/check-i18n.mjs`** (new gate): loads each module (strips TS types →
  data-URL import, so it evaluates the real maps, both file patterns), and fails
  on any en/hi key mismatch, any `TODO-HI`, any empty value, or a key defined in
  two modules. Result: **OK — 17 modules, 716 keys, en/hi in sync, no TODO-HI.**
- **Hindi font swap** (`docs/01-MOCK-SPEC.md §3): the `:root[lang='hi'] .v-serif`
  rule (Newsreader → Noto Sans Devanagari 500, `font-style: normal`) and the
  `--font-serif` var already existed in `src/styles/app.css` — confirmed present
  and correct.
- **Fixed 164px badge → `min-width: 164px`** (`PrincipalHomeScreen` `pillStyle`,
  the approval badges) so Hindi never truncates. **Proven baseline-neutral:**
  principal-home renders **byte-identically** (104225 diff px either way) with
  `width` vs `minWidth`, because every English badge label is ≤164px.

## Gates (real output, this branch)

- `node scripts/check-i18n.mjs` → **OK (17 modules, 716 keys, no TODO-HI)**.
- `grep -rn "TODO-HI" src/` → **nothing** (comments updated too).
- `npx tsc --noEmit` → clean · `npx vitest run` → **38 pass** ·
  `node scripts/check-hex.mjs` → OK (unchanged).
- Rust workspace untouched this session (still 483 pass / clippy clean from the
  first session).

## Fidelity screenshots — environment limitation (honest)

`npx playwright test tests/e2e/fidelity.spec.ts` → **12 failed**, but as
**pixel diffs of ~3–7%** (threshold 0.1%) on **every** screen — **including
`welcome-setup`, which this session did not touch at all** (0.03 ratio). The
committed `*-darwin.png` baselines were generated on a different rendering
environment and do not reproduce on this sandbox's font hinting / antialiasing —
the same pixel-verification limitation P07 recorded. **My changes are not the
cause:** an untouched screen fails identically, and principal-home's diff is
byte-identical with the old `width` and the new `minWidth`. Re-baselining, if
wanted, must be done on the reference machine (or in CI), never here. No baseline
was modified.

## Phrases flagged for owner review (Hindi choices that are debatable)

- **Transliterations kept as-is** (common in Indian schools, but confirm):
  "Day book" → `डे बुक` (alt: `रोकड़ बही`), "Sync" → `सिंक`, "Draft" → `ड्राफ़्ट`,
  "Backup" → `बैकअप`, "Restore" → `रिस्टोर` (alt: `पुनर्स्थापित करें`),
  "Report card" → `रिपोर्ट कार्ड`, "Grade/scale/point" → `ग्रेड/स्केल/पॉइंट`.
- "Advance credit" → `अग्रिम जमा`; "Reversal" → `भुगतान वापसी`.
- `acc.title` "Good day." → `नमस्ते।` (a natural greeting, not a literal
  "शुभ दिन").
- Demo/fixture proper nouns transliterated: "Saraswati Public School" →
  `सरस्वती पब्लिक स्कूल`, "Priya Sharma" → `प्रिया शर्मा` (real deployments show
  the school's own stored name, not this key).
- Fixed mock date strings translated (e.g. `बुधवार, 23 सितंबर`); live dates come
  from `format.ts`, not these keys.

## Part F — font swap + logo lockups (done, "fix all")

After the translation, the two font/logo remainders were fixed and verified:

- **Serif font swap now actually reaches every heading.** All 48 inline serif
  usages across the UI screens (desktop + phone + shell + welcome) were routed
  through the existing `var(--font-serif)` CSS variable, and `app.css` now swaps
  that variable under Hindi:
  `:root[lang='hi'] { --font-serif: 'Noto Sans Devanagari', sans-serif;
  font-synthesis: none; }` — so Hindi headings render in Noto Sans Devanagari with
  **no synthetic italics** (mock-spec §3). `var(--font-serif)` resolves to the
  identical Newsreader stack in English. **Proven byte-identical in English:**
  welcome-setup (41377 px) and principal-home (104225 px) diff ratios are
  unchanged from before the edit. `CollectFeeScreen` keeps its `.v-serif` class
  (Noto **500** per §3); the `.v-serif` rule is retained, so applying that class
  to the other screens' headings is the (optional) way to get the exact 500 weight
  there too — otherwise they render Noto at the heading's own weight.
- **Hindi logo lockups wired.** `vidya-horizontal-hindi-on-{dark,light}.svg` copied
  into `src/assets/`; `AppShell` (sidebar), `TeacherHomeScreen` (header) and
  `WelcomeScreen` (left panel) now pick the Hindi lockup when `getLang() === 'hi'`,
  the English lockup otherwise. **Byte-identical in English** (welcome-setup +
  teacher-home ratios unchanged).

Still deferred (needs Part E / report-language, not a Part F loose end):

- **Report-language** — receipts / report cards / reports in the chosen *report*
  language (independent of the UI language) needs the Settings → Report language
  control (Part E) + the print docs reading it. The 3 print docs
  (`ReceiptDoc`/`ReportCardDoc`/`DayBookDoc`) were deliberately **left on the
  Newsreader stack** (not routed to the UI-language var) so print follows report
  language, not UI language, once that is wired.
- Exact **Noto 500** weight on non-CollectFee Hindi headings (apply `.v-serif`) —
  a cosmetic refinement; flagged for owner.
