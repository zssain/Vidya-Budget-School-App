# Phase 3 handoff — Licence, setup, PIN, recovery key, one-PC school on real data

Branch: `rebuild/p03` (from `rebuild/p02`).
Start HEAD: `84d2fb5` (P02 complete). `git status` at start: clean.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, node v25.3.0, npm 11.7.0. Playwright Chromium present.

## Dependency check
Phase 2 (context §13 core rules + encrypted DB + audit chain + write helper + keys/PIN/recovery)
is **complete** (`docs/phase-notes/phase-2.md`, 264 tests green). All Phase-3 features build only on
Phase-2 primitives + the §13 deps already declared, so no §13 STOP was hit.

## Owner decisions taken this phase
- **Attendance % (mock 91.4% vs the rule):** the mock's "91.4%" cannot come from the owner-approved
  rule `% = P÷(P+A+L)` with "612 of 670 marked" (612 marked → 91.3% or 91.5%, never 91.4%). Owner
  chose **"keep 612/670; show the true 91.3%."** The demo seed produces 559 present / 612 marked /
  670 total → 91.3%. Documented; the mock's 91.4% is a designer-rounded value. (The gallery/fidelity
  path still renders the **fixture's** "91.4%", so fidelity is unchanged — see Tests.)
- **Scope for this session:** owner chose "attempt all 10 steps, label verified vs needs-running-app."
  Steps 1–5, 10 are fully implemented + verified (cargo/tsc/vitest). Steps 6–9 (React) compile (tsc)
  and are structurally complete; pixel-fidelity + click-through of the new screens need the running
  Tauri app to confirm (see "Verification status" per step).
- **`list_staff` command added** (not in the prompt's Step-5 list): the PIN unlock staff picker needs
  it. Added consistently across `commands.json` / `COMMANDS` / handler / `api.ts`.
- Kept Phase-1/2 defaults (accent `#2F7479`, grade scale, lease 30 d, `[Company name]` placeholders).

## State machine (Step 4, `src-tauri/src/state.rs`)
```
                         activate_licence (verify sig)
   no_school ───────────────────────────────────────────► activated
   (no DB / no school)                                     (pending_licence in app_kv)
                                                                │ setup_school (step 1 creates school)
                                                                ▼
   ┌────────────────────────── setup_in_progress(step 1..6) ◄──┘
   │  each step persists to app_kv.setup_step (resume after restart)
   │  step 6 (create_pin marks Ready) ─► setup complete
   ▼
 locked ──── unlock(staff, pin) ────► unlocked(staff) ──── lock / switch_user ────► locked
   (setup done, no session)            (session in RtCtx)

 Off-path (take precedence once a school exists):
   db_key_missing   DB file present but keychain key gone (computed before opening the DB)
   needs_rejoin     any device.needs_rejoin = 1 (restore/transfer; Phase 4+)
   moved            licence.status = 'moved' (licence service said the school moved PCs)
```
`app_state()` returns `{ state: {kind, …}, licence_status }`; the UI routes purely from it
(`src/lib/store.ts routeForState`, unit-tested). Setup progress + the pending licence live in the
encrypted `app_kv` table (migration `0002_p03.sql`). The freshly generated recovery key is held only
in memory (`RtCtx.recovery`) between `create_recovery_key` and `confirm_recovery_key` — never on disk.

## Step 1 — Build config
- `src-tauri/build-config/dev.json` (real dev values; `licence_public_key` filled from the dev service)
  and `release.json.example` (placeholder template). Keys: `licence_api`, `licence_public_key`
  (base64 ed25519), `relay_url`, `google_client_id_desktop`, `google_client_id_android`.
- `src-tauri/src/config.rs` embeds the profile's file with `include_str!` (`dev.json` in debug,
  `release.json` in release) and decodes the public key. `build.rs` **fails a release build** if
  `build-config/release.json` is missing or any value is empty. `release.json` is gitignored.
- **Verified:** `cargo test -p vidya config::` green; dev public key decodes to 32 bytes.

