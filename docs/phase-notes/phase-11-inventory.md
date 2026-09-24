# Phase 11 — Inventory (what exists today, before any v2 code change)

Written at the start of Phase 11 (Step 0), on branch `v2/p11` (from `rebuild/p10` @ `fe92b07`).
One table per area of `docs/02-V2-CHANGES.md`. Status = **exists / partly / missing**.
"Later phase" = the phase that owns the remaining work. All paths are repo-relative.

Tool versions: rustc/cargo/clippy 1.98.1 · node v25.3.0 · npm 11.7.0.

---

## §2 — Zero-monthly-cost platform pieces

| Item | Status | Where it lives today | Later phase / change |
|---|---|---|---|
| `cloud/relay` + relay route | **exists** | `cloud/relay/`; client transport `src-tauri/src/sync/relay.rs`; engine route labels `src-tauri/src/sync/engine.rs:14-36` (`ROUTE_RELAY`) | **P11**: gate behind an off-by-default `instant_sync` module; make `relay_url` optional in build/release checks. |
| `cloud/licence` server, webhooks, admin panel | **exists (P10)** | `cloud/licence/` (own workspace, `exclude`d) | **P12**: mark retired / move to `cloud/_archive/` per §2. Replaced by offline `.vlic` files (§7). Not touched in P11. |
| Company website w/ checkout | **exists as server-rendered site inside `cloud/licence` (web.rs/ui.rs)** | `cloud/licence/src/web.rs`, `ui.rs`, `releases.rs` | **P12**: replace with a static `site/` (UPI QR + mailto), reading `releases.json`. |
| Google Drive as internet route | **partly** | Drive fallback built earlier (`src-tauri/src/sync/drive.rs` per layout); route order LAN→relay→Drive | **P12**: promote Drive to the internet route; two-account model (§6). |
| Build config `relay_url` | **exists, REQUIRED** | `src-tauri/src/config.rs:25-44` (`BuildConfig.relay_url`); `src-tauri/build-config/dev.json`; **release gate** `src-tauri/build.rs:30-44` panics if empty; `scripts/write-release-config.sh`; `.github/workflows/release.yml` (`RELAY_URL`) | **P11**: `relay_url` becomes optional (instant_sync off by default) — relax `build.rs` + release checks. |

## §3 — Attendance (Present/Absent, no cut-off)

| Item | Status | Where it lives today | Later phase / change |
|---|---|---|---|
| Mark type P/A/L | **exists** | `crates/vidya-core/src/types.rs:47-54` `enum Mark { P, A, L }` (UPPERCASE serde) | **P11**: keep `L` for reading legacy rows; reject new `L` (`VALIDATION{field:"mark",rule:"p_or_a"}`). |
| Percentage rule | **exists — already v2-correct** | `crates/vidya-core/src/attendance.rs:125-149` `percent_present(p,a,l)=round_half_up(1000·P/(P+A+L))`; test `percent_leave_counts_in_denominator` (l.314-318) | **P11**: **no formula change** — this already means "legacy L counts as absent" (§3 owner default). Add a P/A-validation test; keep the legacy-% test. |
| DB CHECK on mark | **exists** | `src-tauri/src/db/migrations/0001_init.sql:173-179` `CHECK (mark IN ('P','A','L'))` | **P11**: leave the CHECK (legacy L must still read/write historically); enforce P/A for *new* marks in vidya-core, not the CHECK. |
| Submit completeness | **exists** | `attendance.rs:87-104` `can_submit()` → `IncompleteSheet{remaining}` | unchanged. |
| Migrations loader / highest № | **exists** | `src-tauri/src/db/mod.rs:9-14` embedded `MIGRATIONS`; **highest = 4** (`0004_p08.sql`) | **P11**: add `0005_v2_attendance_pa.sql` (cutover in a `schema_meta` KV) and `0006_v2_modules.sql`. |
| `schema_meta` KV table | **missing** (only `schema_version` exists) | — | **P11**: create `schema_meta(key TEXT PK, value TEXT)` in `0005`. |
| Sync apply for attendance | **exists** | `src-tauri/src/sync/apply.rs:76,87,94-123` routes `attendance_*` ops, resolves `class_id`; reason codes `src-tauri/src/sync/protocol.rs:24-35` (no `LEAVE_MARK_REMOVED`) | **P11**: reject `L` ops with HLC after cutover → `LEAVE_MARK_REMOVED`; pre-cutover `L` applied as legacy. |
| Phone Attendance UI (L button/count/bar/help) | **exists** | `src/screens/phone/AttendanceScreen.tsx` — L style l.92, bar l.338, count l.61/298-301, handler l.474-484; help key `att.hint` | **P11**: remove L button, Leave count, Leave bar segment; help → "34 students · tap P or A". |
| Desktop register (L button + month L column, %) | **exists** | `src/screens/desktop/AttendanceRegisterScreen.tsx` — `Mark` l.14, L style l.23, day L l.166-169, month L col l.195/210, `%` l.211 | **P11**: remove L button + Leave column; render legacy L as muted "Leave (old)" in history/month views. |
| History views of L | **partly** | Report card `src/screens/print/ReportCardDoc.tsx:37` (% only); profile `src/screens/desktop/StudentProfileScreen.tsx:76,126-137` (% only) — no L pill yet | **P11**: add "Leave (old)" pill where per-mark history is shown. |
| Design mock | **exists (shows L)** | `design/screens/Attendance.dc.html:37,40,44,56` | **P11**: owner-approved — remove L button/count/bar/help; regenerate the Attendance fidelity baseline. |
| Attendance i18n | **exists** | phone `src/lib/i18n/strings/attendance.ts:7-53` (`att.count.leave`, `att.hint`, `att.aria.leave`); desktop `attendanceReg.ts` (`areg.leave`, `areg.col.l`) | **P11**: retire live L keys (keep an `att.leaveOld`/"Leave (old)" for history); update `att.hint`. |
| Tests | **exists** | vidya-core `attendance.rs:185-376` (17 tests, incl. L in denom & submit-with-L); Playwright `tests/e2e/fidelity.spec.ts:35-47` (attendance default/markall/submitted) + `tests/e2e/phase1.spec.ts` (mark-all/undo/submit) | **P11**: update submit tests to P/A; add `p_or_a` + cutover tests; regenerate attendance snapshots. |
| Cut-off setting / "10:30" | **fixture only** | `src/dev/fixtures/principalHome.ts:75` "usually done by 10:30" (**dev fixture, not a real setting**); live sub-line already `home.needs.unsubmittedSub`="Not submitted yet today." (`src/lib/i18n/strings/principalHome.ts:57-58`) | **P11**: no cut-off setting exists to remove; fix the fixture; add class-teacher name to the sub-line (prototype: "Class teacher <name> · not submitted yet today"). |
| Low-attendance / "below 75%" | **hardcoded copy** | `src/lib/i18n/strings/reports.ts:52` "Below 75% is highlighted [OWNER default]" (no configurable threshold, no alert engine) | **P11**: change copy to just show the percentage; confirm no threshold setting key exists (none found). |

