# PROGRESS.md — build progress

Agents update this file at the end of every session. The developer ticks the manual checks.

## Status
| Prompt | Title | Status | Date | Notes |
|---|---|---|---|---|
| P0.1 | Apply Edition 2 to the specification | done | 2026-09-17 | 45 prompts installed; React, size, licensed-server and local-backup specifications reconciled. Developer approval of D27–D31 remains. |
| P1.1 | Workspace, tooling, size gate and CI | not started | | Edition 1 implementation exists; Edition 2 requirements must be applied and reverified. |
| P1.2 | React port of the prototype | not started | | Edition 1 plain-JavaScript port exists and must be migrated to React. |
| P1.3 | Desktop shell and security settings | not started | | Edition 1 shell exists; Edition 2 separate-build size gates must be applied. |
| P2.1 | vidya-core domain rules | not started | | Edition 1 implementation exists and must be checked against Edition 2. |
| P2.2 | Permission matrix | not started | | Edition 1 implementation exists; Edition 2 adds backup.run and office-computer context. |
| P2.3 | Encrypted database and migrations | not started | | |
| P2.4 | Platform secrets, folders and app start | not started | | |
| P2.5 | Services foundation, repositories and change log | not started | | |
| P2.6 | Sign-in and sessions | not started | | |
| P2.7 | Command layer and first real screens | not started | | |
| P3.1 | Staff logins and the permission test harness | not started | | |
| P3.2 | Students and admissions | not started | | |
| P3.3 | Fees, receipts and day book | not started | | |
| P3.4 | Attendance | not started | | |
| P3.5 | Marks and report cards | not started | | |
| P3.6 | Settings | not started | | |
| P3.7 | Home screens, reports, activity and Excel export | not started | | |
| P4.1 | License codes crate | not started | | |
| P4.2 | Device ID, activation, server permit and principal reset | not started | | |
| P4.3 | Provider Tool | not started | | |
| P4.4 | Setup wizard, login slips and recovery sheet | not started | | |
| P4.5 | Student import from Excel | not started | | |
| P5.1 | Printing and Hindi spike (decide how documents are made) | not started | | |
| P5.2 | Documents: receipts, report cards, registers, TC | not started | | |
| P6.1 | Encrypted backups, automatic backups and restore | not started | | |
| P6.2 | Backup drives: sync backups to local and removable drives | not started | | |
| P6.3 | New academic session | not started | | |
| P7.1 | Licensed LAN server and background running | not started | | |
| P7.2 | Discovery, firewall and connection check | not started | | |
| P7.3 | Phone approval, signed requests and device management | not started | | |
| P7.4 | Sync engine core | not started | | |
| P7.5 | Sync endpoints and live updates | not started | | |
| P8.1 | Android project and client-only build | not started | | |
| P8.2 | Android secure storage and local database | not started | | |
| P8.3 | Phone discovery, sign-in and approval | not started | | |
| P8.4 | Phone sync, offline work and removal | not started | | |
| P8.5 | Phone screens, PDFs and privacy protections | not started | | |
| P9.1 | Full Hindi | not started | | |
| P10.1 | Size gate and Windows installer | not started | | |
| P10.2 | Mac signing and notarization | not started | | |
| P10.3 | Android signing and release pipeline | not started | | |
| P10.4 | Security and privacy review | not started | | |
| P10.5 | Load test and real-device testing | not started | | |
| P10.6 | Play Store listing and review access | not started | | |

Status values: not started, in progress, done, blocked.

## Check in Windows VM
(Agents add items with exact steps. The developer ticks them.)

- [ ] **P1.1** In GitHub → Actions → latest CI run, confirm the **Windows — verify and installer** job is green. Download the **vidya-windows-installer** artifact, unzip, run the `.exe` in the Windows 11 VM (SmartScreen: More info → Run anyway). Confirm it installs and opens a Vidya window, then uninstall.
- [ ] **P1.3** After P1.3 is pushed, record the NSIS installer size from the **Windows — verify and installer** CI log. Install it per-machine in the Windows 11 VM, launch Vidya twice and confirm only one window/process remains, then uninstall it from Settings → Apps.

## Check on MacBook
- [ ] **P0.1** Read D27–D31 in `docs/DECISIONS.md` and approve them. Confirm teachers should remain unable to run backups at the office computer (the default is no).
- [ ] **P1.1** Run `npm run tauri dev`; a Vidya window opens showing "Vidya" and a version.
- [ ] **P1.1** After pushing, confirm all three CI jobs (macos, windows, deny) are green in the GitHub Actions tab.
- [ ] **P1.2** Run `npm run tauri dev`, load the sample school, then sign in as `sunita`, `anita`, and `sierra` with password `vidya123`. Exercise every screen and tick [PARITY.md](PARITY.md) while comparing it with `reference/VidyaSchoolApp_step1.html` in Chrome.
- [ ] **P1.3** Build the Edition 2 Apple Silicon and Intel DMGs, mount the build for this Mac, right-click `Vidya.app` → Open, and confirm every screen has no CSP console violations.
- [ ] **P1.3** In the running Mac app, confirm Cmd+C/Cmd+V in the username field, Cmd+Q, all application/Edit/Window menu items, one window after launching twice, a log file under the app log directory, and no Inspect action or devtools shortcut in the release build.
- [ ] **P2.1** Read `crates/vidya-core/tests/vectors.rs`; spot-check three fee calculations and three amount-in-words results by hand.

