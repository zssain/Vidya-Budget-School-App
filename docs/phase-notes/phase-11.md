# Phase 11 handoff — re-baseline for v2: merge decisions, attendance P/A, no cut-off, module switches

## Start state
- Branch `v2/p11`, cut from `rebuild/p10` @ `fe92b07` (Phase 10 tip). HEAD at write time `f959045`.
- `git status` at start: clean tree; untracked v2 planning inputs (`docs/02-V2-CHANGES.md`,
  `docs/03-PROTOTYPE-SPEC.md`, `design/prototype/VidyaPrototype.jsx`, `prompts/P11–P18`) — all
  now committed on this branch.
- Tools: rustc / cargo / clippy **1.98.1**; node **v25.3.0**; npm **11.7.0**.
- No git tags exist → **no public release has happened** (relevant to the app-identifier decision).
- Repo conventions differ from the run-skill defaults: there is **no `AGENTS.md`** and **no
  `docs/PROGRESS.md`**; progress lives in `docs/phase-notes/phase-N.md`; prompts are in `prompts/`.

## Step 0 — Inventory
`docs/phase-notes/phase-11-inventory.md` catalogues every 02-V2-CHANGES area (§2 platform, §3
attendance, §6 Drive, §7 licence, §8 foundation 1–12, §9 privacy, §10/§11 modules) with
exists/partly/missing, the files/commands/tables/tests involved, and what a later phase must
change. Headlines that shaped this phase:
- The **attendance % formula already matched the v2 owner default** — `percent_present(p,a,l)`
  is `P÷(P+A+L)`, i.e. legacy L already counts as absent. No formula change was needed.
- There was **no attendance cut-off setting or low-attendance threshold** in the code — only a
  dev-fixture string ("usually done by 10:30") and a report copy string ("Below 75%…").
- **Branding was already largely done in P09** (publisher/copyright in `tauri.conf.json`, Welcome
  footer, INSTALL.md).
- Relay is engaged by the **school server opening its tunnel** (`server/start.rs`), so
  `instant_sync` gates that single point.

