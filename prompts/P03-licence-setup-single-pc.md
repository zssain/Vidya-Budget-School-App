# PHASE 3 of 10 — LICENCE ACTIVATION, SETUP, PIN, RECOVERY KEY, ONE-PC SCHOOL ON REAL DATA

## ROLE
You are a senior Tauri 2 full-stack engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies with the listed features. Anything else → STOP and ask.
4. Never invent features, rules, copy, numbers, API fields, library APIs or config keys.
   Check installed crate source/docs for every API. Unsure → STOP and ask.
5. **The mock wins.** Wiring real data must not change a single pixel of the five mock
   screens (the Phase 1 fidelity test must still pass with the demo seed). New screens use
   only existing components and tokens (derivation rules: context §6.2).
6. Business rules only in `crates/vidya-core`. Commands stay thin.
7. No fake success anywhere.
8. Never delete/weaken tests. Never edit an earlier phase's migration.
9. Run every command you mention; paste real output. Separate environment vs code failures.
10. Work on branch `rebuild/p03`; small commits; no push/merge/tag unless asked.
11. Stop conditions are real.
12. Finish with the handoff file.

## OBJECTIVE
One desktop PC can run a school on its own: buy-code activation (against the dev licence
service), setup wizard, recovery key, PIN, role homes on real data, fee collection with
receipts, attendance, correction requests and the Principal's Approvals screen. No
networking between devices yet.

## DONE MEANS
- Fresh install → Welcome → Set up my school → code from `cloud/licence --dev` → wizard →
  recovery key confirmed → PIN → Principal Home on REAL data.
- Record a payment → receipt number shown; Principal Home "Collected today" updates.
- Submit attendance (phone layout via Android build or desktop dev flag) → locked →
  correction request → Principal approves in Approvals → register changed, audit shows it.
- Demo seed reproduces the mock numbers exactly; the Phase 1 fidelity test passes against
  the REAL screens with the demo seed.
- Audit chain verifies; DB unreadable without key.

## STEPS

### Step 1 — Build config
`src-tauri/build-config/dev.json` and `release.json.example` with keys: `licence_api`,
`licence_public_key` (base64 ed25519), `relay_url`, `google_client_id_desktop`,
`google_client_id_android`. `src-tauri/src/config.rs` loads the file for the build profile
at compile time (`include_str!`). Release build fails (build.rs) if any value is empty.
No other file may contain these values.

### Step 2 — Dev licence service (`cloud/licence`, dev mode only in this phase)
A small separate Rust binary (axum + serde + ed25519-dalek + rand; its own Cargo.toml,
NOT a workspace member of the app so it never ships). `--dev` mode:
- In-memory code store; CLI `cargo run -- gen-code` prints a valid
  `VIDYA-XXXX-XXXX-XXXX` (Crockford base32, excludes I L O U) and stores it.
- Dev ed25519 keypair generated on first run into `cloud/licence/.dev-keys/` (gitignored);
  prints the public key to paste into `dev.json`.
- Implements context §10 exactly: `/v1/activate` (idempotent for the same machine_id,
  409 for another machine), `/v1/check`, `/v1/transfer` (returns 501 NOT_IMPLEMENTED
  until Phase 8 — say so in the handoff).
Phase 10 turns this into the production service.

### Step 3 — App licence module (`src-tauri/src/licence/`)
`machine_id` = random UUID stored in the OS keychain (desktop) — stable across app
updates, new on OS reinstall (documented). `activate(code, school_name)` → POST →
verify signature with vidya-core → store in `licence`. Offline/503 → "Activation needs
internet once — please try again." `CODE_ALREADY_USED` → "This code has already been used
by another school. Contact support." Re-check every 30 days when online; only `revoked`
/ `moved` answers change status (perpetual — no expiry, no grace). Banners for revoked/moved.

### Step 4 — App state machine (`src-tauri/src/state.rs`)
`no_school → activated → setup_in_progress(step) → locked → unlocked(staff) ` plus
`db_key_missing`, `needs_rejoin`, `moved`. Command `app_state()` returns it; the UI routes
from it. Setup progress is stored encrypted; the wizard resumes at the last completed
step after a restart. Sensitive values (PIN, recovery key) are never written to plain
files.

### Step 5 — Commands + typed API
Commands (each: check unlocked session, call vidya-core `can`, run `with_write` for writes,
return DTOs; errors → `{code, message_key, vars}`):
`app_state, activate_licence, setup_school, setup_session, setup_classes, setup_principal,
create_recovery_key, confirm_recovery_key, create_pin, unlock, lock, switch_user,
list_classes, list_students, search_students, get_student, create_student,
get_attendance_sheet, save_attendance_draft, submit_attendance, list_fee_dues,
record_payment, list_payments, create_request, cancel_request, list_requests,
get_request, decide_request, dashboard_principal, dashboard_accountant,
dashboard_teacher, set_accent, verify_audit_chain, seed_demo_school (debug builds only,
`#[cfg(debug_assertions)]`)`.
`src/lib/api.ts`: one typed wrapper per command. A Rust test compares the registered
command list with a shared JSON list that `api.ts` is generated from/checked against;
mismatch fails the build. No JS fallback to fixtures when a command fails — show the error.

