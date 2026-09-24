# PHASE 18 — V2 HARDENING, UPGRADE SAFETY, FULL-SUITE FIDELITY, DOCS AND RELEASE v2.0.0

## ROLE
You are a senior release engineer and QA lead shipping **Vidya Budget School v2.0.0**.

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
14. Run every command you mention and paste real output. Branch `v2/p18`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-18.md`.


Extra rule for this phase: **no new features.** Only fixes, tests, performance, size, security,
docs and packaging.

## OBJECTIVE
Prove that v2 is safe to install fresh and to upgrade from v1 with real data, that every module works
together offline and online, that every screen matches the mock or prototype, and produce the v2.0.0
release files within the size limits at ₹0 monthly running cost.

## DONE MEANS
- Upgrade test: a v1.x database (demo seed + 1 term of activity + legacy L marks + v1 licence) →
  v2 app → migrations pass, money totals identical to the paisa, no licence prompt, all screens load.
- Full automated suite green: vidya-core, integration, role matrix (every new command and endpoint ×
  every role × module on/off), sync harness (LAN / Drive / queue / PC off / epoch fencing), soak test
  (3 clients × 10,000 ops including new modules), Playwright mock + prototype fidelity for every
  screen in 03-PROTOTYPE-SPEC §3.
- Performance budgets from the original Phase 9 still met with all modules on (record numbers,
  including Accounts and Profit with a full year of data).
- Each installer ≤ 40,000,000 bytes download and ≤ 50,000,000 bytes installed (Telugu font counted).
- Three languages complete (`check-i18n` 3-way), phrases for native review listed.
- Docs updated; draft release built by the owner's tag.

## WORK

### 1. Known issues
Fix every known issue in phase notes 11–17 or get the owner's written deferral.

### 2. Upgrade and migration safety
- Build `tests/upgrade/` with frozen v1 databases (from the last v1 release) and scripts that run the
  v2 app's migrations and compare: row counts, payment/receipt totals per day and mode, audit chain
  still verifying, vouchers balanced, guardian mapping counts, attendance history (legacy L shown as
  "Leave (old)").
- Interrupted migration (kill mid-way) → next start resumes or rolls back cleanly.
- Now that Phase 12's migration is proven, drop the deprecated columns (`staff.google_email`, old
  guardian columns on `student`) in a new migration, after checking no code reads them.
- Devices on v1 with unsent ops → after the school PC upgrades, they still sync (protocol version
  handling; clear "Update Vidya on this phone" message where needed).

### 3. Security and privacy review (new surfaces)
Email sender (tokens encrypted, no message bodies in logs), WhatsApp Cloud API token storage, share
plugin (FileProvider paths limited to the cache folder), attachment blobs (sealed, size limits,
hash checks), notes folder (not encrypted by design — confirm only study material paths are
uploaded), licence-maker (never in any app artifact: search binaries for its code/strings), epoch
file signature checks, module-off enforcement on the server, consent rule before any guardian
message, erase-on-request leaves no personal data in search indexes or caches.

### 4. Performance and size
Seed a large school (1,500 students, 60 staff, 1 year of attendance, fees with instalments, 2,000
expenses, 12 salary runs, 500 circulars/messages, 2 exams with seating) and measure: Principal Home,
Accounts → Profit, Dues list, Timetable, seating generation, report card batch print. Record size
per platform (download + installed) and the delta from v1.

### 5. Accessibility and languages
Contrast and focus for new screens; Android 130% font scale on new phone screens (absence, notes,
staff day); TalkBack labels for P/A buttons, check-in and share. Telugu and Hindi layout checks at
360 px width (no truncation of badges — min-width rule).

### 6. Demo seed and videos
Extend `seed_demo_school` so every prototype state can be reproduced with the same names and numbers
(Saraswati Public School, Meena Iyer, Kavya Singh, the cash book day, the salary month, the
September calendar, Circular 14). This lets the owner record demo videos from the real app.

### 7. Docs
- `INSTALL.md`: two Google accounts; joining with the sync password; checking the machine code;
  entering the licence key; unsigned-build steps; recommended server PC setup (on in school hours,
  auto-start, UPS).
- `ADMIN-GUIDE.md`: every new module in plain language; module switches; privacy duties (consent,
  export/erase, incidents, 72-hour note, "confirm with a lawyer"); automatic WhatsApp setup and
  cost; moving to a new PC with a transfer licence.
- `SELLING.md` (owner): final version.
- `CHANGELOG.md`: v2.0.0 summary (added, changed — attendance P/A, no cut-off, licence files,
  Drive sync account — removed: licence server).
- `OWNER-DECISIONS.md`: every decision's final state; anything still open is listed in the release
  notes as a known limitation.

### 8. Release
- Version 2.0.0 single-sourced; `release.yml` no longer needs `LICENCE_API`/`RELAY_URL`; needs
  `licence_public_key` (release, not dev) and Google client IDs.
- Clean-machine runs (record date, device, result): fresh Windows 10 install → licence key → setup
  → invite; cheap Android join via sync account → attendance offline → confirm via Drive with the
  PC on; PC off → provisional → PC on; upgrade Windows v1 → v2 with data; restore on a Mac with a
  transfer licence → old PC fenced; email + WhatsApp share from a phone; Telugu UI walkthrough.
- `site/releases.json` updated by the workflow; site build checked locally (not deployed).
- The owner tags `v2.0.0`; the workflow creates the DRAFT release; the owner reviews and publishes.

## STOP CONDITIONS
Any upgrade test that changes money or audit data. Any size limit exceeded. Any feature that would
need a company server.

## FINAL HANDOFF → `docs/phase-notes/phase-18.md`
Release checklist · upgrade test results · measured performance + sizes · security findings + fixes
· language status · owner decisions final state · exact commands to rebuild each binary.