## Step 2 — Dev licence service (`cloud/licence`)
Standalone axum binary with its **own `[workspace]`** and excluded from the app workspace
(`Cargo.toml exclude`), so it never ships. `--dev` serve on 127.0.0.1:8787; `gen-code` mints a
`VIDYA-XXXX-XXXX-XXXX` (Crockford base32, excludes I/L/O/U); `print-key`. Dev ed25519 keypair +
code/activation store persist to `.dev-keys/` (gitignored). Implements §10 exactly:
- `POST /v1/activate` → `200 {licence, signature}`; idempotent for the same `machine_id`; **409
  `CODE_ALREADY_USED`** for another machine; **404 `CODE_NOT_FOUND`**.
- `POST /v1/check` → `{status}`.
- `POST /v1/transfer` → **501 `NOT_IMPLEMENTED`** (Phase 8 owns licence transfer/restore).
- **Verified:** `cargo test` (2) + clippy clean **inside `cloud/licence`**, plus a live `curl` run of
  every endpoint (idempotent-identical body confirmed; signed licence matches `vidya_core::Licence`).

## Step 3 — App licence module (`src-tauri/src/licence/`)
- `machine.rs`: `machine_id` = random UUIDv7 in the OS keychain (service `in.vidyabudget.app`,
  account `machine-id`) — stable across app updates, gone after an OS reinstall (documented). File
  fallback on Android/tests.
- `mod.rs`: `activate` (reqwest POST) + pure `parse_activate` (verify the ed25519 signature via
  `vidya_core::licence::verify`, then bind `server_machine_id` to this machine); `check` (`None` when
  unreachable → stays active, perpetual); `should_recheck` (30 days). Errors map to the exact copy:
  `Unreachable` → "Activation needs internet once — please try again.", `CodeAlreadyUsed` → "This
  code has already been used by another school. Contact support.", plus `CodeNotFound`/`Invalid`.
- The verified licence is stored in `app_kv.pending_licence` at activation and inserted into the
  `licence` table by `setup_school` (the FK needs a school row first).
- **Verified:** unit tests for verify/machine-binding/wrong-key/HTTP-code mapping/recheck timing.

## Step 5 — Commands + typed API
- `src-tauri/src/ctx.rs` `RtCtx` managed state (encrypted conn, session, machine_id,
  `device_mode = Server` for single-PC, http client, in-memory recovery key). `lib.rs` bootstraps it
  (machine-id, DB key via keychain, open+migrate; `db_key_missing` when the key is gone).
- `src-tauri/src/commands/{mod,logic}.rs`: **35 commands** (thin `#[tauri::command]` wrappers over
  pure `*_logic` fns testable without Tauri). Errors → `CmdError { code, message_key, vars }`
  (`error.rs`, `From<CoreError|LicenceError|KeyError>`). Business rules delegate to vidya-core
  (`permissions::can`, `fees`, `receipts`, `validation`); audited writes use `with_write`.
- **Consistency (the Step-5 requirement):** `src/lib/commands.json` is the single source of truth; a
  Rust test asserts `COMMANDS` matches it, and `src/lib/api.test.ts` asserts `api.ts` exposes exactly
  one wrapper per command. All three are in sync (build fails otherwise).

Commands with request → response (camelCase JS args → snake_case Rust; DTOs in `src/lib/api.ts`):
| Command | Request | Response |
|---|---|---|
| `app_state` | – | `AppStateResponse` |
| `activate_licence` | code, schoolName | `AppStateResponse` |
| `setup_school/session/classes/principal` | input structs | `void` |
| `create_recovery_key` | – | `{ key }` |
| `confirm_recovery_key` | group3, group5 | `void` |
| `create_pin` | pin | `void` |
| `unlock` | staffId, pin | `AppStateResponse` (session set) |
| `lock` / `switch_user` | – | `void` (clears session) |
| `list_staff` | – | `StaffDto[]` |
| `list_classes` | – | `ClassDto[]` |
| `list_students` | classId? | `StudentDto[]` |
| `search_students` | query | `StudentDto[]` (FTS5) |
| `get_student` / `create_student` | id / input | `StudentDto` |
| `get_attendance_sheet` | classId, date | `AttendanceSheetDto` |
| `save_attendance_draft` / `submit_attendance` | classId, date, marks | `void` (submit checks INCOMPLETE_SHEET) |
| `list_fee_dues` | studentId | `FeeDuesDto` |
| `record_payment` | input | `PaymentDto` (validate_collection, allocate, receipt no.) |
| `list_payments` | studentId? | `PaymentDto[]` |
| `create_request/cancel_request/list_requests/get_request/decide_request` | … | `RequestDto` / `RequestDto[]` |
| `dashboard_principal/accountant/teacher` | – | dashboard DTOs |
| `set_accent` | hex | `void` (only the 3 offered accents) |
| `verify_audit_chain` | – | `{ ok, first_bad_seq }` |
| `seed_demo_school` (debug only) | – | `void` |