## Step 1 — One source of truth (merged-context diff summary)
- `git mv docs/00-SYSTEM-CONTEXT.md docs/archive/00-SYSTEM-CONTEXT-v1.md`.
- New `docs/00-SYSTEM-CONTEXT.md` = v1 text with every 02-V2-CHANGES decision folded in, superseded
  v1 rules removed: **§0** business/branding/zero-cost + app identifier; **§1** product paragraph
  (offline licence, LAN→Drive, relay optional, English/Hindi/**Telugu**); **§3** rules (Telugu,
  zero-cost, privacy, vouchers); **§4a** languages + Noto Sans Telugu; **§5** roles-as-data +
  attendance-duty; **§7** data model (attendance L is legacy-only for new marks; `schema_meta`;
  **§7a v2 tables to add** incl. `module_setting`); **§8** sync (LAN→Drive→relay, module gate,
  cutover); **§9** security **+ privacy (DPDP)**; **§10** offline licence + static site; **§11**
  two-account Drive; **§13** deps (+`@fontsource/noto-sans-telugu`, relay optional); **§14**
  modules; glossary + owner-decisions pointer.
- `02-V2-CHANGES.md` kept as the decision record with a "merged in Phase 11" note at the top.
- New `docs/OWNER-DECISIONS.md` register (15 rows; decision · default · where it lives · status ·
  answer).
- `01-MOCK-SPEC.md` gained a **"v2 amendments"** section (attendance P/A, no cut-off copy,
  prototype is the reference for new screens).

## Step 2 — App identifier (report only)
`in.vidyabudget.app` — `src-tauri/tauri.conf.json` (`identifier`), Android `namespace` +
`applicationId` (`gen/android/app/build.gradle.kts`), macOS `CFBundleIdentifier`. **Not changed.**
No owner answer is recorded and **no public release exists** (no git tags), so no STOP condition
fired. Recorded as OWNER-DECISIONS #1 ("must decide before the first public release").

## Step 3 — Branding
- Settings → About now shows **"Developed by Zuhair Hussain"**, **"© 2026 Zuhair Hussain"**,
  support email `mohammedzuhairhussain28@gmail.com`, website `vidya.zuhairhussain.com` (new i18n
  keys en/hi).
- `tauri.conf.json` homepage → `https://vidya.zuhairhussain.com`.
- `INSTALL.md` + `ADMIN-GUIDE.md`: developer credit + support email + product website; removed the
  `[Company name]` / `[support contact]` placeholders from ADMIN-GUIDE.
- No phone number anywhere (verified). **Receipt footer deferred to P14**: `ReceiptDoc.tsx` has no
  footer band today, and the prompt only adds the credit "if the receipt template has a footer" —
  the receipt already shows the *school's* name/address, not developer branding.

## Step 4 — Attendance: Present / Absent only  (+ cutover constant)
- **vidya-core `attendance.rs`**: new `validate_new_mark(Mark)` → P/A ok, `L` →
  `VALIDATION{field:"mark", rule:"p_or_a"}`; new `parse_mark(&str)` still reads legacy `L`. The
  percentage doc was reframed (denominator `P+A+L_legacy`, legacy L counts as absent); the formula
  is unchanged. Mark enum keeps `L` for legacy reads.
- **Write path** (`commands/logic.rs`): `upsert_sheet_and_marks` validates every mark P/A;
  `correct_attendance_mark_logic` restricted to P/A (was `P|A|L` → now rejects `L` with `p_or_a`).
- **Sync cutover** (`sync/apply.rs` + `protocol.rs`): an `attendance_mark` op carrying `mark:"L"`
  whose HLC `wall_ms` is **≥ the cutover** is rejected with reason code **`LEAVE_MARK_REMOVED`**;
  pre-cutover L ops apply as legacy.
- **Cutover constant** = **`2026-09-25T00:00:00Z`**, written by migration `0005_v2_attendance_pa.sql`
  into a new `schema_meta` KV table as both `attendance_pa_cutover_iso` and
  `attendance_pa_cutover_ms` (Unix ms, compared against the HLC's leading 13 digits). **No existing
  attendance mark is changed.**
- **UI**: phone `AttendanceScreen` and desktop `AttendanceRegisterScreen` lose the L button, the
  Leave count and the Leave bar segment; help text "34 students · tap P or A"; a legacy `L` shows a
  muted **"Leave (old)"** pill. Fixture `initialMarks` seeds no L. i18n `att.*` / `areg.*` updated
  (retired `att.count.leave`, `att.aria.leave`, `areg.leave`; added `att.leaveOld`, `areg.leaveOld`).
- **Mock** `design/screens/Attendance.dc.html` updated (owner-approved v2 amendment): L button /
  count / bar / hint removed and the sample data no longer seeds L. See "Fidelity baselines" below.

## Step 5 — No cut-off, no low-attendance threshold
- There is **no cut-off or threshold *setting* to remove** — none existed (confirmed by a repo
  sweep; the only `cutoff` left is backup-retention in `reliability.rs`, unrelated).
- Home "Needs attention": the dashboard `attendance_pending` now carries the **class teacher**
  (new `dash::PendingClass{class, teacher}`), so the sub-line is **"Class teacher <name> · not
  submitted yet today"** (prototype), falling back to "Not submitted yet today." The dev fixture's
  "usually done by 10:30" is gone.
- Reports: `reports.att.note` no longer says "Below 75% is highlighted" — it just describes the
  percentage (there was never any threshold-highlighting logic, only the copy).
- Absent names keep the existing absent colour in the register/day list — no alerts, no thresholds.

## Step 6 — Module switches (keys, mapping, scope exclusion)
- **Migration `0006_v2_modules.sql`**: `module_setting(key PK, enabled, changed_by, changed_at)`
  seeded with **§11 defaults** — `accounts, classroom, hr, circulars` **ON**; `wa_auto, store,
  instant_sync` **OFF**. Core has no row and is always on.
- **vidya-core `modules.rs`**: `enum Module`; `module_for(action)` maps **every** `Action`
  **exhaustively** (all Core in P11 — no built feature belongs to a toggle-able module yet, so no
  existing command breaks; a new action forces a mapping decision at compile time);
  `require_module`/`require_enabled` → `MODULE_OFF{module}` (new `CoreError::ModuleOff`).
- **Enforcement**: server `sync/apply.rs` loads the enabled set and rejects a disabled module's op
  with `MODULE_OFF` before the permission check ("every server endpoint checks it, one helper").
  `src-tauri/modules.rs` provides `enabled_set` / `is_enabled`. Per-command gating lands with each
  module's own commands (P14–P17) via the same helper; in P11 all commands are Core.
- **Scope exclusion**: `sync/scope.rs` `module_of_table` (all Core today) + `table_in_scope`
  exclude a disabled module's tables from `snapshot`/`visible_row` — the mechanism is wired and
  ready for P14–P17 tables.
- **Instant sync**: `server/start.rs` opens the relay tunnel only when `instant_sync` is on;
  `relay_url` is now **optional** in the release build check (`build.rs`).
- **Settings → Languages & modules** (prototype settings state 3): a Core-locked row + toggles for
  accounts / classroom / hr / circulars / wa_auto / store, each with a **confirmation dialog**;
  changes are **audited** (`set_module` writes `module_setting` + an audit entry). `instant_sync`
  is **data-only** in P11 (not shown in the card — matches the prototype; it is a paid add-on wired
  later). New commands `list_modules` / `set_module` (Principal-only; registered in `commands.json`
  + both `generate_handler!` lists). The Languages card shows English + हिंदी (Telugu in P13).

## Step 7 — Principal sidebar (prototype NAV_PRINCIPAL)
- `PRINCIPAL_NAV`: **Overview**[Home, Approvals] · **Academics**[Students, Attendance, Marks &
  exams] · **Finance**[Fees] · **School**[Staff, Backups, Settings]. Unbuilt items are **hidden**:
  Timetable & Calendar (P16/P13), Accounts & School store (P15), Circulars (P14).
- **Sync & devices** removed from the sidebar — reachable via Settings' existing "Sync & devices"
  link and the header sync pill; `/sync` now highlights Settings.
- **Day book** and **Reports** are reachable as actions on the Fees screen; their routes highlight
  Fees (Day book becomes an Accounts tab in P15). `nav.marks` → "Marks & exams".
- **Fidelity note**: `PrincipalHomeScreen`'s `chrome={true}` sidebar (the pixel-exact `Main.dc.html`
  contract used only by the fidelity gallery) is unchanged; only the **real app's** AppShell nav
  moved to the prototype grouping (`chrome={false}`). So the sidebar divergence between the v1 mock
  and the v2 app is intentional and does not affect the principal-home fidelity render.
- **Open item flagged**: Reports has no home in the prototype `NAV_PRINCIPAL`; it is parked under
  Fees/Finance for now (owner to confirm its final placement — reporting largely folds into
  Accounts/Fees in P15).

## Step 8 — Tests
Every command below was run; results are real.
- **vidya-core** (`cargo test -p vidya-core`): **new** — `validate_new_mark` P/A, legacy-`L`
  `p_or_a`, `parse_mark` reads legacy L; module mapping (`every_action_maps_to_a_module`),
  `require_enabled` MODULE_OFF on/off, keys/defaults. Existing L-in-denominator % test kept (value
  unchanged, comment reframed).
- **src-tauri**: `modules.rs` — seeded §11 defaults + toggle. `sync/apply.rs` — **cutover**:
  a post-cutover `L` op → Rejected `LEAVE_MARK_REMOVED` (nothing written); a pre-cutover `L` op →
  Confirmed (applied as legacy); a post-cutover `P` op → Confirmed. `commands.json` ↔ `COMMANDS`
  parity test still green with the two new commands.
- **Playwright**: `phase1.spec.ts` attendance interaction passes and now asserts **no Leave button**
  and the **"34 students · tap P or A"** hint (reliable, non-screenshot).
- **Full suite (real output):**
  - `cargo test --workspace --locked` → **0 failed** (P01–P10 + new; ~512 assertions across lib /
    integration / doc tests).
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` → **clean**
    (fixed one MSRV slip: `Option::is_none_or` → `map_or`, MSRV 1.77.2).
  - `cargo tree -i aws-lc-rs` → **empty** (ring only).
  - `npm run verify` (tsc + check:hex + check:i18n + check:deps + check:version + check:contrast +
    check:logs + vitest) → **green**; i18n **814 keys, en/hi in sync**; vitest **38 passed**.
  - `npm run check:release-clean` → **OK** (no dev routes/seed/fixtures/keys in the build).

### Changed test expectations (with reasons)
1. `src-tauri/src/seed.rs` dashboard test: `attendance_pending` was `vec!["VII-B"]` → now
   `vec![PendingClass{class:"VII-B", teacher:Some("Meena Iyer")}]` (Step 5 added the class teacher).
2. `crates/vidya-core/src/attendance.rs` `percent_leave_counts_in_denominator`: **value unchanged**
   (800); only the comment was reframed to "legacy L counts as absent".
3. `commands/logic.rs` `correct_attendance_mark_logic`: rejects `L` (rule `p_or_a`) instead of the
   old `invalid`; existing correction tests use P/A, so they still pass.

### Fidelity baselines (important)
- The `tests/e2e/__screens__/…` PNGs are **gitignored** (`.gitignore:55`) — they are **not
  committed**; the fidelity spec regenerates each baseline **from the mock** on first run in every
  environment. So updating `Attendance.dc.html` (Step 4) and `Main.dc.html` (Step 5,
  "not submitted yet today") **is** the baseline change; there is nothing to commit.
- The pixel-fidelity screenshot suite **cannot be asserted-green in this sandbox**: the untouched
  `welcome-setup` screen diffs **~4%** vs its baseline here, i.e. this machine's font rendering
  differs from the canonical baseline environment. **Action for the owner/CI:** on the canonical
  macOS baseline machine, run `npx playwright test tests/e2e/fidelity.spec.ts --update-snapshots`
  to (re)generate `attendance-default`, `attendance-markall`, `attendance-submitted` and
  `principal-home` from the updated mocks, then assert. Playwright *interactions* (mark-all/submit)
  do run correctly here.
- A Playwright **Settings → Languages & modules** screen test was **not added**: Settings is driven
  by live Tauri commands (`list_modules`), which the vite/Playwright harness has no backend for, and
  there is no gallery fixture entry for it. The module system is covered by the vidya-core + src-
  tauri unit/integration tests, `typecheck`, and `check:i18n`. Adding a gallery/mock harness for
  Settings is a small P12+ follow-up.

## Migrations added
- `0005_v2_attendance_pa.sql` — `schema_meta` KV + the attendance P/A cutover (ISO + ms).
- `0006_v2_modules.sql` — `module_setting` seeded with §11 defaults.
Both are additive; no committed migration was edited; no financial/audit rows touched.

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
1. **App identifier** (before first public release). 2. Price. 6. Telugu logo lockup. 7. Salary
deduction formula. 8. Leave types/quotas. 9. Remote staff check-in. 10. Retention for students who
left. 11. Owner's UPI QR image. 12. Native-speaker review of Hindi/Telugu. 13. Google verification
for `gmail.send`. 14. Code signing. 15. GST/invoice on bills. **Plus (new this phase):** the final
home of the **Reports** screen (parked under Fees for now, as it is absent from the prototype nav).

## What Phase 12 needs
- Phase 12 = zero-cost Drive / licence / static site. First step is the **[VERIFY]** Drive spike
  (§11): one real sync account + 3 devices proving `drive.file` cross-device read/write and the
  Android sign-in method — **stop and report if it fails.**
- Retire the P10 online `cloud/licence` (move to `cloud/_archive/`), build offline `.vlic` files,
  the machine-code screen, and the `tools/licence-maker/` CLI (owner laptop only).
- Replace the checkout site with a static `site/` (UPI QR + `mailto:`, reads `releases.json`).
- `relay_url` is already optional (Step 6) and `instant_sync` is off by default, so a release
  without a relay is valid now.
- When the first non-core module ships (P14 Circulars/Communication), wire its **commands** through
  `vidya_core::modules::require_module` and add its **tables** to `scope::module_of_table` — the
  server apply gate and scope exclusion are already in place, so `MODULE_OFF` and pull-exclusion
  become exercisable end-to-end then.
- Regenerate the four changed fidelity baselines on the canonical machine (see above).
</content>
