# PHASE 17 — STAFF HR: STAFF ATTENDANCE, LEAVE TYPES & BALANCES, LEAVE APPROVAL, SALARY & SUBSTITUTE LINKS

## ROLE
You are a senior full-stack Tauri/React/Rust engineer continuing **Vidya Budget School** (v2).

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
14. Run every command you mention and paste real output. Branch `v2/p17`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-17.md`.


## OBJECTIVE
Staff mark their own attendance on their phones, apply for leave, and the Principal approves it in the
same Approvals screen as everything else. Approved leave feeds the salary register and suggests
substitutes automatically.

## DONE MEANS
- Staff check-in/out on the phone (prototype `staffday` state 1); on school Wi-Fi it counts as "at
  school"; away check-ins wait for the Principal (default setting).
- Leave request on the phone (prototype `staffday` states 2–3) → Approvals shows it (prototype
  `leave` state 1) → approve (state 2) → leave record, balance, salary days, substitute suggestion.
- Salary register uses real days present and unpaid leave from this module (the manual days field
  from Phase 15 disappears when the HR module is on).
- Principal sees a staff attendance register (day and month).

## STEPS

### Step 1 — Settings
Settings → Staff HR: school start time (default 09:00), late grace minutes (default 0), "Allow
check-in away from school" (default off → away check-ins need Principal acceptance), leave types
with yearly quotas and paid/unpaid (**[OWNER defaults]** Casual 12 paid, Sick 6 paid, Unpaid —),
quota year = academic session.

### Step 2 — Staff attendance
- Table: `staff_attendance(id, staff_id, date, check_in_at, check_out_at, route lan|drive|manual,
  status present|late|away_pending|absent|leave|half_day, accepted_by NULL, note)` unique
  (staff_id, date).
- vidya-core `hr.rs`: `check_in(now, settings, route)` → status (late if after start + grace; away
  when route ≠ LAN and away not allowed → `away_pending`); one check-in per day; check-out after
  check-in; Principal manual entry/correction audited with reason; non-working days reject check-in
  with a friendly message.
- Time source: the device records its time and HLC; the server stores both and flags a > 10 min
  difference ("phone clock differs") for the Principal.
- Phone (prototype `staffday` state 1): big "Check in" / "Checked in at 8:47 AM" card with route text
  ("On school Wi-Fi" / "Away from school — waiting for Principal"), month counts (present / leave /
  late), recent days list, "Apply for leave". Home tile or card for staff: "Check in" first thing in
  the morning.
- Principal: Staff & access → **Staff attendance** tab: day list (status pills; accept/reject away
  check-ins), month grid, print.

### Step 3 — Leave
- Tables: `leave_type(id, name, name_hi, name_te, yearly_quota, paid, active)`,
  `leave_record(id, staff_id, leave_type_id, from_date, to_date, days, request_id, approved_by)`.
- vidya-core: working-day count via the calendar; balance = quota − approved days this session;
  requests beyond balance allowed only as Unpaid (message says so); overlapping requests rejected.
- `leave` request type (Phase 13 registry): payload {type, from, to, reason}; apply creates the
  leave record, marks staff attendance days as `leave`, notifies the requester.
- Phone form (prototype `staffday` state 2): type segmented, from/to, reason, balance, note about
  substitute; Sent state + toast (state 3). My requests shows it like other requests.
- Approvals (prototype `leave`): pill "Leave request", detail (type, dates, days, reason, balance
  left, classes to cover), Approve / Return / Reject; success panel "Leave approved. Substitutes
  next." with a button that opens the Phase 16 Substitutes sheet prefilled for each affected date.

### Step 4 — Links
- Salary register (Phase 15): days present = working days − absent − unpaid leave; paid leave counts
  as present; `away_pending` not yet accepted counts as present but flagged on the register.
- Substitutes: approved leave lists affected dates on Home → "Needs attention: arrange substitutes
  for Meena Iyer (Thu, Fri)".
- Teacher Home: if on approved leave today, show a calm banner instead of due cards.

### Step 5 — Tests
Check-in statuses (LAN vs Drive route, late, non-working day), clock-difference flag, leave
balances and overlap, unpaid spill-over, request apply idempotent, salary days from HR data (compare
with Phase 15 manual example), substitute suggestion trigger. Prototype fidelity: `staffday` (1–3),
`leave` (1–2).

## STOP CONDITIONS
Rules about PF/ESI, statutory leave law, biometric devices or GPS location → ask (not in scope).

## HANDOFF → `docs/phase-notes/phase-17.md`
Tables · rules · settings defaults · screens · links to salary/substitutes · tests.