- **Verified:** logic unit tests — `record_payment` numbers the receipt **R-A2-0419** and settles
  Kavya's ₹3,100 dues; overpayment → `AMOUNT_EXCEEDS_DUE`; a teacher is `FORBIDDEN` to record a
  payment; wrong PIN → `PIN_WRONG{remaining}` ×4 then `PIN_LOCKED{until}`; correct PIN unlocks;
  search finds all three Kavyas; the command-list consistency test passes.

## Steps 6, 7, 9 — new screens (derived from the mock, §6.2)
| Screen | File | Route | Derives from |
|---|---|---|---|
| Setup wizard (6 steps) | `src/screens/shared/SetupWizard.tsx` | `activated`/`setup_in_progress` state | Welcome shell (left paper panel + right navy panel showing the step list) |
| PIN unlock | `src/screens/shared/PinUnlockScreen.tsx` | `locked` state | Welcome left-panel pattern (logo/name, staff picker, PIN field, lockout copy) |
| Approvals | `src/screens/desktop/ApprovalsScreen.tsx` | `#/principal/approvals` | Home approval list + Collect-fee Sheet pattern; Segmented tabs; BEFORE→AFTER; J/K/A/R |

`src/lib/store.ts` (React context + `useSyncExternalStore`, per §13) drives routing; `App.tsx` shows
pre-unlock states from `app_state` and routes within the app by hash once unlocked. i18n keys for
errors/licence/setup/pin/approvals added in `src/lib/i18n/strings/errors.ts` (English verbatim from
the prompt where given; `TODO-HI` mirrors for Phase 8; a couple of `[OWNER]` copy defaults flagged
below). The Welcome CTA is wired to `activate_licence` with a **non-visual** edit (added
`value`/`onChange`/`onClick`; no pixels changed — confirmed by the interaction test still passing).

## Step 8 — wiring the mock screens to real data (COMPLETE)
All wiring keeps the fixture/gallery path byte-identical (containers pass real data + optional
callbacks; the mock screens' no-props render is unchanged → fidelity untouched, confirmed by the
interaction tests still passing 5/5).
- **Principal Home:** `containers/PrincipalHomeContainer.tsx` maps `dashboard_principal` → the
  pixel-exact screen (loading + error states). The seed is proven to reproduce the fixture numbers.
- **Collect fee:** `containers/CollectFeeContainer.tsx` lists students + their real dues into the
  table, loads the selected student's statement, and records via **`record_payment`** (shows the
  REAL receipt number). Print / Share open a "coming in a later phase" notice (Phase 7). The screen
  gained non-visual optional props (`onRecord`, `onSelectStudent`, `onCommsNotice`, controlled
  reference + dynamic receipt).
- **Attendance:** `containers/AttendanceContainer.tsx` loads the sheet (`get_attendance_sheet`),
  renders the real roster + marks, and submits/saves via `submit_attendance`/`save_attendance_draft`
  (screen gained non-visual optional `marks`/`onSubmit`/`onSaveDraft`).
- **Teacher home** is a static navigation launcher — no per-school data to inject this phase (its
  dynamic attendance-due badge is a P8 refinement); it renders the fixture.

## Step 9 tail + end-to-end flows (added this session)
- `decide_request` now **applies** the change in one transaction + audits it: `attendance_correction`
  updates the mark, `payment_reversal` appends a `reversal` row (append-only). `marks_correction` /
  `student_details` / `access_change` are approved + audited but `apply_state='not_applied'` (their
  editors land in P7 — honest, no fake apply). `create_request` rejects a duplicate on the same
  target (`REQUEST_ALREADY_PENDING`).
