# PROGRESS.md — build progress

Agents update this file at the end of every session. The developer ticks the manual checks.

## Status
| Prompt | Title | Status | Date | Notes |
|---|---|---|---|---|
| P1.1 | Workspace, tooling and CI | done | 2026-09-15 | `npm run verify` + `VERIFY_DENY=1` pass. Pending developer manual checks (window, CI green, Windows installer). |
| P1.2 | Frontend port of the prototype | not started | | |
| P1.3 | Desktop shell and security settings | not started | | |
| P2.1 | vidya-core domain rules | not started | | |
| P2.2 | Permission matrix | not started | | |
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
| P4.2 | Device ID, activation and principal reset | not started | | |
| P4.3 | Provider Tool | not started | | |
| P4.4 | Setup wizard, login slips and recovery sheet | not started | | |
| P4.5 | Student import from Excel | not started | | |
| P5.1 | Printing and Hindi spike (decide how documents are made) | not started | | |
| P5.2 | Documents: receipts, report cards, registers, TC | not started | | |
| P6.1 | Local and pen drive backups, restore | not started | | |
| P6.2 | Google Drive backup | not started | | |
| P6.3 | New academic session | not started | | |
| P7.1 | LAN server and background running | not started | | |
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
| P10.1 | Size audit and Windows installer | not started | | |
| P10.2 | Mac signing and notarization | not started | | |
| P10.3 | Android signing and release pipeline | not started | | |
| P10.4 | Security and privacy review | not started | | |
| P10.5 | Load test and real-device testing | not started | | |
| P10.6 | Play Store and Google verification | not started | | |

Status values: not started, in progress, done, blocked.

## Check in Windows VM
(Agents add items with exact steps. The developer ticks them.)

- [ ] **P1.1** In GitHub → Actions → latest CI run, confirm the **Windows — verify and installer** job is green. Download the **vidya-windows-installer** artifact, unzip, run the `.exe` in the Windows 11 VM (SmartScreen: More info → Run anyway). Confirm it installs and opens a Vidya window, then uninstall.

## Check on MacBook
- [ ] **P1.1** Run `npm run tauri dev`; a Vidya window opens showing "Vidya" and a version.
- [ ] **P1.1** After pushing, confirm all three CI jobs (macos, windows, deny) are green in the GitHub Actions tab.

## Check on Android phone

## Session log
(Newest first. Paste each session summary here.)

### 2026-09-15 — P1.1 Workspace, tooling and CI
- Dev environment installed (Rust + both macOS targets, cargo-deny 0.20.2); doctor.sh core tools all present.
- Created root files, npm+Vite frontend, Tauri 2 app crate (`vidya-app`/`vidya_app_lib`, id `in.vidya.school`), 10-crate Rust workspace with dependency arrows matching ARCHITECTURE.md §2, eslint/prettier/rustfmt config, check-api-drift + check-i18n + verify.sh, deny.toml, and `.github/workflows/ci.yml` (macos/windows/deny jobs).
- `VERIFY_DENY=1 npm run verify` → ALL CHECKS PASSED.
- Decisions: dropped auto-added `log`/`tauri-plugin-log` from the app crate (not in DEPENDENCIES.md; plugins land in P1.3); scoped cargo-deny `unmaintained = "workspace"` for transitive Tauri deps; added kit source-of-truth markdown to `.prettierignore` (AGENTS.md rule #5). See docs/KNOWN_ISSUES.md.
- Pending: developer manual checks above (dev window, CI green, Windows installer in VM).