## §6 — Google accounts / Drive route

| Item | Status | Where | Later phase |
|---|---|---|---|
| Drive fallback (v1: per-staff account, shared folders) | **exists** | `src-tauri/src/sync/drive.rs` (+ `crates/vidya-core/src/audience.rs`) | **P12**: two-account model, LAN→Drive route order, epoch.json fencing. Not in P11. |

## §7 — Licence v2 (offline files)

| Item | Status | Where | Later phase |
|---|---|---|---|
| Online activation API | **exists (P10)** | `cloud/licence` + app `crates/vidya-core/src/licence.rs`, `src-tauri/src/licence/` | **P12**: offline `.vlic` verify, machine code, licence-maker CLI. Not in P11. |

## §8 — Foundation upgrades (items 1–12)

| # | Item | Status | Where / note | Later phase |
|---|---|---|---|---|
| 1 | School calendar backbone / `is_working_day` | **missing** | — | P13. |
| 2 | Guardians table | **missing** (student has inline `guardian_*`) | `student` cols in `0001_init.sql` | P13. |
| 3 | General ledger (voucher/ledger_entry) | **missing** | — | P15. |
| 4 | Numbering engine (`number_series`) | **partly** (receipts/admissions series per device exist) | `crates/vidya-core/src/receipts.rs`, `admissions.rs` | P14/P15 generalise. |
| 5 | Approval engine + new types | **partly** | `crates/vidya-core/src/requests.rs` (fixed type set) | P14/P17 add `leave`,`attendance_duty`,`class_notice`. |
| 6 | Messaging engine | **missing** | — | P14. |
| 7 | Print templates engine | **partly** | `src/screens/print/*` per-doc | P14+. |
| **8** | **Module switches** | **missing** | see §10 row below | **P11 (this phase).** |
| 9 | Custom fields | **missing** | — | later. |
| 10 | Roles as data (`role`/`role_permission`) | **missing** (roles are an enum) | `crates/vidya-core/src/permissions.rs` | P13/P17. |
| 11 | Telugu everywhere | **missing** (`Lang='en'|'hi'`) | `src/lib/i18n/index.ts:26` | P13. |
| 12 | `school_id` on every table | **partly/none** (single-school; no explicit column) | — | later. |

## §9 — Privacy (DPDP)

| Item | Status | Later phase |
|---|---|---|
| Consent / export / erase / retention / incident log | **missing** | P13. |

## §10/§11 — Modules & defaults (the P11 build)