## Check on Android phone

## Session log
(Newest first. Paste each session summary here.)

### 2026-09-17 — P0.1 Apply Edition 2 to the specification
- Installed the Edition 2 book at `docs/PROMPTS_BOOK.md` and split all 45 prompt files, including the new P0.1 and replacement P6.2 backup-drives prompt.
- Added D27–D31 and reconciled AGENTS, UI, architecture, permissions, API, backup format, platforms, dependencies, data defaults, product wording and testing guidance.
- Removed active cloud-backup, external-authorization, legacy renderer and universal-build assumptions; retained only the historical D15/D30 supersession note and explicit forbidden-rendering rule text.
- Added the empty `docs/SIZE.md` ledger and restored `.claude/commands/run.md` with the Edition 2 prompt path.
- Edition 1 code verification is intentionally deferred to P1.1/P1.2 because P0.1 permits document changes only and the new permission/API documents now intentionally lead the old implementation.

### 2026-09-17 — P2.2 Permission matrix
- Added all 39 documented actions in order, exact string parsing, an explicit no-wildcard role/action access matrix, section scope helpers and UI permission-name generation.
- Added a machine-readable parity test against `docs/PERMISSIONS.md` plus principal, accountant and assigned-section teacher behavior tests.
- Deliberately changed one document cell: the test failed with the exact action, role, document value and code value; restored the cell and reran green. `cargo test -p vidya-core` → 39 passed; `npm run verify` → ALL CHECKS PASSED.

### 2026-09-17 — P2.1 vidya-core domain rules
- Implemented pure domain modules for localisable errors, roles/actors, Indian money, amount-in-words, dates/sessions, validation, fees/payments, marks/grades, attendance, usernames, supplied-byte secret formatting, and hybrid logical clocks.
- Added matching embedded core locales, exact table-driven vectors, three-or-more valid/invalid cases for every validator, and proptest coverage for money round trips, non-negative fees/balances and HLC text ordering.
- `cargo test -p vidya-core` → 31 passed; forbidden `now()`/`SystemTime`/`f32`/`f64` scan → no matches; `npm run verify` → ALL CHECKS PASSED.

### 2026-09-16 — P1.3 Desktop shell and security settings
- Added the strict CSP and explicit main-window capability, command allowlist manifest, desktop plugins, macOS native menus, release context-menu protection, platform trait/fake skeleton, generated icon set, and macOS/Windows bundle configuration.
- Built a 6.8 MB universal `Vidya.app`; `lipo` confirms `arm64` and `x86_64`, and its Info.plist confirms macOS 11.0, Education category and copyright metadata.
- Launched the release app twice: the second process exited successfully while the first remained, and `~/Library/Logs/in.vidya.school/Vidya.log` was created.
- `VERIFY_DENY=1 npm run verify` → ALL CHECKS PASSED (71 frontend tests; 462 matching translation keys; Rust format, clippy, tests and dependency audit pass).
- The normal Finder-decorated DMG path raced the macOS 26 volume unmount. Tauri's supported `CI=true` path skipped only Finder cosmetic positioning and produced a checksum-valid 3,182,807-byte universal DMG with the standard Applications link.
- CI run 35087674060 passed macOS verification, Windows verification/NSIS packaging, and cargo-deny. The downloaded `Vidya_0.1.0_x64-setup.exe` is 1,222,379 bytes (artifact ZIP reported by CI: 1,203,570 bytes).

### 2026-09-16 — P1.2 Frontend port of the prototype
- Completed the setup wizard, credential slips, pre-school restore, attendance, marks, reports, staff logins, activity, backup and settings screens; registered all desktop views and the client-safe mobile subset.
- Expanded locales to 462 matching keys, added mocked-command screen tests plus real mock-DTO integration coverage (71 tests total), and created `docs/PARITY.md` with 94 comparison items.
- Edition 1 safe-template checks passed; Edition 2 supersedes that renderer with React components and a complete raw-DOM-write ban.
- `VERIFY_DENY=1 npm run verify` → ALL CHECKS PASSED. Developer side-by-side UI parity check remains pending.

### 2026-09-15 — P1.1 Workspace, tooling and CI
- Dev environment installed (Rust + both macOS targets, cargo-deny 0.20.2); doctor.sh core tools all present.
- Created root files, npm+Vite frontend, Tauri 2 app crate (`vidya-app`/`vidya_app_lib`, id `in.vidya.school`), 10-crate Rust workspace with dependency arrows matching ARCHITECTURE.md §2, eslint/prettier/rustfmt config, check-api-drift + check-i18n + verify.sh, deny.toml, and `.github/workflows/ci.yml` (macos/windows/deny jobs).
- `VERIFY_DENY=1 npm run verify` → ALL CHECKS PASSED.
- Decisions: dropped auto-added `log`/`tauri-plugin-log` from the app crate (not in DEPENDENCIES.md; plugins land in P1.3); scoped cargo-deny `unmaintained = "workspace"` for transitive Tauri deps; added kit source-of-truth markdown to `.prettierignore` (AGENTS.md rule #5). See docs/KNOWN_ISSUES.md.
- Pending: developer manual checks above (dev window, CI green, Windows installer in VM).
