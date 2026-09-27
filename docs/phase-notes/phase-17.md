# Phase 17 handoff — Staff HR: staff attendance, leave types & balances, leave approval, salary & substitute links

## Start state / environment
- Branch `v2/p17`, cut from `v2/p16` @ `4fcf614`. HEAD at write time: the handoff
  commit, on top of the five step commits (below); `git status` clean.
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**.
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives
  here; prompts in `prompts/`). The `run` skill still points at paths that don't
  exist — the P17 prompt + `docs/` specs are the authority (as in P11–P16).

## Dependencies check (Standing Rule 4)
No new crates or npm packages. Everything reuses §13 deps (rusqlite, serde, `time`).
`cargo tree -i aws-lc-rs` → not in the tree (ring only). `npm run check:deps` unchanged
(31 npm + 34 cargo, all within §13).

## Commits (one per step)
| Commit | Step |
|---|---|
| `bc1c9fb` | 1 — Settings → Staff HR (check-in policy + leave types) |
| `c4f92c0` | 2 — Staff attendance (check-in/out, register, away accept/reject, manual entry) |
| `5326d06` | 3 — Leave requests, approval, and the leave record |
| `2ff5637` | 4 — Salary + substitute links |
| `7a2d7fb` | 5 — Tests + full verification |
| _(this)_ | handoff |