| Item | Status | Where it will hook in | P11 change |
|---|---|---|---|
| Any existing module/feature-flag concept | **missing** | `school.settings_json` exists (`0001_init.sql:22`; used only for `phone`, `commands/logic.rs:388-394`) | **P11**: add a dedicated `module_setting` table (not settings_json). |
| Command choke-point | **exists** | `src-tauri/src/ctx.rs:60-66` `require_session()`; every command: `require_session()?` → `with_db(logic)` (`commands/mod.rs`); registry `commands/mod.rs:27-122`, handler list `lib.rs:200-253` | **P11**: add `require_module` helper called by commands that belong to a toggle-able module (none of the *existing* commands map off-core, so none break; wired as modules are built P14–P17). |
| Server re-validation | **exists** | `src-tauri/src/server/service.rs`; `sync/apply.rs:132-175` (actor → revoked → permission) | **P11**: insert a module check between actor-check and permission-check for module tables. |
| Sync scope (table lists by role) | **exists** | `src-tauri/src/sync/scope.rs:18-21` (`FEE_TABLES`,`MARK_TABLES`), `snapshot()` l.142+, `can_see_table()` | **P11**: exclude disabled-module tables from the pulled snapshot. |
| Instant sync (relay) toggle | **missing** | relay always attempted today | **P11**: `instant_sync` off by default → engine never tries relay; `relay_url` optional. |
| Settings → Languages & modules screen | **missing** | `src/screens/desktop/SettingsScreen.tsx` (Language toggle en/hi in localStorage l.103-117; About l.154-159) | **P11**: add a Languages & modules section matching prototype `settings` state 3 (modules card + languages card en/hi; Telugu added P13). |
| Module keys/defaults | **spec only** | 02-V2-CHANGES §11 | **P11**: seed `accounts=on, classroom=on, hr=on, circulars=on, wa_auto=off, store=off, instant_sync=off` (core has no key, always on). |

## Cross-cutting: relay / cloud-licence / website wiring today

- **Relay**: app client `sync/relay.rs` opens `…/s/<school_id>/v1/sealed`; `relay_url` from build config (`config.rs`), **required** at release (`build.rs`, `write-release-config.sh`, `release.yml`). Instant-sync module (P11) makes this optional.
- **cloud/licence**: separate crate/workspace (never shipped); app calls `/v1/activate|check|transfer` via `licence.rs`. Dev signing key in `.dev-keys`; app verifies with `licence_public_key` in build config. Retired/replaced in P12 (offline files).
- **Website**: currently server-rendered inside `cloud/licence` (`web.rs`), lists downloads from `releases.json`. Becomes a static `site/` in P12.
- **CI/release**: `.github/workflows/ci.yml` (tsc, `npm run verify` = check:hex/i18n/deps/version/contrast/logs + vitest, `check:release-clean`, `cargo test --workspace --locked`, clippy `-D warnings`, aws-lc-rs gate, size gates) and `release.yml` (writes `release.json`, size/SHA). Playwright `test:e2e` is separate.

## Branding / identifier (Steps 2–3 inputs)

- **App identifier `in.vidyabudget.app`** — `src-tauri/tauri.conf.json:5`; Android `namespace`/`applicationId` `src-tauri/gen/android/app/build.gradle.kts:18,21`; macOS `CFBundleIdentifier` in the built `Info.plist`. **No git tags → no public release → not locked.**
- **Branding largely done (P09/P10):** `tauri.conf.json` `publisher="Zuhair Hussain"`, `copyright="© 2026 Zuhair Hussain"`; Welcome footer `welcome.footer.copyright`; `INSTALL.md` "Published by: Zuhair Hussain" + support email. **Remaining P11:** Settings→About (add developer credit + copyright + support email + website), verify `ADMIN-GUIDE.md`, receipt footer (ReceiptDoc has **no** footer today → defer to P14 receipt rework; note it).

## Navigation (Step 7 input)

- Current Principal nav `src/lib/nav.ts:27-61`: Overview[Home,Approvals] · Academics[Students,Attendance,Marks,Reports] · Finance[Fees,Day book] · School[Staff,Sync,Backups,Settings]. Routes in `src/App.tsx` (`desktopRoute` l.175-211); labels `src/lib/i18n/strings/shell.ts:7-26`.
- Prototype target `NAV_PRINCIPAL` (VidyaPrototype.jsx:270-273): Overview[Home,Approvals] · Academics[Students,Attendance,Marks & exams,Timetable,Calendar] · Finance[Fees,Accounts,School store] · School[Staff & access,Circulars,Backups,Settings]. P11: hide unbuilt (Timetable P16, Calendar P13/16, Accounts P15, School store P15, Circulars P14); move Sync into Settings; keep Day book reachable under Fees; **Reports** is not in NAV_PRINCIPAL — keep it reachable under Fees/Finance for now (flag in handoff).
</content>
</invoke>
