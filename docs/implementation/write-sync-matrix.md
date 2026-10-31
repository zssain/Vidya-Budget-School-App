# Write → sync matrix (P21)

Which writes made on a **joined device** (second PC, Android, iPhone app) reach the
school PC, how, and who receives the canonical result. See `sync-protocol-v2.md` for
the mechanics. "Replay" = the school PC runs the same domain function as the author.

| Domain | Command(s) | Who may run it on a device | Sync | Canonical visibility | Test |
|---|---|---|---|---|---|
| Student attendance | save_attendance_draft, submit_attendance | class teacher, dated substitute/duty, Principal | one atomic `attendance_sheet` action → replay (`attendance_write`); submitted sheets locked | staff of that class + Principal | `attendance_sync::phone_attendance_*`, `sheet_rules_*`, `a_teacher_cannot_take_another_class_*` |
| Marks | save_marks_draft, submit_marks | subject teacher, Principal | one atomic `marks_sheet` action → replay (`upsert_marks`); per-subject lock | own subjects + class-teacher classes, Principal | `phone_marks_reach_the_school_pc_and_submitted_sheet_locks` |
| Leave / requests | request_leave, create_request, cancel_request | requester (raisers per type) | `request` action (`RequestCmd`, device-chosen id) → replay | requester + Principal | `phone_leave_request_and_cancel_*`, `a_teacher_cannot_cancel_someone_elses_*` |
| Staff check-in | staff_check_in, staff_check_out | any active staff, for themselves | `staff_attendance` action → replay; route from the transport (Drive ⇒ never "at school") | own rows + Principal | `lan_check_in_counts_at_school`, `a_drive_check_in_claiming_lan_is_recorded_as_away` |
| Messages / absence alerts | record_message | per purpose (absence alert: own class) | `message` ClientCmd → replay; consent re-checked; email/WhatsApp sent only by the school PC | sender + Principal | `teacher_phone_messages_reads_remarks_and_note_deletes_sync` |
| Circular read | mark_circular_read | any staff | `circular_read` ClientCmd (deterministic id) | reader + Principal | same |
| Report remarks | save_report_remark | class teacher, Principal | `report_remark` ClientCmd | class staff + Principal | same |
| Homework notes | save_homework_note / delete_homework_note | teacher of the class, Principal | insert = complete row; delete = ClientCmd → replay, **tombstone** to other devices | class staff + Principal | `phone_homework_note_lands_complete`, tombstone in the teacher-phone test |
| Payments | record_payment | accountant, Principal | complete `payment` insert → always accepted (§8.6), re-allocated, voucher in the same tx; receipt no. from the device's own series | accountant + Principal | `second_pc_payment_uses_its_own_series_and_lands_once_with_a_voucher` |
| Admissions | create_student | accountant, Principal | `student` ClientCmd → replay with the same id + provisional no.; official admission no. assigned on the school PC | accountant + Principal; teachers for their classes | `second_pc_admission_and_expense_reach_the_school_pc` |
| Expenses | record_expense | accountant, Principal | `expense` ClientCmd → replay; voucher posted on the school PC | accountant + Principal | same |
| Everything else that writes | 69 commands (settings, structure, approvals, staff/devices, corrections, timetable, exams, calendar, circulars, custom fields, privacy, backups, imports, transfers, guardian edits, store sales) | — | **SERVER_ONLY** on a joined device (no fake local success) | — | — |

## iPhone app (PWA) coverage

Ported: PIN/lock, attendance read + save/submit, check-in/out, leave request, absence
alerts (list + record), circulars (list + read), homework notes, teacher home,
timetable, class list, **marks entry** (read + save draft/submit: one `marks_sheet`
action op `{exam_subject_id, submit, entries[]}`, id `msheet-<exam_subject>`) and
**report remarks** (read + save: `report_remark` ClientCmd `save_report_remark`, id
`rmk-<exam>-<student>`) — the same op JSON the native client sends, with the same
local permission checks. Data arrives through the per-device Drive down export.
Not ported (by design): accountant collect-fee (`record_payment` returns
`NOT_AVAILABLE_ON_WEB`; payments stay on the school PCs and the web UI hides the
Collect-fee entry points).