## Tables added (migrations 0028–0030, additive & numbered; `migrations_apply_and_are_idempotent` green at 30)
All belong to the `hr` module (default **ON**, §14); `scope::module_of_table` maps them
to `"hr"` so they stop syncing when the module is off. All carry the standard sync
columns + `school_id`.
- **0028** `leave_type` (name/`_hi`/`_te`, `yearly_quota` NULL=unlimited, `paid`, `active`,
  `sort_order`). Seeds the **[OWNER defaults]** Casual 12 paid · Sick 6 paid · Unpaid
  (no quota). Hindi/Telugu names are DRAFTS (native review pending, OWNER #12).
- **0029** `staff_attendance` (staff_id, date, check_in_at/check_out_at = server UTC ISO,
  check_in_min/check_out_min = device-local minutes, route lan|drive|manual,
  status present|late|away_pending|absent|leave|half_day, accepted_by, clock_warning,
  note) UNIQUE(staff_id, date). Not append-only (a day's row is updated); the audit log
  records every change.
- **0030** `leave_record` (staff_id, leave_type_id, from_date, to_date, days, paid_days,
  unpaid_days, request_id, approved_by).

## vidya-core (all pure, all tested)
- **`hr.rs`** (new module): `Route` (lan/drive/manual), `AttnStatus`, `HrSettings`
  (start_min/grace_min/allow_away), `parse_hhmm`/`fmt_hhmm`, `validate_hr_settings`,
  `check_in_status(check_in_min, settings, route)` (away route + away-not-allowed →
  `away_pending`; else late after start+grace, else present), `accepted_away_status`,
  `clock_warning`/`clock_skew_minutes` (> 10 min flag), `leave_balance`,
  `split_leave_days` (paid + unpaid spill-over), `ranges_overlap`/`overlaps_any`,
  `validate_leave_dates`, `days_present(working_days, absent, unpaid_leave)` (the salary
  link). 10 tests incl. the prototype's 8:47 check-in and the "9 of 12" balance.
- **permissions.rs**: 2 new actions, `Action::ALL` **60 → 62** — `StaffCheckIn`
  (everyone-allow, `Own`; every active staff marks their own attendance) and
  `ManageStaffHr` (Principal only; register, manual entry, accept away, leave types /
  settings). Both mapped to `Module::Hr` in `modules::module_for`. `role_matrix`,
  `matrix_shape_matches_section_5` and `default_permissions` still pass (the everyone /
  Principal derivations pick the new actions up automatically).
- **requests.rs**: `RequestType::Leave` is now `apply_available = true` (approval writes
  the leave record). **Changed test** `apply_available_matches_current_behaviour` now
  includes Leave (Rule 11).

## Rules of note
- **Check-in** (`staff_check_in`): one per working day; a non-working day is rejected
  with a friendly message; a check-in away from school (route ≠ LAN) waits for the
  Principal (`away_pending`) unless "allow away" is on; the device time + HLC are stored
  and a > 10-min difference from the server clock sets `clock_warning` for the Principal.
  Route is derived from LAN reachability of the school server (see the deferral below).
- **Away accept/reject**: the Principal accepts an `away_pending` (→ present/late by the
  time rule) or rejects it (→ absent), audited.
- **Leave** rides the P13 approval registry: `request_leave` validates the working-day
  count, rejects overlaps, and computes the paid/unpaid split against the type's balance
  (unpaid spill-over). Approving via `decide_request` writes the `leave_record`, marks
  each working day `leave` (note = the type name), and notifies the requester. A leave
  request created via the generic `create_request` with no leave payload is approved but
  writes no record (guarded). Balance = quota − approved days **this session**.
- **Salary link**: with the HR module on, the salary register's days present come from
  `working_days − absent − unpaid_leave`; paid leave and a not-yet-accepted `away_pending`
  count as present (the register flags away with `away_flagged`). The manual days-present
  field disappears (`hr_on`). With HR off, the P15 manual field still works.
- **Substitute link**: an approved class-teacher leave surfaces on Principal Home
  ("Arrange substitutes for <name>", with the uncovered dates); the Approvals leave
  success panel opens the P16 Substitutes sheet prefilled (`?sub=<teacherId>&date=<date>`);
  `assign_substitute` ties the substitution to the covering leave (`substitution.leave_record_id`).

## Commands added (all in `commands.json` ↔ `COMMANDS` ↔ both `invoke_handler` lists ↔ `api.ts`, parity test green)
Settings: `get_hr_settings`, `set_hr_settings`, `list_leave_types`, `save_leave_type`.
Attendance: `my_staff_day`, `staff_check_in`, `staff_check_out`, `staff_attendance_day`,
`staff_attendance_month`, `accept_away_checkin`, `reject_away_checkin`,
`mark_staff_attendance`. Leave: `request_leave` (approval via the existing
`decide_request`). Every write is one transaction (row + audit + op for the
device-originating check-ins; row + audit for Principal register writes, synced by
snapshot as with P16 substitutions).

## Settings defaults (§10.5, OWNER-DECISIONS #8/#9)
Stored in `school.settings_json.hr`: **school start time 09:00 · late grace 0 minutes ·
"allow check-in away from school" OFF** (away check-ins need Principal acceptance). Leave
types: **Casual 12 paid · Sick 6 paid · Unpaid** (quota year = the academic session).
All editable in Settings → Staff HR (shown only when the `hr` module is on).

## Screens (prototype-matched, built from existing tokens; no component library)
- **Settings → Staff HR** (`src/screens/desktop/StaffHrSettings.tsx`): start time + grace +
  allow-away toggle; leave types (name / quota / paid|unpaid / active) with add + save.
- **Phone My attendance / check-in** (`src/screens/phone/MyAttendanceScreen.tsx`, prototype
  `staffday` state 1): check-in card ("Check in" → "Checked in at 8:47 AM" with route text),
  month counts (present / leave / late), recent-days list, "Apply for leave"; route
  `/teacher/checkin`.
- **Phone Apply for leave** (`src/screens/phone/ApplyLeaveScreen.tsx`, prototype `staffday`
  2–3): type segmented, from/to, reason, balance, substitute note; Sent + toast; route
  `/teacher/leave`.
- **Teacher Home check-in card** (`TeacherHomeScreen.tsx`): a card that shows "Check in" /
  "Checked in at …" / the calm "on approved leave today" state; hidden when HR is off.
- **Principal Staff & access → Staff attendance tab** (`StaffAttendanceRegister.tsx`): day
  list (status pills; accept/reject away; clock-differs flag), month grid, print. The tab
  appears only when HR is on.
- **Approvals → Leave** (`ApprovalsScreen.tsx`): a "Leave" tab + leave detail (type, dates,
  days, reason, balance left, classes to cover, unpaid note) + Approve/Return/Reject; a
  success panel "Leave approved. Substitutes next." whose button opens the P16 Substitutes
  sheet prefilled. `TimetableScreen`'s `SubstitutesSheet` gained optional prefill props.
- **Salary register** (`SalaryRegister.tsx`): the editable days field disappears when HR is
  on; away check-ins are flagged with `*`.
- **Nav**: no new nav item — Staff attendance is a tab on the existing "Staff & access"
  screen (per 03-PROTOTYPE-SPEC's `staff`/`staffday` mapping). Reused icons `clock` /
  `check` for the check-in card (no `login`/door icon exists in the mock set — the P16
  reuse precedent, Rule 6).

## Links to salary / substitutes (Step 4)
- Salary: `salary_register` → `hr_days_present` (attendance + unpaid leave) when `hr` on;
  `SalaryRowDto.away_flagged`, `SalaryRegisterDto.hr_on`.
- Substitutes: `dash::substitute_needs` (Principal Home) + the Approvals→Substitutes-sheet
  prefill + `substitution.leave_record_id`.
- Teacher Home: the check-in card shows the calm leave state when today's status is `leave`.

## Sync / modules / scope
- `module_of_table`: `leave_type` / `staff_attendance` / `leave_record` → `"hr"`.
- `snapshot`: leave types are reference data for every role; the Principal gets all
  `staff_attendance` + `leave_record`; a teacher/accountant gets only their OWN rows.
  `visible_row` enforces the same per-row.
- `sync/apply::apply_op`: a `staff_attendance` op re-validates as `StaffCheckIn` when the
  row's `staff_id` is the author (self check-in) and `ManageStaffHr` otherwise (a Principal
  manual entry), so a staff member cannot forge another staff member's attendance; module
  gated (`hr`). `leave_type`/`leave_record` ops → `ManageStaffHr`.

## Demo seed
A fortnight of teacher check-ins up to yesterday (present, in 8:45 / out 3:40), with
Meena's most-recent working day an `away_pending` (to demo accept/reject) and Nair an
occasional late. Approving a seeded/created leave then lights up the Home "arrange
substitutes" item and the salary-from-HR days.

## Tests (all real, this phase)
- `cargo test --workspace --locked` green: **vidya-core 414 lib + 7 role_matrix**,
  **app 290 lib** (+15 this phase), all integration bins (drive/relay/sync/e2e/durability).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` clean;
  `cargo tree -i aws-lc-rs` not present (ring only).
- `npm run verify` green: typecheck; **check-hex 42 tokens, no new colours**;
  **check-i18n 30 modules, 1473 keys, en/hi in sync, no TODO-HI**; check-deps within §13;
  contrast; logs; **vitest 38** (api.ts ↔ commands.json parity + format + store).
- New tests of note (per the prompt Step 5 checklist): check-in statuses (LAN vs Drive,
  late, non-working day rejected); the > 10-min clock-difference flag; leave balances +
  overlap; unpaid spill-over; leave apply idempotent (no double record); salary days from
  HR data = the **P15 R. Nair anchor** (26 working days, 2 unpaid → ₹1,308 deducted, net
  ₹15,692); the Home substitute-suggestion trigger + `leave_record_id` link; the scope
  gate (a teacher sees only their own attendance; leave types are reference data).

### Changed test expectations (Standing Rule 11)
1. `vidya_core::requests` `apply_available_matches_current_behaviour` — now includes
   `Leave` (leave auto-applies on approval).
2. `permissions` `action_all_covers_every_variant…` `ALL.len()` **60 → 62** (StaffCheckIn,
   ManageStaffHr).
3. `commands::logic` `teacher_can_raise_a_leave_request_and_it_stores_pending` — approving a
   leave now marks it `apply_state='applied'` (was `not_applied`; a bare P13 payload writes
   no record, but the request is applied).
4. `commands::logic` `salary_register_and_pay_posts_balanced_vouchers` — turns the `hr`
   module off to exercise the P15 **manual** days-present path (the register uses HR data
   when HR is on; the HR-driven path has its own test).
No other existing test value changed; no test weakened or deleted.

## Prototype fidelity (deferred, sandbox precedent)
Playwright pixel fidelity for `staffday`(1–3) and `leave`(1–2) **cannot be asserted here**
(`tests/e2e/__screens__` baselines are gitignored, only assertable on the canonical macOS
machine — P11/P13/P14/P15/P16 precedent). The screens are built from the mock's tokens
(check-hex passes) and preserve the prototype `data-hl` regions (`checkin`, `dates`,
`balance`, `send`, leave `detail`/`done`). Regenerate + assert on the baseline machine.
One deliberate deviation to regenerate with owner approval (as the P11 attendance change):
the **Teacher Home** now shows a small check-in card the original mock does not draw.

## Follow-ups (documented, non-blocking)
1. **Live LAN route detection on a real phone**: `staff_check_in` takes the route from a
   `server_status` reachability probe (honest in the single-PC + LAN case). Wiring the
   phone's live LAN probe into the route is part of the broader multi-device sync work
   deferred since P14 (Android toolchain / live LAN not in this sandbox).
2. **Server-side recompute of `clock_warning`** when a client check-in op is applied on a
   different machine (today it's computed at record time — correct for the single-PC case).
3. **Cross-month unpaid-leave attribution** in the salary register: a leave record's unpaid
   days are attributed to the month of its `from_date` (a leave spanning a month boundary is
   a rare edge case; documented).
4. **Accountant phone check-in**: the accountant is on the desktop; their check-in is via
   the Principal's manual entry today. A desktop check-in card is a small follow-up.
5. **Native Hindi/Telugu review** of the new `staffhr` strings + the seeded leave-type names
   (OWNER-DECISIONS #12); `te` falls back to English for the new bundle.
6. **Playwright prototype-fidelity baselines** for `staffday`/`leave` on the canonical macOS
   machine + the Teacher-Home baseline regeneration.

## STOP conditions (prompt) — none hit
No PF/ESI, statutory leave law, biometric devices or GPS location were touched: check-in
uses the app's own LAN reachability (not GPS), leave is school-configured types (not
statutory law), and there is no biometric hardware. The salary deduction reuses the P15
**[OWNER default]** formula unchanged.

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
This phase builds the stated defaults for **#8 leave types/quotas** (Casual 12 / Sick 6 /
Unpaid) and **#9 remote staff check-in** (away over non-LAN needs Principal acceptance;
"allow away" default off). Also open, unchanged: #1 app identifier · #2 price · #6 Telugu
logo · #7 salary formula (reused) · #10 retention · #11 UPI QR · #12 native Hindi/Telugu
review (now also the P17 strings + leave-type names) · #13 gmail.send · #14 code signing ·
#15 GST.
