# PROGRESS.md — build progress

Agents update this file at the end of every session. The developer ticks the manual checks.

## Status
| Prompt | Title | Status | Date | Notes |
|---|---|---|---|---|
| P0.1 | Apply Edition 2 to the specification | done | 2026-09-17 | 45 prompts installed; React, size, licensed-server and local-backup specifications reconciled. Developer approval of D27–D31 remains. |
| P1.1 | Workspace, tooling, size gate and CI | done | 2026-09-17 | React foundation, desktop/mobile split, lint guard, size gate and CI packaging complete. arm64 DMG 1.42 MB; installed app 2.57 MB. Windows figures pending CI/VM. |
| P1.2 | React port of the prototype | done | 2026-09-19 | React screens and interactions ported; screenshot-reported dialog/form/dashboard CSS mismatch fixed with tests. Developer visual review remains. |
| P1.3 | Desktop shell and security settings | in progress | 2026-09-19 | Local implementation and all three Mac size measurements complete; CSP screen-by-screen and Windows CI/VM checks remain. |
| P2.1 | vidya-core domain rules | done | 2026-09-19 | Edition 2 vectors and pure-rule audit pass; default grade-band test aligned to DATA_MODEL. Developer spot-check remains. |
| P2.2 | Permission matrix | in progress | 2026-09-19 | Existing matrix and mutation tests pass, but PERMISSIONS.md rule 8 conflicts with the prompt's principal-phone behavior; developer decision required before completion. |
| P2.3 | Encrypted database and migrations | in progress | 2026-09-19 | Local implementation, Mac tests, and mounted-DMG size pass; current Windows CI/provider/installer size remain. |
| P2.4 | Platform secrets, folders and app start | in progress | 2026-09-20 | macOS Keychain (key + secrets), 0700 data/backups dirs, start sequence, `app_status` command, StartupError screen — implemented and verified on Mac (`cargo test -p vidya-app` 3+1 tests, StartupError RTL 2 tests, `npm run verify` green). Windows DPAPI/ProgramData code written against verified windows 0.61.3 APIs but **not yet compiled** (needs CI push + VM). |
| P2.5 | Services foundation, repositories and change log | done | 2026-09-21 | `Services` container (injected clock/ids/random, HLC), `ServiceError` (+DomainError/DbError, to_dto, internal ref), change-log writer with secret redaction, 17 vidya-db repo modules, Argon2id password hashing, `SchoolService::header`/`SettingsService::get`, and the deterministic Vaani sample school (136 students, users, receipts, attendance, marks). `cargo test -p vidya-db -p vidya-services -p vidya-testkit` + `npm run verify` green (insta SettingsDto snapshot committed). |
| P2.6 | Sign-in and sessions | done | 2026-09-21 | `AuthService`: sign-in with @school check, lockout (staff lock at 5, principal 5-min pause), pending first-password tokens, `SessionStore` (SHA-256-hashed tokens, idle timeout), change password, set language, sign out, and `actor_for_token` (reloads user fresh, ends sessions on deactivate/lock). Added `sha2` 0.10, users-repo update methods. Made the change-log writer's DB part (`write_entry`) return `DbError` so it composes in `Db::write`. 11 auth tests + `npm run verify` green. Manual security review done (security-reviewer subagent not registered in this env). |
| P2.7 | Command layer and first real screens | done | 2026-09-21 | `AppState` now owns the built `Services` + `SessionStore` (device id created on first start); `AppError::from_service` translates into the actor's language; `with_actor`/`without_actor` command harness (desktop always `OfficeComputer` origin). 9 real commands (app_status extended, sign_in, set_first_password, sign_out, current_user, change_password, set_language, get_settings, debug-only load_sample_school). 60-second idle watcher emits `session-expired`. Frontend: those commands call real `invoke`; Welcome sample button debug-only; SessionProvider handles `session-expired` + language; a best-effort mock bridge keeps feature screens working (KNOWN_ISSUES #7). `npm run verify` green (50 JS tests, API drift 9 Rust commands). |
| P3.1 | Staff logins and the permission test harness | done | 2026-09-21 | `UserService` (list/create/update/reset_password/unlock/set_active) with username generation, temp passwords, licence `max_users` limit, teacher-section rules, and session ending on reset/switch-off. Reusable permission-matrix harness proves every service method against 6 role/origin actors; a coverage test lists the remaining 36 actions in `NOT_YET_IMPLEMENTED` with prompt ids. 6 commands + `views/Users.jsx` (real commands, section picker); mock `users.js` removed. 7 UserService tests + `npm run verify` green. |
| P3.2 | Students and admissions | done | 2026-09-21 | Role-shaped DTOs (`StudentTeacherDto` vs `StudentOfficeDto`, tagged `shape` enums for list/detail) make it structurally impossible for a teacher response to carry fee, concession or category data — proved by a serialization leak test. `StudentService` (list/get/add/update/mark_left): sequential admission numbers from a counter (survive reopen), per-section roll numbers never reused, section-scoped teacher visibility, concession gated on `students.set_concession`, offline guard on client mode, and duplicate-name confirmation. 5 commands + real `StudentList`/`StudentDetail`/`StudentForm` (section selector from settings, duplicate-confirm, mark-left). Student mock removed (only Excel export stays mock). 6 service tests + 5 permission-matrix cases; `npm run verify` green. |
| P3.3 | Fees, receipts and day book | done | 2026-09-21 | `FeeService` (register/account/collect/get_receipt/cancel/day_book/list_alerts/resolve_alert) over append-only receipts. `account.rs` computes due/paid/balance/state/one-term and previous-session dues from `vidya-core::fees`. Collection validates through `validate_payment` (RTE refused, over-balance, zero-balance, UPI 12-digit + reuse-while-active), assigns a per-device receipt series from `counters` (`PC-0001…`, `T1-0001` after a device-code change, continues across reopen), and appends a change-log entry. Principal-only cancellation restores the balance (paid excludes cancelled) and the day book shows cancelled receipts struck out and excluded from mode totals. Overpayment alerts are role-filtered (accountant sees overpayment only) and resolvable. 8 commands + real fee views + receipt/day-book print; fee mock removed (Excel export stays mock). 13 service tests + trigger + matrix cases; `npm run verify` green. |
| P3.4 | Attendance | done | 2026-09-21 | `AttendanceService` (sheet/save/register/today_summary). Teachers mark their own sections for today; the principal corrects past days (`check_attendance_date`-driven `read_only_reason`, translated to the actor's language). `save` refuses future dates, requires every active student marked (`attendance.error.not_marked` names the first three), rejects marks for students not in the section, and replaces the whole day's marks in one transaction with a `replace_set` change-log entry keyed `saved`/`corrected`. Monthly register builds per-day cells and present counts. 3 commands + reworked `Attendance.jsx` (section selector from settings, `useReducer` draft, card cycling P→A→L, mark-all). Attendance mock removed. 6 service tests + a cycling RTL test + matrix cases; `npm run verify` green. |
| P3.5 | Marks and report cards | done | 2026-09-21 | `MarksService` (sheet/save/report_card/class_report_cards). Every total, percent (one-decimal text) and grade is computed in `vidya-core` — no maths in JS. `save` parses each cell with `parse_mark`, collects **all** bad cells before writing and returns one `marks.error.cells` error carrying a human list plus a machine `cells` param the UI uses to highlight; blank deletes the row, value/AB upserts; one `replace_set` change-log entry with a per-cell sync payload. Report card renders every active exam as columns with totals/percent/grade and session attendance. 4 commands + reworked `Marks.jsx` (memoised rows, cell-error highlighting) and `ReportCard.jsx`; marks mock removed. 5 service tests (77/125→"61.6"/C, AB-in-max, blank-deletes, above-max-saves-nothing, teacher-scope) + a cell-highlight RTL test + matrix cases; `npm run verify` green. |
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
- [ ] **P1.1 Edition 2** Confirm the macOS and Windows CI logs show the size table below 30 MB. Record the Windows installer and real installed-folder sizes in `docs/SIZE.md` after installing in the VM.
- [ ] **P1.3** After the current P1.3 code is pushed, record the NSIS installer size from the **Windows — verify and installer** CI log. Install it per-machine in the Windows 11 VM, measure `C:\Program Files\Vidya` and enter the real installed-folder size in `docs/SIZE.md`; launch Vidya twice and confirm only one window/process remains, then uninstall it from Settings → Apps.
- [ ] **P2.3** After pushing the current database changes, confirm the Windows CI `Show SQLCipher version and provider` test prints non-empty values, the Windows test and NSIS jobs are green, and the installer is below 30 MB. Install in the Windows 11 VM, open a sample encrypted database, and record the installed folder size in `docs/SIZE.md`.
- [ ] **P2.4** Push, then confirm the Windows CI job compiles `platform/windows.rs` and the `windows_dpapi` integration test passes (protect/unprotect round trip, tampered blob fails, wrong entropy fails). In the Windows 11 VM: start Vidya, confirm `C:\ProgramData\Vidya\data\vidya.db` and `key.bin` exist; restart and confirm it reopens; sign in from a second Windows user account and confirm it opens the same data.

## Check on MacBook
- [ ] **P0.1** Read D27–D31 in `docs/DECISIONS.md` and approve them. Confirm teachers should remain unable to run backups at the office computer (the default is no).
- [ ] **P1.1** Run `npm run tauri dev`; a Vidya window opens showing "Vidya" and a version.
- [ ] **P1.1** After pushing, confirm all three CI jobs (macos, windows, deny) are green in the GitHub Actions tab.
- [ ] **P1.2** Run `npm run tauri dev`, load the sample school, then sign in as `sunita`, `anita`, and `sierra` with password `vidya123`. Exercise every screen and tick [PARITY.md](PARITY.md) while comparing it with `reference/VidyaSchoolApp_step1.html` in Chrome.
- [ ] **P1.3** Mount the built Apple Silicon DMG, right-click `Vidya.app` → Open, and confirm every screen has no CSP console violations. The Apple Silicon, Intel and universal builds and size measurements are already complete.
- [ ] **P1.3** In the running Mac app, confirm Cmd+C/Cmd+V in the username field, Cmd+Q, all application/Edit/Window menu items, one window after launching twice, a log file under the app log directory, and no Inspect action or devtools shortcut in the release build.
- [ ] **P2.1** Read `crates/vidya-core/tests/vectors.rs`; spot-check three fee calculations and three amount-in-words results by hand.
- [x] **P2.3** Mounted the new ARM DMG read-only; `du -sk Vidya.app` reported 3492 KiB (3.58 decimal MB), recorded in `docs/SIZE.md`.
- [ ] **P2.4** `npm run tauri dev`: confirm `~/Library/Application Support/in.vidya.school/data/vidya.db` appears and `sqlite3 <that file> .tables` reports "file is not a database" (it is encrypted).
- [ ] **P2.4** `VIDYA_KEYCHAIN_TEST=1 cargo test -p vidya-app --test macos_keychain` passes (allow Keychain access if prompted).
- [ ] **P2.4** Keychain Access shows the item `in.vidya.school.dbkey`.
- [ ] **P2.7** `npm run tauri dev` → "Load a sample school" → sign in as `sunita` / `vidya123` works against the real encrypted database.
- [ ] **P2.7** Wrong password 5 times for `sierra` locks the login; the message matches the prototype.
- [ ] **P2.7** Quit and reopen the app: the sample school is still there (real database persists).
- [ ] **P2.7** Change language to हिं, sign out and back in: the language persists.

## Check on Android phone

## Session log
(Newest first. Paste each session summary here.)

### 2026-09-21 — P3.1 staff logins and the permission test harness
- `services/users.rs` `UserService` (borrows `&Services` + `&SessionStore`): `list` (section labels, status, never the hash), `create` (role teacher/accountant only, `validate_person_name`/`validate_mobile`, teacher ≥1 active section, licence `max_users` limit, username via override or `username_base_from_name`/`fallback_username_base`+`unique_username`, `format_temp_password`, must_change), `update`, `reset_password` (+ends sessions), `unlock`, `set_active` (principal can't be switched off; ending sessions on off). DTOs `UserDto`/`CredentialSlipDto`/`CreateUserInput`/`UpdateUserInput`. Added `ServiceError::validation`, repo `update_profile`/`reset_for_temp_password`/`unlock`/`count_active`/`section_label`/`section_is_active`, and `users.error.*` core locale keys.
- `tests/permissions_matrix.rs`: reusable harness — sample school + 6 actors (principal/accountant/teacher_own(V-A)/teacher_other(VI-A)/principal_phone/accountant_phone); `check()` compares each call to `access()` + office-only; a coverage test asserts every `Action` has a case except the 36 in `NOT_YET_IMPLEMENTED` (each tagged with its prompt). `tests/users.rs`: 7 behaviour tests (sierra2 dedup, principal refused, 41st→limit, reset invalidates old password, switch-off ends session, temp password never in change log, principal_phone→office-only).
- `commands/users.rs` (6 commands via new `with_actor_sessions` helper) + registered. `views/Users.jsx` rewritten for the real `UserDto` (status/sections/lastLoginAt) with a section-checkbox picker from `getSettings`; mock `users.js` deleted. `npm run verify` → ALL CHECKS PASSED.

### 2026-09-21 — P2.7 command layer and first real screens
- Verified Tauri 2.11 command-arg convention (default `ArgumentCase::Camel` → Rust snake_case args map to JS camelCase). `state.rs`: `AppState { slot(condvar), sessions }` owns the built `Services` (SystemClock/UuidV7/OsRandom, `Mode::Server`, device id read/created in `meta.device_id`) and the `SessionStore`; `StartResult::{Ready{core,services},Failed}`. `AppCore.db` is now `Arc<Db>`.
- `commands/error.rs`: `AppError::from_service(err, lang)` via `ServiceError::to_dto`. `commands/mod.rs`: `with_actor` (resolves the actor on a blocking thread, maps errors in the actor's language) and `without_actor`. `commands/app.rs`: `app_status` extended (`licensed`/`schoolName`/`schoolCode`/`serverAllowed=false`) + debug-only `load_sample_school`. `commands/auth.rs`: `sign_in` (`SignInDto` tagged `status`), `set_first_password`, `sign_out` (sync), `current_user` (null when not signed in), `change_password`, `set_language`, `get_settings`. `background.rs`: 60-s idle watcher → `session-expired` event. `lib.rs`: builds services in setup, spawns the watcher, registers all commands (debug/release split so `load_sample_school` is release-excluded).
- Trimmed `vidya-testkit` deps to core/db/services (server/etc. re-added in P7.4) and added it to `src-tauri` for the debug sample command (stripped from release). Added `vidya-core`/`uuid` to `src-tauri`.
- Frontend: the 9 commands call real `invoke` with the in-memory token; `getToken` bridge; Welcome sample button under `import.meta.env.DEV`; SessionProvider listens for `session-expired` (resilient dynamic import) and follows the user's language. Best-effort mock mirror of sign-in/sample-load keeps feature screens working during the transition (KNOWN_ISSUES #7). Added a `session-expired` RTL test. `AppError::from_service`, `with_actor`, and the extended DTO documented in API.md. `npm run verify` → ALL CHECKS PASSED.

### 2026-09-21 — P2.6 sign-in and sessions
- `auth/sessions.rs`: `SessionStore` keyed by `SHA-256(token)` (raw tokens never stored); `create_full`/`create_pending` (10-min pending TTL), `resolve` (refreshes activity, drops expired), `end`/`end_all_for_user`/`end_others_for_user`, `expire_idle`. Timeout from `app_settings.session_timeout_minutes` (default 30).
- `auth/service.rs`: `AuthService::sign_in` (trim/lowercase, `@school` check vs license code, unknown/inactive → `dummy_verify` + `no_login`, locked/paused, verify, lockout at 5 → staff `locked` / principal `locked_until +5min`, `must_change` → pending token, else full session + `record_login`); `set_first_password`, `change_password` (ends other sessions), `set_language`, `sign_out`, `actor_for_token` (fresh user load, `signed_out` on inactive/locked, `session_expired` on idle). `CurrentUserDto` carries `permission_names(role, origin)`; origin trusted from caller (phone can't upgrade).
- Added `sha2` 0.10; `repo::users` update methods (`set_login_state`, `record_login`, `set_password`, `set_language`, `set_active`). Split the change-log writer into `write_entry` (DbError, composes in `Db::write`), `ensure_payload_safe`, and `record` (ServiceError). Added `auth.error.*` core locale keys (en/hi).
- Tests (`tests/auth.rs`, advanceable `TestClock` + sample school via a dev-dependency cycle): all 11 prompt cases pass (office permissions, @school, unknown/inactive, 4-then-ok reset, 5→lock, 5→pause→clear, must-change flow, idle expiry, deactivate ends sessions, no secrets in change log, phone origin blocks office-only). `npm run verify` → ALL CHECKS PASSED. Manual security review (subagent not registered).

### 2026-09-21 — P2.5 services foundation, repositories and change log
- Added deps (verified against installed source): `uuid` 1.26.1 (v7), `rand` 0.9.5 (pinned below 0.10 for argon2 compat), `argon2` 0.6.0 + `password-hash` 0.6.1 (`hash_password_with_salt`, salt from our own `Random`), `tracing` 0.1.44, dev `insta` 1.48.0. `chrono` gets the `clock` feature only in services; core still forbids `now()`.
- `env.rs`: `Clock`/`IdGen`/`Random` traits with `SystemClock`/`UuidV7`/`OsRandom` and deterministic `FixedClock`/`SeqIds`/`SeededRandom`. `error.rs`: `ServiceError` (+`From<DomainError>`/`From<DbError>`, `to_dto` via core i18n, `ERR-XXXX` refs; DbError SQL text never leaks). `lib.rs`: `Services` container loading `meta.hlc_last`, `next_hlc`, `school()`/`settings()`.
- `change_log.rs`: `record()` assigns `<device_id>:<change_counter>` and a fresh HLC, stores both in the same tx, and refuses any payload/params containing `password`/`passwordHash`/`password_hash`/`key`/`secret` at any depth (tested). 17 `vidya-db/src/repo/` modules (meta+counters via `RETURNING`, school, sessions, classes, fee_plans, subjects, exams, grade_scale, app_settings, users+user_sections, students, enrollments, receipts, cancellations, attendance, marks, license). `auth/password.rs`: Argon2id (19456/2/1) hash/verify/dummy_verify.
- `SchoolService::header` and `SettingsService::get` (authorize→read→assemble DTOs). `vidya-testkit` sample school: Vaani, Nursery–VIII (17 sections), sunita/anita/sierra/rakesh (`vidya123`), 136 students + enrollments, receipts, 6 days attendance, Unit Test 1 marks, demo license — deterministic from seed.
- Tests: repo insert/read round-trip (all 17 groups), change-log clean+secret-refusal, sample determinism (same seed→same names/counts, different seed→differs), `insta` SettingsDto snapshot (committed). `cargo test -p vidya-db -p vidya-services -p vidya-testkit` all green; `npm run verify` → ALL CHECKS PASSED. Deps left at member level (workspace-dep refactor deferred; single versions, no cargo-deny duplicates).

### 2026-09-20 — P2.4 platform secrets, folders and app start
- Verified every native API against installed sources before writing code: `security-framework` 3.7.0 (`passwords::{set,get,delete}_generic_password`, `random::SecRandom::copy_bytes`, `Error::code()`, `errSecItemNotFound = -25300` in `security-framework-sys` 2.17.0); `windows` 0.61.3 (`CryptProtectData`/`CryptUnprotectData`, `CRYPT_INTEGER_BLOB`, `CRYPTPROTECT_LOCAL_MACHINE`, `BCryptGenRandom`+`BCRYPT_USE_SYSTEM_PREFERRED_RNG`, `SHGetKnownFolderPath`+`FOLDERID_ProgramData`+`KF_FLAG_DEFAULT`, `LocalFree`/`CoTaskMemFree`); Tauri 2.11.5 `app.path().app_data_dir()`.
- macOS (`platform/macos.rs`): `data_dir`/`backups_dir` under `~/Library/Application Support/in.vidya.school` at `0700`; `load_or_create_db_key` and `store/load/delete_secret` via login Keychain generic passwords, keys wrapped in `Zeroizing`. Windows (`platform/windows.rs`, CI-only): `ProgramData\Vidya` via known-folder, DPAPI local-machine + entropy, atomic `key.bin` write, `secrets/<name>.bin`.
- Start sequence (`state.rs`): `start()` (data dir → key → `Db::open`) never panics, maps to `StartupFailure {DataFolder, SecureStorage, WrongKey, Migration, Unknown}`; `AppState` publishes the result behind a condvar. Setup hook runs it on a blocking thread. `app_status` command (`commands/app.rs`) waits and returns `AppStatusDto { ready, failure?, hasSchool, platform, version }`. `StartupError.jsx` + "Opening Vidya…" state wired into `DesktopApp`; `appStatus()` now calls `invoke`.
- Tests: `state` unit tests (create+reopen; wrong key → WrongKey), `macos_keychain` integration (round trip, gated on `VIDYA_KEYCHAIN_TEST=1`), `windows_dpapi` integration (CI-only), `StartupError` RTL (message per kind + Unknown fallback). `cargo test -p vidya-app` and `npm run verify` green (49 JS tests). ARM release DMG 2.07 MB (+0.04 MB vs P2.3).
- **Not done:** `platform/windows.rs` not yet compiled anywhere (no push made from the dirty worktree); Windows CI + VM checks and the macOS Keychain manual checks remain for the developer. Keychain accessibility uses the default (KNOWN_ISSUES #6).

### 2026-09-19 — P2.3 encrypted database local implementation
- Verified installed `libsqlite3-sys` 0.38.2's Apple CommonCrypto/vendored OpenSSL branches and `r2d2_sqlite` 0.35.0's `rusqlite` 0.40 dependency against Cargo.lock; selected target-specific SQLCipher features. Apple Silicon test reports SQLCipher 4.14.0, provider `commoncrypto`.
- Brought `0001_init.sql` into byte-for-byte parity with `DATA_MODEL.md` and added a document-parity test. Added keyed four-connection pooling, plaintext/wrong-key refusal, memory-security/foreign-key/busy-timeout connection setup, WAL, transactional migrations and serialized immediate writes with rollback.
- Added idempotent school-independent seeds and tests for encryption, reopen, schema version/table count, WAL, constraints, append-only trigger messages, rollback on errors and panic, and seed idempotence. `cargo test -p vidya-db` passes 11 tests.
- Built the fully linked ARM release DMG: 2,031,836 bytes, +571,214 bytes from the pre-database build, under the four-MB stop threshold. Mounted it read-only and measured `Vidya.app` at 3492 KiB on disk. Windows CI remains; no push was made from the dirty worktree.

### 2026-09-19 — P1.2 UI presentation repair
- Fixed the screenshot-reported account dialog overflow by styling the React portal's actual `modal-back`/header/body/footer classes, and made shared `Field` controls and `Button` variants match their React class names.
- Put all three dashboard variants in the shell's scroll surface; separated each class row's student count and teacher text; polished sidebar, cards, entry screens and attendance controls while keeping the documented palette and no new runtime dependencies.
- Added restrained dialog/hover transitions and a `prefers-reduced-motion` path. Print CSS now hides portal overlays. Added two DOM contract regressions (47 frontend tests total).
- `npm run build:mobile`, bundle split check, desktop build, and `VERIFY_DENY=1 VERIFY_SIZE=1 npm run verify` passed. The development app/server were relaunched; visual inspection of the refreshed window remains for the developer because macOS denied assistive access to the window inspection command.

### 2026-09-19 — P2.2 permission audit paused for specification conflict
- Confirmed the 40-action table, exact role matrix, `backup.run` office-only behavior, scope helpers and document parity test pass.
- Temporarily changed the accountant `backup.run` cell: parity test failed with `action=backup.run, role=Accountant, document=No, code=Yes`; restored it. Temporarily removed `backup.run` from the office-only list: parity test failed showing the missing action; restored it. `cargo test -p vidya-core --test permissions_doc` and `npm run verify` then passed.
- `PERMISSIONS.md` rule 8 forbids principal phone access to `devices.*`, `reports.*` and license actions, but P2.2's behavior test requires every action outside the eight-item office-only list to be allowed on a principal phone. The implementation currently follows the latter. No permission change is made until the developer resolves the conflict.

### 2026-09-19 — P2.1 Edition 2 core audit
- Reviewed all 12 Edition 2 tasks against `vidya-core`: pure domain modules, error/i18n fallback, origin-aware actor, integer money/fees/marks, dates, validation, attendance, usernames, supplied-byte secrets and HLC are present.
- Corrected the test's default grade scale to the authoritative A80/B65/C50/D33/E0 values and added B/C and C/D boundary vectors.
- `cargo test -p vidya-core` → 41 tests passed; forbidden clock/floating-point scan returned no matches; `npm run size` passed. Manual fee/word vector spot-check remains for the developer.

### 2026-09-19 — P1.3 Edition 2 shell audit (local work complete; manual checks pending)
- Removed the legacy inline-style CSP exception from release, added `font-src` and `form-action`, and isolated Vite dev allowances in `devCsp`. Production assets contain no inline style attributes or script bodies; screen-by-screen CSP console review remains for the developer.
- Added log rotation capped at five files of 2,000,000 bytes each, the release context-menu guard, explicit NSIS LZMA compression, and the Edition 2 `VolumeInfo`, `MountedVolume`, and `volume_info` trait skeleton.
- Built current Apple Silicon, Intel and universal DMG/app pairs. `npm run size` reports 1.46/2.62 MB, 1.52/3.25 MB and 2.94/5.85 MB respectively; all pass. The universal comparison has over 24 MB spare but D28 keeps separate shipping builds.
- Launched the ARM release app twice: the second launch exited with code 0 while one process remained; the app log file exists. Windows installed size and manual menu/CSP checks are not claimed complete.

### 2026-09-17 — P1.2 React port
- Replaced the P1.1 placeholders with React i18n, router, session, toast, modal, confirmation, print, query and mutation providers; added safe links and pure Indian formatting helpers.
- Restored the prototype CSS and 462-key locale catalog, added JSX-aware translation scanning, mock-backed command wrappers, desktop/mobile view registries, shared components and required screen modules.
- Added React Testing Library coverage for hooks, modal behavior, navigation permissions, hostile text rendering, field errors and a render smoke test for every feature view (38 tests currently pass).
- Ported all setup, account, dashboard, student, attendance, marks, report-card, fee, report, user, activity, backup and settings interactions; every agent-side entry in `docs/PARITY.md` is checked.
- Production `dist/` is 400 KiB; JavaScript is 375,692 bytes (106,570 bytes gzip). `VERIFY_DENY=1 VERIFY_SIZE=1 npm run verify` → ALL CHECKS PASSED. Developer side-by-side visual comparison remains pending.

### 2026-09-17 — P0.1 Apply Edition 2 to the specification
- Installed the Edition 2 book at `docs/PROMPTS_BOOK.md` and split all 45 prompt files, including the new P0.1 and replacement P6.2 backup-drives prompt.
- Added D27–D31 and reconciled AGENTS, UI, architecture, permissions, API, backup format, platforms, dependencies, data defaults, product wording and testing guidance.
- Removed active cloud-backup, external-authorization, legacy renderer and universal-build assumptions; retained only the historical D15/D30 supersession note and explicit forbidden-rendering rule text.
- Added the empty `docs/SIZE.md` ledger and restored `.claude/commands/run.md` with the Edition 2 prompt path.
- Edition 1 code verification is intentionally deferred to P1.1/P1.2 because P0.1 permits document changes only and the new permission/API documents now intentionally lead the old implementation.

### 2026-09-17 — P1.1 Workspace, tooling, size gate and CI
- Installed React 19.3.0, ReactDOM 19.3.0 and the required JSX lint/test tooling; ESLint was set to 9.39.5 because the verified `eslint-plugin-react` 7.37.5 peer range does not support ESLint 10.
- Added React desktop/mobile entry points, a build-time split check, raw-DOM-write lint guard, updated Vite/Testing Library setup, and removed the superseded Edition 1 renderer from the source tree before the P1.2 React port.
- Added the 25/30 MB size checker and tests, wired optional size verification, and updated CI to build/measure the Apple Silicon DMG and Windows NSIS installer before artifact upload.
- Compared release optimization: `"s"` binary 3,403,024 bytes; `"z"` binary 2,535,792 bytes, so `"z"` was retained. Apple Silicon DMG 1.42 MB and installed app 2.57 MB.
- `VERIFY_DENY=1 VERIFY_SIZE=1 npm run verify` → ALL CHECKS PASSED; desktop/mobile production builds and split check passed.

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