- **`src-tauri/tests/e2e_flows.rs` (4 tests, green)** drive the DONE-MEANS through the real command
  layer + encrypted DB: (1) fresh setup → PIN → unlocked; (2) record payment → R-A2-0419, "collected
  today" grows by ₹3,100, audit valid; (3) attendance A→P correction request → Principal approves →
  **the mark changes** + audit valid + duplicate blocked + teacher forbidden to approve; (4) payment
  reversal request → approve → reversal row appended. **A Tauri-window Playwright e2e cannot run on
  macOS (tauri-driver = Linux/Windows only)**, so these are the portable proof; the browser-driven
  Playwright flow specs for CI are a follow-up (needs a Linux/Windows Tauri build).

## Step 10 — demo seed (`src-tauri/src/seed.rs`, debug only) vs the mock
Internally consistent world; a Rust test asserts every headline number:
| Mock value | Seed produces | Test |
|---|---|---|
| attendance 91.4% *(→ true 91.3%, owner)* | 559 present / 612 marked / 670 → 91.3% | ✔ |
| "612 of 670 marked · VII-B pending" | 612 marked, VII-B draft (pending) | ✔ |
| classes 96·92·94·90·93·91·(pending) | Nursery 96, LKG 92, I-A 94, II-A 90, III-A 93, V-A 91, VII-B – | ✔ |
| collected today ₹48,500 · 23 receipts | 23 confirmed A2-0396..0418 = ₹48,500 | ✔ |
| Cash ₹21,000 / UPI ₹27,500 | 10×₹2,100 cash + (12×₹2,100 + ₹2,300) upi | ✔ |
| ₹2,400 more waiting | 1 unconfirmed (on_device) payment | ✔ |
| outstanding ₹6,84,200 · 212 with dues | 211 general + Kavya = 212 → ₹6,84,200 | ✔ |
| 670 active · 4 admissions this week | 670 students, 4 created today | ✔ |
| fee chart 32k·41k·28.5k·55k·46.2k·48.5k = ₹2,51,200 | 5 prior school days + today (Sun skipped) | ✔ |
| next receipt R-A2-0419 | max A2 seq = 418 | ✔ |
| 4 approvals (Kavya 62→72, R-A2-0418, Rahul A→P, Aarav mobile) | 4 pending requests w/ mock summaries | ✔ |
| V-A 34 mock names incl. Rahul Kumar, Aarav Gupta | seeded verbatim | ✔ |
| Kavya Singh ₹3,100 dues | T1 paid, T2 ₹2,400, Exam ₹500, Transport ₹200 | ✔ |

All staff get demo PIN `1234` (debug-only). Cross-screen note: the mock screens are independent
design snapshots and cannot ALL be simultaneously exact from one DB (e.g. the teacher Attendance
mock shows V-A mid-marking while Principal Home shows V-A submitted at 91%); the seed makes
**Principal Home** exact and seeds V-A submitted for the dashboard.

## Verification (real output)
- `cargo test --workspace` → **300 pass** (vidya lib 50 · e2e_flows 4 · vidya-core 235 · no-floats 1
  · doc 10; the 1,500-student benchmark stays `#[ignore]`). `cloud/licence` tests **2 pass** (separate).
- `cargo clippy --workspace --all-targets -- -D warnings` → **clean** (0). `cloud/licence` clippy clean.
- `npx tsc --noEmit` → **0**. `npx vitest run` → **38 pass** (api-consistency 2 · store-routing 2 ·
  format 34). `npm run build` (tsc + vite) → OK, JS **247.85 kB / 69.62 kB gzip**, CSS 16.47 kB.
- Playwright interactions (`phase1.spec`) → **5/5 pass** (collect-fee, attendance, welcome radios) —
  confirms the mock screens' behaviour is intact after wiring.
