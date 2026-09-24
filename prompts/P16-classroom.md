# PHASE 16 — CLASSROOM: TIMETABLE, SUBSTITUTES, HOMEWORK & NOTES, REPORT REMARKS, EXAM SEATING & HALL TICKETS, CALENDAR

## ROLE
You are a senior full-stack Tauri/React/Rust + Android engineer continuing **Vidya Budget School** (v2).

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
14. Run every command you mention and paste real output. Branch `v2/p16`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-16.md`.


## OBJECTIVE
Give the school its day-to-day academic tools: a clash-free timetable, substitutes with time-limited
attendance duty, homework and class notes shared to parents, report card remarks, exam seating plans
with hall tickets, and the school calendar screen.

## DONE MEANS
- Timetable (prototype `timetable` state 1) with clash detection; teachers see "My timetable".
- Substitutes (prototype `timetable` states 2–3): assigning gives the substitute that day's attendance
  duty for the class; it expires automatically at midnight (test with clock injection).
- Attendance duty requests (teacher asks → Principal approves) work end to end.
- Homework & notes (prototype `notes`): write offline, attach photos/PDF, share via the Android share
  sheet or email; files land in the sync account's `notes/` folder; class history visible.
- Report card remarks print (prototype `reportcard`).
- Seating plan and hall tickets (prototype `exams`), deterministic for the same inputs.
- Calendar screen (prototype `calendar`).

## STEPS

### Step 1 — Timetable
- Tables: `period(id, session_id, no, starts_at, ends_at)`, `timetable_slot(id, session_id,
  class_id, weekday, period_no, class_subject_id, teacher_id)` (effective from/to for mid-year
  changes).
- vidya-core `timetable.rs`: `clashes(slots)` → teacher double-booked, class double-booked, teacher
  not assigned to that class-subject; `teacher_day(teacher, date)`; `free_teachers(date, period,
  staff, leaves)`.
- Screens: Principal Timetable (prototype `timetable` week view: class picker, days × periods grid,
  today highlighted), slot editor sheet (class-subject + teacher; clash errors inline), copy a week
  to another class, print (print engine). Teacher phone: My classes → **My timetable** (today first).

### Step 2 — Substitutes and attendance duty
- Tables: `substitution(id, date, absent_teacher_id, substitute_teacher_id, class_id, period_no NULL,
  includes_attendance bool, leave_record_id NULL, created_by, created_at)`.
- vidya-core `permissions`: a teacher may take attendance for class C on date D if class teacher of
  C, or an active substitution with `includes_attendance` for (C, D), or an approved
  `attendance_duty` request for (C, D). Expiry is by date (server date in Asia/Kolkata).
- `attendance_duty` request apply (from Phase 13 registry): creates the duty; Principal approval screen
  shows class, dates, reason.
- Substitutes sheet (prototype `timetable` state 2): absent teacher (from approved leave in Phase 17, or
  chosen manually now), periods needing cover + class-teacher attendance, free teachers list
  (radio), note "Access ends automatically tonight", Assign. After assign: grid shows the swap
  (prototype state 3); substitute gets an in-app notification and the class appears on their home.
- Teacher-side: "Request attendance duty" action on My classes (class, dates, reason).

### Step 3 — Homework & notes
- Table: `homework_note(id, class_id, class_subject_id, kind homework|notes, text, attachments_json
  [{name, size, mime, drive_file_id NULL, local_hash}], created_by, created_at, shared_json)`.
- Phone screen (prototype `notes`): class + subject header, Homework / Class notes chips, text,
  attach photos (camera or gallery, compressed via the plugin) and PDFs (≤ 10 MB each, ≤ 20 MB total),
  warning "Study material only — no student photos or marks", **Share with parents** → share sheet
  (prototype `notes` state 2) or Email to the class's parents (queued, Phase 14 pipeline, consent
  rules apply).
- Upload: attachments go to the sync account's `notes/<class>/<yyyy-mm>/` (not encrypted, private
  folder; no public link unless the teacher taps "Get link" for email of large files — then "anyone
  with the link can view", shown clearly). Works offline: queued until online.
- History: per class list (all teachers of that class + Principal can see); teachers delete only
  their own within 24 h (audited); files kept in Drive until the session ends + 1 year **[OWNER
  default]**.

### Step 4 — Report card remarks
- Table: `report_remark(id, exam_id, student_id, text, template_key NULL, author, …)`; templates in
  en/hi/te (Settings → Report cards; seeded with 10 neutral templates marked for review).
- Teacher (class teacher) enters remarks per student on phone/desktop (list with suggestions); locked
  when the Principal marks report cards final for that exam (same lock pattern as marks).
- Report card print (prototype `reportcard`): remarks block, actual attendance days from the
  calendar, signatures line.

### Step 5 — Exam seating and hall tickets
- Tables: `exam_room(id, exam_id, name, rows, cols, invigilator_id NULL)`, `exam_seat(id, exam_id,
  room_id, seat_no, student_id)`, `exam_schedule(id, exam_id, date, class_id, class_subject_id,
  starts_at)`.
- vidya-core `seating.rs`: pair classes (Principal chooses pairs or auto-pairs adjacent classes),
  alternate seats between the two classes, fill column by column, deterministic; errors when
  capacity is short (lists how many seats are missing).
- Screens (prototype `exams`): rooms list with invigilators, room grid preview, "Generate seating",
  "Print seating charts", "Print hall tickets" (4 per A4: logo, school, exam, name, class, roll,
  room, seat, schedule, signature line) via the print engine.

### Step 6 — Calendar screen
Prototype `calendar`: month grid (weekly offs muted, today highlighted), events coloured by kind
(holiday / exam / event), "Coming up" list, working-day count for the month, Add/edit (Principal),
"Share on WhatsApp" (text summary of the month via tap-to-WhatsApp), link from circulars. Teachers
and accountants view only.

### Step 7 — Tests
Timetable clashes; free-teacher search; duty expiry at midnight; attendance permission matrix incl.
substitutes; notes offline queue + Drive upload (fake Drive) + share plugin call; remark lock;
seating determinism and capacity errors; calendar working days. Prototype fidelity: `timetable`
(1–3), `notes` (1–2), `reportcard`, `exams` (1–2), `calendar`.

## STOP CONDITIONS
Timetable rules beyond clash detection (e.g. automatic timetable generation) → ask. Any need to make
notes public by default.

## HANDOFF → `docs/phase-notes/phase-16.md`
Tables · rules · screens · Drive notes layout · print outputs · owner defaults · tests.
