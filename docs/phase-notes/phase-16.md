# Phase 16 handoff — Classroom: timetable, substitutes & attendance duty, homework & notes, report-card remarks, exam seating & hall tickets, calendar

## Start state / environment
- Branch `v2/p16`, cut from `v2/p15` @ `65fa055`. HEAD at write time: the handoff commit
  (below), on top of the six step commits.
- `git status`: clean; all work committed on `v2/p16` (no push/merge/tag).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**.
- Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`; progress lives here;
  prompts in `prompts/`). The `run` skill points at `docs/PROGRESS.md` + a doubled prompt path
  that do not exist — the P16 prompt + `docs/` specs are the authority (as in P11–P15).

## Dependencies check (Standing Rule 4)
No new crates or npm packages. Everything reuses §13 deps (rusqlite, serde, `qrcode` unused here,
`chacha20poly1305` via the existing attachment store/seal, `time`). `cargo tree -i aws-lc-rs` is
empty (ring only). `npm run check:deps` unchanged (31 npm + 34 cargo, all within §13).

## Commits (one per step)
| Commit | Step |
|---|---|
| `8cd0184` | 1 — Timetable (clash detection, Principal week view, teacher My timetable) |
| `f50b996` | 2 — Substitutes + attendance duty |
| `34b7a3e` | 3 — Homework & class notes |
| `379223a` | 4 — Report-card remarks |
| `f4ba4c7` | 5 — Exam seating & hall tickets |
| _(this)_ | 6 — Calendar screen · 7 — tests already land per-step · handoff |

## Tables added (migrations 0023–0027, additive & numbered; `migrations_apply_and_are_idempotent` green at 27)
All belong to the `classroom` module (default **ON**, §14): `scope::module_of_table` maps them to
`"classroom"`, so they stop syncing to a device when the module is off. All carry the standard sync
columns + `school_id`.
- **0023** `period` (session_id, no, starts_at, ends_at) · `timetable_slot` (session_id, class_id,
  weekday ISO 1–7, period_no, class_subject_id, teacher_id, effective_from/to for mid-year changes).
- **0024** `substitution` (date, absent_teacher_id, substitute_teacher_id, class_id, period_no NULL,
  includes_attendance, leave_record_id NULL, created_by).
- **0025** `homework_note` (class_id, class_subject_id, kind homework|notes, text, attachments_json
  `[{name,size,mime,drive_file_id,local_hash}]`, shared_json, created_by).
- **0026** `report_remark` (exam_id, student_id, text, template_key NULL, author) UNIQUE(exam,student)
  · `report_lock` (exam_id PK, finalised_by, finalised_at) · `report_template` (key, language, text)
  seeded **10 keys × en/hi/te = 30 rows** (Hindi/Telugu DRAFT, native review pending).
- **0027** `exam_room` (exam_id, name, rows, cols, invigilator_id NULL, sort_order) · `exam_seat`
  (exam_id, room_id, seat_no, student_id, class_slot) UNIQUE(exam,student) · `exam_schedule`
  (exam_id, date, class_id, class_subject_id NULL, starts_at NULL).

## vidya-core modules & functions added (all pure, all tested)
- `timetable.rs` — `Slot`, `Clash` (teacher_double_booked / class_double_booked / teacher_not_assigned),
  `clashes(slots, assigned)`, `slot_is_valid(candidate, others, assigned)`, `teacher_day(teacher,
  weekday, slots)`, `free_teachers(weekday, period, slots, teachers, on_leave)`.
- `permissions.rs` — attendance duty (§10.4): `AttendanceGrant`, `granted_attendance_classes(grants,
  date)` (ISO-string date range, so it's clockless), `may_take_attendance(actor, class, date, grants)`
  = class-teacher/Principal OR an active substitution/duty grant for that date.
- `notes.rs` — `NoteKind`, `validate_kind`, `validate_attachments` (≤ 10 MB each, ≤ 20 MB total),
  `validate_note`, `can_delete_own(is_author, elapsed_ms)` (24 h window, `DELETE_WINDOW_MS`).
- `report.rs` — `validate_remark` (non-empty, ≤ 300 chars).
- `seating.rs` — `RoomPlan`, `Seat`, `SeatingError`, `generate_seating(rooms)` (interleave two
  classes A/B/A/B, fall back to the remainder, number **column by column**, **deterministic**;
  capacity-short rooms return errors and no seats), `auto_pairs(classes)`.
- **Permissions (§5): 8 new actions, `Action::ALL` 52 → 60** — `ManageTimetable`, `ViewTimetable`,
  `ManageSubstitutes`, `ManageNotes`, `ViewNotes`, `EnterReportRemark`, `FinalizeReportCards`,
  `ManageExamSeating`. Principal: all. Teacher: `ViewTimetable`; `ManageNotes`/`ViewNotes` for a class
  they teach (subject or class teacher); `EnterReportRemark` for a class they class-teach; the four
  Principal-only ones denied. Accountant: no classroom access. All mapped to `Module::Classroom` in
  `modules::module_for`; `role_matrix` + `every_action_maps_to_a_module` still pass.
- `audience.rs` — class-scoped academic data (`timetable_slot`, `homework_note`, `report_remark`) →
  `Class(class_id)`; `period`/`substitution`/`exam_*`/`report_lock`/`report_template` → `Admin`.
- Attendance-duty **request** reuses the P13 registry (`RequestType::AttendanceDuty`, raiser Teacher,
  decider Principal): the grant is conferred by the request reaching `status='approved'` (no separate
  apply), so `spec().apply_available` stays false.

## Rules of note
- **Timetable clashes** are rejected by vidya-core at the slot editor and on copy-week (all-or-nothing).
- **Attendance duty** is by **date** and ends automatically at midnight: a grant for (class, D) does
  not cover D+1 (ISO string range). `may_take_attendance` is the single authoritative gate, called by
  BOTH the attendance command path (`save/submit_attendance`) AND the server op re-validation
  (`sync/apply::apply_op` re-checks attendance ops with `attendance_grants_for` so a legitimate
  substitute's op is accepted, not rejected as "not class teacher").
- **Report remarks** lock like marks: once the Principal makes an exam's report cards final
  (`report_lock`), a teacher can no longer edit that exam's remarks; the Principal still can (audited).
- **Seating** is deterministic for the same rosters + room dimensions; a short room lists how many
  seats are missing and writes nothing.
- **Notes**: study-material only (UI warning); ≤ 10 MB each / ≤ 20 MB total (vidya-core); a teacher
  deletes only their own within 24 h (audited); the Principal may delete any.

## Commands added (all in `commands.json` ↔ `COMMANDS` ↔ `api.ts`, parity test green)
Timetable: `get_timetable`, `my_timetable`, `save_timetable_slot`, `delete_timetable_slot`,
`copy_timetable_week`, `save_periods`. Substitutes: `substitute_plan`, `assign_substitute` (attendance
duty raised via the existing `create_request`, approved via `decide_request`). Notes:
`list_homework_notes`, `save_homework_note`, `delete_homework_note`, `email_homework_note`. Remarks:
`get_report_remarks`, `list_report_templates`, `save_report_remark`, `finalize_report_cards`. Exams:
`list_exam_rooms`, `get_exam_seating`, `save_exam_room`, `delete_exam_room`, `generate_seating`,
`save_exam_schedule`. Calendar: `working_days` (authoritative month count via vidya-core;
`get_calendar` / `add|update|delete_calendar_event` already existed from P13).

## Screens (prototype-matched, built from existing tokens; no component library)
- **Timetable** (`src/screens/desktop/TimetableScreen.tsx`, prototype `timetable` 1–3): class picker,
  days × periods grid (today highlighted), slot editor sheet (class-subject → teacher, clash errors
  inline), **Substitutes sheet** (absent teacher → periods + attendance to cover → free teachers radio
  → "Access ends automatically tonight" → Assign), copy-week sheet, in-place print.
- **Teacher My timetable** (`src/screens/phone/TeacherTimetableScreen.tsx`): own periods, today first;
  footer links to Homework & notes and "Request attendance duty" (a class/date/reason form → request).
  Reached from the teacher home "My classes" tile (`/teacher/timetable`).
- **Homework & notes** (`src/screens/phone/NotesScreen.tsx`, prototype `notes` 1–2): class+subject
  picker (from `my_timetable`), Homework/Class-notes chips, text, attach photos/PDF (metadata), warning,
  Share sheet (WhatsApp / Email / Save in Vidya) + per-class history with own-delete.
- **Report-card remarks** (`src/screens/desktop/ReportRemarksScreen.tsx`, Marks & exams → Remarks):
  exam + class picker, per-student remark input with template suggestions, finalize (Principal), print.
  `ReportCardDoc` now renders the remark block + actual attendance days ("X / Y days · Z%") + class-
  teacher/Principal signatures (prototype `reportcard`).
- **Exam seating & hall tickets** (`src/screens/desktop/ExamsScreen.tsx`, prototype `exams`): rooms +
  invigilators, room grid preview (two-colour by class), Generate seating (capacity errors listed),
  add-room sheet; `HallTicketsDoc` (A4, 4 per page: logo, school, exam, name, class, roll, room, seat,
  schedule, signature) + `SeatingChartDoc` (room grids). Reached from Marks & exams.
- **Calendar** (`src/screens/desktop/CalendarScreen.tsx`, prototype `calendar`): month grid (weekly
  offs muted, today highlighted, events coloured by kind), month nav, working-day count, "Coming up"
  list, Add event (Principal) + delete, **Share on WhatsApp** (month text summary via tap-to-WhatsApp).
  Principal edits; **accountant sees it read-only** (nav item added, edit controls hidden).
- **Nav**: Academics now shows **Timetable** (module `classroom`) + **Calendar**; exam seating +
  remarks are actions on **Marks & exams** (per 03-PROTOTYPE-SPEC's `exams`/`reportcard` mapping),
  not new nav items. Reused icons per Rule 6 (`myClasses` for Timetable, `clock` for Calendar) — no
  new icons invented (P14/P15 precedent), documented here.

## Drive notes layout (§11)
Note attachments are destined for the sync account's `notes/<class>/<yyyy-mm>/` folder (NOT encrypted —
study material). This phase records the note + attachment **metadata** (`drive_file_id` = NULL until
uploaded) and queues email through the P14 pipeline. The **actual Drive file upload** and the **Android
native share sheet** are **deferred** (no Android toolchain / no live Drive in this environment — the
same documented deferral as P14's share plugin and P15's blob transport). Wire + verify on real
devices; the note rows and share/email records are honest in the meantime.

## Print outputs (via the print engine / @page docs)
- Report card (A4, remark + attendance days + signatures) — extends the P7 doc.
- Hall tickets — A4, **4 per page**, with the student's exam schedule.
- Seating charts — A4, one room per section (grid of seats coloured by class, invigilator, signature).
- Timetable — in-place print (AppShell chrome hidden by `@media print`, P7 pattern).
- **Pixel-fidelity screenshots** (`tests/e2e/__screens__`, gitignored) can only be asserted on the
  canonical macOS baseline machine (P11/P13/P14/P15 precedent). New states to regenerate + assert
  there: `timetable`(1–3), `notes`(1–2), `reportcard`, `exams`(1–2), `calendar`. The screens are built
  from the mock tokens (`check-hex` green) and preserve the prototype `data-hl` regions where drawn.

## Owner defaults used (list every open one — Standing Rule + STOP conditions)
- **Notes retention [OWNER default]:** files kept in Drive until the session ends + 1 year — recorded
  as the intended policy; enforcement lands with the live Drive upload/cleanup (deferred above).
- **Report templates:** 10 neutral suggestions seeded; Hindi/Telugu are DRAFTS for native review
  (OWNER-DECISIONS #12). Suggestion language = the school's `settings_json.language`, else English
  (per-staff UI language is client-side, not stored on `staff`).
- **Seating:** auto-pairs adjacent classes (by class sort order); the prompt's "Principal chooses
  pairs" manual mode is a small follow-up (auto-pairing is the default). Alternation is by seat
  fill-order (column-major).
- No STOP condition hit: **no automatic timetable generation** (only clash detection); **notes are
  never public by default** (private `notes/` folder; a public link only if a teacher taps "Get link"
  for large-file email — that UI is part of the deferred Drive wiring).

## Sync / modules / scope
- `module_of_table`: all P16 tables → `"classroom"`. `can_see_table`: accountants receive **no**
  classroom tables. Teachers receive their classes' `timetable_slot` / `homework_note` / `report_remark`
  and their own `substitution` rows; `period` + `report_template` are school-wide reference reads;
  exam_* + report_lock are Principal-only. `report_lock` is server-authoritative (no `id` column, not
  synced); `get_report_remarks` returns the locked flag.
- `sync/apply::action_for` maps `homework_note` → `ManageNotes`, `report_remark` → `EnterReportRemark`;
  attendance ops re-validate via the grant-aware rule.

## Demo seed additions
6 periods + a clash-free V-A weekly timetable; V-A class-subjects (English/Maths/Science/Social) + 3
new subjects; two exam rooms + V-A English added to the Half-Yearly + a small exam schedule (so the
ExamsScreen and hall tickets have data). Teacher-home "My classes" tile now opens My timetable.

## Tests (all real)
Per-step increments landed with each commit; the whole suite is green after each step:
- **vidya-core** lib: timetable (7), notes (4), report (2), seating (5), attendance-duty permission +
  classroom permission matrix rows, audience mappings — added to the existing suite.
- **src-tauri** lib: timetable clashes + my-timetable + role gating; substitute grant on covered day
  only + duty request range + plan/notify + raiser gating; notes save/list/delete/size-limit + email
  consent gating; remark enter/lock/card + cross-class gating; seating determinism + capacity-short +
  hall tickets + role gating; working-days month count.
- **Changed existing expectation (Standing Rule 11):** `permissions::…action_all_covers…` `ALL.len()`
  **52 → 60** (8 P16 actions); `reports_audit_scoping_and_exam_results` — exam-hy now has **2** exam
  subjects (the P16 seed adds V-A English for seating) instead of 1. No other existing test value
  changed; no test weakened or deleted.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo tree -i aws-lc-rs`
  empty. `npm run verify` green: typecheck; check-hex (42 tokens, no new colours); **check-i18n
  en/hi in sync, no TODO-HI** (4 new bundles: timetable, notes, reportcard, exams; calendar +
  academics extended); check-deps within §13; contrast; logs; vitest 38 (api.ts ↔ commands.json parity).