- Playwright fidelity (`fidelity.spec`) → app-vs-mock **0.03–0.07** on every screen (welcome 0.03–0.04,
  principal-home 0.07, collect-fee 0.03–0.04, teacher 0.03, attendance 0.03–0.04) — **identical to
  Phase 1**. `collect-fee-success` 0.33 and `attendance-submitted` are the known Phase-1 support.js
  test-driver artifacts (the mock renderer doesn't advance to those states). **≤0.1% is still not met
  (font rasterisation)** — the pre-existing Phase-1 item deferred to P9; **no Phase-3 regression** (the
  four unchanged mock screens are byte-identical to P02; only WelcomeScreen changed, non-visually).
  Baselines are gitignored (regenerated locally, per Phase 1).
- DB unreadable without the key: still proven by the Phase-2 test (unchanged).
- **Not run this phase:** the setup/payment/attendance/approval **e2e against a running Tauri app**
  (needs a Tauri-driven Playwright harness or a real window; the browser `invoke` is unavailable), and
  a **release build / size re-measure** (the Step-1 gate now requires `release.json`; P9 owns sizes —
  note the binary now links reqwest/rustls/ring/ed25519 via the licence module, so it will exceed
  Phase 2's 4.18 MB). Report separately from code.

## Edge cases (status)
- Keychain key missing while a DB exists → `db_key_missing` state (computed before opening; never
  overwrites) — implemented + unit-tested (`security::keys`, `state`). **Recover screen is a stub**
  status message (real recovery is Phase 8 restore).
- Licence API down / same code retried same machine / code used by another machine → mapped to the
  exact copy / idempotent / `CODE_ALREADY_USED` (parse_activate tested; dev service curl-verified).
- Wizard killed at each step → resumes (`app_kv.setup_step`, `state` tested).
- PIN lockout doubles and persists across restart → stored in `staff.pin_fail_count/pin_locked_until`;
  lockout curve tested. Persistence across restart is DB-backed (not yet exercised end-to-end).
- User switch clears the session (`switch_user`); clearing in-memory drafts/pending is a **UI-side
  TODO** when Collect/Attendance containers land.
- Midnight rollover: dashboards take `today` as a parameter (recomputed per call) — handled server-side.
- `LICENCE_LIMIT` copy only when caps are non-null — `check_limits` in vidya-core; UI surfacing TODO.

## Sizes
JS bundle 247.85 kB (69.62 kB gzip), CSS 16.47 kB. Native binary/installer **not re-measured** this
phase (see Verification). Expect it above Phase 2's 4.18 MB now that the licence module links
reqwest/rustls/ring/ed25519; a release measurement needs `build-config/release.json` (Step 1 gate).

## [OWNER] defaults used (please confirm)
1. Attendance stat shows the **true 91.3%** (not the mock's 91.4%) — see Owner decisions.
2. `licence.code_not_found` / `licence.invalid` copy are **sensible defaults** (the prompt only
   specified the offline + already-used copy) — confirm wording.
3. Dev `relay_url = ws://127.0.0.1:8788`, Google client ids empty (unused until P5/P6).
4. Setup wizard board options CBSE/ICSE/State Board; default classes Nursery…XII section A.
5. Added `list_staff` to the command surface (PIN picker).

## What's left in Phase 3 (for the next session)
1. **Browser-driven Playwright flow specs** (setup with `cloud/licence --dev`, payment, attendance→
   correction→approval) to run in **CI on Linux/Windows** — the logic is already proven by the Rust
   `e2e_flows` here; this adds the UI-window pass where tauri-driver is supported.
2. Small polish: show the real **school name** on the PIN screen (add a `get_school` command or an
   `app_state` field); surface **activation errors** inside the Welcome panel (there's a banner now,
   above the panel); auto-lock after 5 min background; clear in-memory drafts on `switch_user`;
   apply `marks_correction`/`student_details`/`access_change` when their P7 editors land.
3. Re-measure release size once `release.json` exists (P9 gate).

## Questions for the owner — ANSWERED (2026-09-23)
1. **Attendance %:** confirmed — show the true **91.3%** (mock's 91.4% is unreachable under the rule).
   `code_not_found`/`invalid` copy: sensible defaults kept (still open to reword).
2. **`list_staff`:** kept as an added command (the PIN staff picker needs a staff source).
3. **Tauri e2e:** a Tauri-window run can't happen on macOS (tauri-driver = Linux/Windows only), so
   the four DONE-MEANS flows are proven by **Rust integration e2e** (`e2e_flows.rs`, runnable
   everywhere); the browser-window Playwright specs are deferred to CI (item 1 above).