### Step 6 — Setup wizard (new screens, derived from the mock)
Desktop, same shell as Welcome (left paper panel + right navy panel; the right panel shows
the step list instead of the headline):
1. **School** — name, address, board (select), UDISE (optional), phone.
2. **Session & terms** — session label auto (April–March, "2026–27"), Term 1 Apr–Sep,
   Term 2 Oct–Mar (editable).
3. **Classes** — default Nursery, LKG, UKG, I–XII section A; add/remove sections; class
   teachers assigned later.
4. **You** — Principal name, mobile (validated), PIN (4–6 digits, entered twice).
5. **Recovery key** — show the 30-char key in 6 groups; "Print" and "Copy" buttons; the
   user must retype groups 3 and 5 to continue. Copy text: the key is needed to restore
   backups and move to a new PC; Vidya cannot recover it.
6. **Ready** — checklist (Licence active · School created · Recovery key saved · Next:
   invite staff, connect Google Drive — both marked "later" until Phases 4–6).
Back keeps entries; each step validates before Continue; errors beside fields with focus
on the first invalid one.

### Step 7 — PIN screens
Unlock screen (derived: Welcome left-panel pattern): school logo/name, staff picker (list
of active staff on this device), 4–6 digit PIN field, error copy with remaining attempts
and wait time (`PIN_WRONG{remaining}`, `PIN_LOCKED{until}`). Auto-lock after 5 min in the
background. Switching user clears all screen state, drafts in memory and pending requests
(test).

### Step 8 — Wire the five mock screens
Replace fixtures with API data through a small store (`src/lib/store.ts`, React context +
`useSyncExternalStore`), keeping every pixel. Add loading (skeleton using existing
surfaces), empty and error states using existing components. Dashboard queries per
context/earlier spec: today's attendance % + "612 of 670 marked · VII-B pending"; unsubmitted
classes after cut-off (default 10:30, setting); collected today + receipt count +
waiting-for-server sum; outstanding this term + students with dues; active students +
admissions this week; 4 latest pending approvals + total (sidebar badge); needs attention
(attendance not submitted, open conflicts, review flags, backup status); attendance by
class; fee collection last 6 school days (skip Sundays).
Collect fee: `validate_collection` on the device; success shows the REAL receipt number
and, in single-PC server mode, "Confirmed by school server". "Print receipt" and "Share on
WhatsApp" stay visible but open a "Coming in a later phase" notice until Phase 7 — do not
fake printing.

### Step 9 — Approvals (new screen, derived)
Principal sidebar "Approvals": Segmented tabs All / Marks / Payment reversal / Attendance
/ Student details / Access; list rows exactly like Home's approval list; right detail
panel in the Sheet pattern: badge, requester, age, reason, BEFORE → AFTER table,
revision history, buttons Approve / Return (note) / Reject (note). Approve → vidya-core
decision → one transaction applying the change + audit + marking `applied`. Stale target
→ show current values and require re-review. Keyboard: J/K move, A approve, R return.
Sidebar badge = pending count.

### Step 10 — Demo seed
`seed_demo_school` (debug only) creates EXACTLY the mock's world: Saraswati Public School;
Priya Sharma (Principal), Suresh Patel (Accountant), Anita Rao, Meena Iyer, R. Nair
(teachers); classes and numbers so every mock value appears (670 students, 612 marked,
VII-B pending, ₹48,500 today from 23 receipts incl. Cash ₹21,000 / UPI ₹27,500, ₹2,400
waiting, ₹6,84,200 outstanding, 212 with dues, 4 admissions this week, the 4 approvals,
the conflict text, 34 V-A students with the mock names, Kavya Singh's dues). Run the
Phase 1 fidelity test against the real screens with this seed.

## EDGE CASES (test each)
Keychain entry missing → `db_key_missing` screen offering Recover (never overwrite the DB)
· licence API down during activation → clear message, retry works · same code retried on
same machine → same licence · code used by another machine → message · wizard killed at
each step → resumes · PIN lockout doubles and persists across restart · midnight rollover
refreshes "today" · user switch clears state · licence `max_students` reached →
`LICENCE_LIMIT` copy (only if non-null).

## VERIFICATION
`cargo test --workspace`, clippy `-D warnings`, `tsc --noEmit`, `vitest`, Playwright:
fidelity (demo seed), setup flow e2e (licence-dev running), payment e2e, attendance +
correction + approval e2e. `sqlite3 vidya.db "select * from school"` fails (encrypted).

## STOP CONDITIONS
A new screen needs a pattern the mock can't express → propose the smallest option and ask.
Licence API questions → ask (Phase 10 owns the real API).

## HANDOFF → `docs/phase-notes/phase-3.md`
State machine diagram (text) · commands with request/response types · new screens with
routes and the mock pattern each derives from · demo-seed numbers vs mock · test results ·
sizes · [OWNER] defaults · questions.
