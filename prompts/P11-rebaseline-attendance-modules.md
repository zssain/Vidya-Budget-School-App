# PHASE 11 — RE-BASELINE FOR V2: MERGE DECISIONS, ATTENDANCE P/A, NO CUT-OFF, MODULE SWITCHES

## ROLE
You are a senior Tauri 2 + React + Rust engineer continuing **Vidya Budget School**. Phases 1–10
are built and working. This is the first phase of v2.

## STANDING RULES (same in every v2 phase)
1. Read IN FULL before anything else: `docs/00-SYSTEM-CONTEXT.md`, `docs/01-MOCK-SPEC.md`,
   `docs/02-V2-CHANGES.md` (wins over 00 where they differ, until this phase merges them),
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
14. Run every command you mention and paste real output. Branch `v2/p11`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-11.md`.

## OBJECTIVE
Bring the code and docs in line with the v2 decisions before any new module is built: one merged
source of truth, an inventory of what exists, attendance with Present/Absent only, no attendance
cut-off, new branding, module switches, and the new Principal menu layout.

## DONE MEANS
- `docs/00-SYSTEM-CONTEXT.md` is the merged v2 context; the v1 copy is archived.
- `docs/phase-notes/phase-11-inventory.md` lists what exists for every area in 02-V2-CHANGES.
- `docs/OWNER-DECISIONS.md` lists every open owner decision with its default and status.
- Attendance offers P/A only on phone and desktop; legacy L marks display as "Leave (old)".
- No cut-off setting or copy anywhere; Home says "not submitted yet today".
- Module switches work end to end (hide screens, block commands, exclude synced tables).
- Principal sidebar matches the prototype's grouping (unbuilt modules hidden, not placeholders).
- "Developed by Zuhair Hussain" in About, installers and receipts footer.
- All P01–P10 tests green (with listed, justified expectation updates) + new tests green.

## STEPS

### Step 0 — Inventory (write before changing code)
Create `docs/phase-notes/phase-11-inventory.md` with one table per area of 02-V2-CHANGES
(§2 platform pieces, §3 attendance, §6 Drive, §7 licence, §8 foundation items 1–12, §9 privacy,
§10 modules). For each item: exists / partly / missing; the files, commands, tables and tests
involved; and what a later phase must change. Include how the relay, `cloud/licence` and the
company website are wired into the app, CI and release today.

### Step 1 — One source of truth
1. `git mv docs/00-SYSTEM-CONTEXT.md docs/archive/00-SYSTEM-CONTEXT-v1.md`.
2. Write a new `docs/00-SYSTEM-CONTEXT.md` = the v1 text with every 02-V2-CHANGES decision merged
   into the right section (roles, attendance, languages, Drive, licence, data model tables to be
   added, dependencies incl. `@fontsource/noto-sans-telugu`, module list, owner decisions).
   Where v1 text is superseded, remove it (don't leave two rules).
3. Keep `02-V2-CHANGES.md` unchanged as the decision record, with a note at the top: "Merged
   into 00-SYSTEM-CONTEXT.md in Phase 11."
4. Create `docs/OWNER-DECISIONS.md` from 02-V2-CHANGES §14: decision, default in use, where
   the default lives in code/config, status (open/decided), owner's answer.
5. Add to `docs/01-MOCK-SPEC.md` a short "v2 amendments" section: Attendance P/A (no L),
   Home cut-off copy removed, prototype is the reference for all new screens.

### Step 2 — App identifier check (report only)
Report the identifier in `tauri.conf.json`, Android Gradle and macOS bundle. Do NOT change it.
Record in OWNER-DECISIONS.md: "Must be decided before the first public release." If the owner
has already written an answer there, apply it only if no public release has happened (check git
tags and phase notes); otherwise STOP and ask.

### Step 3 — Branding
Publisher / developer / copyright strings → "Zuhair Hussain" (NSIS publisher, macOS copyright,
Android app info where applicable, Settings → About, INSTALL.md, ADMIN-GUIDE.md, receipt footer
line "Developed by Zuhair Hussain" in small muted text if the receipt template has a footer).
Support email and website from 02-V2-CHANGES §1 in About and docs. No phone number anywhere.

### Step 4 — Attendance: Present / Absent only
1. vidya-core `attendance.rs`: mark enum keeps `L` only for reading legacy rows. New marks must
   be `P` or `A` (`VALIDATION{field:"mark", rule:"p_or_a"}`). Percentage = P ÷ (P + A + L_legacy),
   i.e. legacy L counts as absent **[OWNER default]**. Tests for both.
2. Sync: an op carrying `L` whose HLC is **after** the v2 cutover (a constant written into the
   migration below) is rejected with reason code `LEAVE_MARK_REMOVED` ("Leave marks were removed;
   update Vidya on this phone"). Ops from before the cutover are applied as legacy.
3. Migration `NNNN_v2_attendance_pa.sql`: records the cutover time in `schema_meta`; no data
   changes to existing marks.
4. UI: phone Attendance and desktop register lose the L button, the Leave count and the Leave bar
   segment exactly as the prototype `attendance` screen shows; the help text becomes "34 students ·
   tap P or A". History views show legacy L as a muted "Leave (old)" pill.
5. Update `design/screens/Attendance.dc.html` the same way (owner-approved change) and regenerate
   the Phase 1 fidelity baseline for Attendance; note it in the handoff.

### Step 5 — No cut-off, no low-attendance threshold
Remove the attendance cut-off setting (UI + settings key; leave any DB column in place, unused)
and the low-attendance threshold setting. Home "Needs attention" shows "<class> attendance not
submitted" with sub-line "Class teacher <name> · not submitted yet today". Register and daily list
highlight absent students' names (existing absent colour) — no alerts, no thresholds. Reports that
highlighted "below 75%" now just show the percentage.

### Step 6 — Module switches (foundation item 8)
1. Migration: `module_setting(key TEXT PRIMARY KEY, enabled INTEGER, changed_by, changed_at)`,
   seeded with 02-V2-CHANGES §11 defaults (keys: `accounts`, `classroom`, `hr`, `circulars`,
   `wa_auto`, `store`, `instant_sync`; core has no key and is always on).
2. vidya-core `modules.rs`: `module_for(action)` mapping every action to a module (core by
   default) and `require_module(enabled_set, action)` → `MODULE_OFF{module}`.
3. Every command and every server endpoint checks it (one helper). Sync scope excludes the tables
   of disabled modules. Turning a module off never deletes data; turning it on again shows it.
4. `instant_sync` wraps the existing relay route: when off (default), the sync engine never tries
   the relay and `RELAY_URL` becomes optional in build config and release checks.
5. Settings → **Languages & modules** screen matching prototype `settings` state 3 (modules card;
   the languages card shows English and हिंदी now, Telugu is added in Phase 13). Toggling asks for
   confirmation and is audited.

### Step 7 — Principal sidebar
Reorganize exactly like the prototype `NAV_PRINCIPAL`: Overview (Home, Approvals) · Academics
(Students, Attendance, Marks & exams, Timetable, Calendar) · Finance (Fees, Accounts, School store)
· School (Staff & access, Circulars, Backups, Settings). Items whose module or screen doesn't exist
yet are **hidden** (not placeholders). "Sync & devices" moves into Settings (list item) and stays
reachable from the header sync pill. "Day book" becomes a tab inside Accounts in Phase 15; until then
keep it reachable under Fees.

### Step 8 — Tests
- vidya-core: P/A validation, legacy L percentage, cutover rejection, module mapping (every
  action has a module), `MODULE_OFF`.
- Integration: module off → command rejected, route hidden, table not in a teacher's pull.
- Playwright: Attendance P/A (updated baseline), Home copy, Settings → Languages & modules.
- Full P01–P10 suites.

## STOP CONDITIONS
Identifier decision needed and a public release already exists. A behaviour change would alter
financial or audit data. A module mapping is unclear for an existing action.

## HANDOFF → `docs/phase-notes/phase-11.md`
Start state · inventory summary · merged-context diff summary · attendance changes + cutover
constant · removed settings · module switch design (keys, mapping, scope exclusion) · sidebar
changes · test results (old + new, with every changed expectation and reason) · owner decisions
open · what Phase 12 needs.
