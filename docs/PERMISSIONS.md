# PERMISSIONS.md — who can do what

The table below is **machine-checked**: a test in `vidya-core` parses this table and fails if `permissions.rs` differs. Keep the exact format: one action per row, values `yes`, `no` or `own` (teacher's assigned sections only).

| Action | Principal | Accountant | Teacher |
|---|---|---|---|
| students.view | yes | yes | own |
| students.view_fees | yes | yes | no |
| students.view_contact | yes | yes | own |
| students.add | yes | yes | no |
| students.edit | yes | yes | no |
| students.set_concession | yes | no | no |
| students.mark_left | yes | yes | no |
| students.import | yes | yes | no |
| students.export | yes | yes | no |
| students.issue_tc | yes | no | no |
| attendance.view | yes | no | own |
| attendance.mark_today | yes | no | own |
| attendance.edit_past | yes | no | no |
| attendance.print_register | yes | no | own |
| marks.view | yes | no | own |
| marks.enter | yes | no | own |
| reportcard.view | yes | no | own |
| reportcard.print | yes | no | own |
| fees.view | yes | yes | no |
| fees.collect | yes | yes | no |
| fees.cancel_receipt | yes | no | no |
| fees.daybook | yes | yes | no |
| fees.export | yes | yes | no |
| reports.view | yes | no | no |
| reports.export | yes | no | no |
| users.view | yes | no | no |
| users.manage | yes | no | no |
| activity.view | yes | no | no |
| settings.view | yes | no | no |
| settings.edit | yes | no | no |
| session.change | yes | no | no |
| backup.manage | yes | no | no |
| backup.run | yes | yes | no |
| devices.view | yes | no | no |
| devices.approve | yes | no | no |
| devices.manage | yes | no | no |
| alerts.view | yes | yes | no |
| license.view | yes | no | no |
| account.change_own_password | yes | yes | yes |
| account.set_own_language | yes | yes | yes |

## Office computer only
These actions are refused for requests from phones, whatever the role.
- `backup.run`
- `backup.manage`
- `session.change`
- `students.import`
- `settings.edit`
- `users.manage`
- `devices.approve`
- `devices.manage`

## Rules the table cannot express
1. **setup.run** is allowed only when the database has no school, and only on the office computer. No session exists yet, so it is not a role action.
2. **Principal role** can never be created by `users.manage`. Only setup creates the principal. Enforced in service and database index.
3. **attendance.mark_today** for a teacher: date must equal today on the office computer's clock (phones: today by server-corrected time). Principal with `attendance.edit_past` may use any date up to today, never future.
4. **Own** means the target section is in `user_sections` for that teacher **at the time the server applies the change**. A phone change made before the principal removed the section is rejected on sync.
5. **Reads are filtered, not just blocked.** A teacher calling a student list gets a DTO type without fee fields (`StudentTeacherDto`). The type must not contain the fields at all.
6. **alerts.view** for accountants shows only fee alerts (`overpayment`).
7. **Switched-off, locked or must-change-password users** have no permissions except `account.change_own_password` during first sign-in (pending token).
8. **Phones** additionally cannot perform: setup, license, the office-computer-only actions above, devices.*, or reports.* — these commands are not compiled into the Android build where possible and are always refused by services.
9. **Concession field** sent by a non-principal is an error (`permission`), not silently ignored.
10. **students.add on a phone** requires a live connection to the office computer (admission number allocation).

## DTO visibility by role
| DTO | Principal | Accountant | Teacher |
|---|---|---|---|
| Student basic (name, class, section, roll, gender, dob, father, mother) | yes | yes | own |
| Student contact (mobile, locality) | yes | yes | own |
| Student flags (RTE, bus, category, Aadhaar, APAAR) | yes | yes | no |
| Student fees (due, paid, balance, receipts, concession) | yes | yes | no |
| Attendance and marks | yes | no | own |
| Users (list) | yes | no | no |
| Change log | yes | no | no |