## Follow-ups (documented, non-blocking)
1. **Live Drive note upload** to `notes/<class>/<yyyy-mm>/` (not encrypted) + retention cleanup, and the
   **Android native share sheet** (in-repo `vidya-android` plugin) — deferred (no toolchain / live
   Drive here), as with P14/P15. Note rows + email queue work today.
2. **Teacher phone remark entry** (backend + permissions support it; the desktop entry ships now).
3. **Manual seating pairs** + a schedule editor UI (auto-pairing + `save_exam_schedule` command ship now).
4. **Teacher phone calendar** (read-only) — data syncs; the desktop screen ships for Principal +
   accountant.
5. **Playwright prototype-fidelity baselines** on the canonical macOS machine (`timetable`, `notes`,
   `reportcard`, `exams`, `calendar`).
6. **Native Hindi/Telugu review** of the new strings + the 10 report templates (OWNER-DECISIONS #12).

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
Unchanged list: #1 app identifier · #2 price · #6 Telugu logo · #7 salary formula · #8 leave types ·
#9 remote check-in · #10 retention · #11 UPI QR · #12 native Hindi/Telugu review (now also the P16
strings + report templates) · #13 gmail.send · #14 code signing · #15 GST. New note: **notes
retention** default (session + 1 year) to confirm when the Drive upload lands.
