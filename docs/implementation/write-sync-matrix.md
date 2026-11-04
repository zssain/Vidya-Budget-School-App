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
| Request decisions (W1) | decide_request (approve / reject / return) | active Principal on a live device | `request` action `RequestCmd::Decide` → replay (`decide_request_with`): role, pending state (`WHERE status='pending'` in the tx → `ALREADY_DECIDED`) and the off-server policy re-checked; effects (leave record, corrected mark) written only on the school PC, audited with the device | Principal (requester sees the outcome) | `w1_decisions_store_sync::principal_on_a_second_pc_*`, `a_decision_on_an_already_decided_*`, `a_non_principal_*`, `a_revoked_device_*`, `a_duplicate_decision_*` |
| Away check-in decisions (W1) | accept_away_checkin, reject_away_checkin | active Principal (`hr` module on) | `staff_attendance` action `{cmd: accept_away_checkin\|reject_away_checkin, id}` → replay; row must still be `away_pending` | own rows + Principal | same |
| Store sales (W1) | record_store_sale | accountant, Principal (`store` module on) | `store_sale` ClientCmd `record_store_sale` (device-chosen id, printed S-receipt no., prices charged) → replay: recorded **once** with its voucher, never refused for stock/price; oversell → `store_oversold` flag, price/receipt mismatch → `store_sale_review` flag | accountant + Principal | `a_second_pc_store_sale_replayed_twice_*`, `an_oversold_replay_*`, `store_sale_ops_respect_*`, `a_decision_and_a_store_sale_also_arrive_over_drive` |
| Everything else that writes | 66 command gates (settings, structure, staff/devices, money corrections incl. approving a payment reversal from a device, timetable, exams, calendar, circulars, custom fields, privacy, backups, imports, transfers, guardian edits, store catalogue/stock, manual staff-attendance entry, conflict/review-flag resolution) | — | **SERVER_ONLY** on a joined device (no fake local success) | — | — |

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
Refused ops (rejected / conflict / flagged in the school PC's ack) stay in the PWA
outbox with their reason and are listed by `list_sync_problems` — shown on the
phone's About screen and as a Teacher-home notice (never dropped silently).
The PWA's session/audience keys are PIN-wrapped (repair K2), so its Drive loop only
runs while the app is unlocked: passes are skipped on the PIN screen and one runs
immediately on unlock (queued ops are never lost).
