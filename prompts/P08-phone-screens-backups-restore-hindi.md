# PHASE 8 of 10 — TEACHER PHONE SCREENS, BACKUPS, RESTORE, NEW SESSION, SETTINGS, STORAGE, HINDI

## ROLE
You are a senior full-stack Tauri/React/Rust engineer finishing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`. Reopen `TeacherHome.dc.html` and `Attendance.dc.html` for every
   phone screen.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies. Anything else → STOP and ask.
4. Never invent features, rules, copy, numbers or library APIs. Unsure → STOP and ask.
5. **The mock wins.** Phone pages = Attendance navy-header pattern + light body + bottom
   action bar; lists = AttendanceRow/TaskCard patterns. No new colours, sizes, radii.
6. Business rules only in vidya-core.
7. No fake success: "Backed up" only after verification; restore only after checks pass.
8. Every string via `t()`; Hindi must be complete at the end of this phase.
9. Never delete/weaken tests. Never edit an earlier phase's migration.
10. Run every command you mention; paste real output. Work on branch `rebuild/p08`.
11. Stop conditions are real. Finish with the handoff file.

## OBJECTIVE
Teachers can do all their work on a cheap phone; the school is protected by verified
backups and can move to a new PC; a new academic session can start; every setting works;
the whole app is in natural Hindi. After this phase NO placeholder remains anywhere.

## PART A — TEACHER PHONE (the nine tiles in `TeacherHome.dc.html`)
Due cards on Teacher Home are computed: today's unsubmitted attendance for classes where
they are class teacher; returned requests; marks drafts past the exam end date. First card
pulses (mock `vPulse`).
1. **Attendance** — class picker if they teach more than one class-teacher class; date
   limited to today (new) and the last 7 days (view/drafts/corrections); submitted sheets
   open read-only with "Request correction" (student, old → new mark, reason).
2. **Marks** — exam → their class-subjects → one row per student (AttendanceRow layout
   with a numeric input 44px tall instead of P/A/L, `inputmode="numeric"`, AB toggle);
   "Next" moves to the next student; Save draft; Submit subject (locks it; confirmation
   names the subject and class); submitted → read-only + "Request correction". 60 students
   must scroll smoothly on a 2 GB phone (hand-written windowing only if you measure jank).
3. **Report cards** — pick class + exam → list → open a report card preview → Print /
   Share via the Android print plugin (Phase 7) or the OS share sheet (PDF from print).
4. **My classes** — cards per class/subject with student count and today's status.
5. **Students** — scoped, read-only list + profile (no fee data; address only if class
   teacher).
6. **My requests** — list with pills; detail timeline; returned → the Principal's note +
   "Edit and resend"; cancel while pending.
7. **Inbox** — decisions, rejected changes (Edit & resend / Discard), conflict results,
   Drive/relay/lease notices; read/unread.
8. **Sync** — current route, last confirmed sync, pending count, Sync now, server address,
   Drive account.
9. **Profile** — change PIN, language, Google account (connect/disconnect), sign out,
   "Remove this phone" (PIN-confirmed; unsent work warning).
Android back button: pops screen; on Home asks before exit only if drafts are unsaved.
Principal on Android (if they install on a phone): same client role screens allowed by
context §5 via the server, but never server setup, restore or hosting (hide those).

## PART B — BACKUPS (server only; `src-tauri/src/backup/`)
Scheduler checks every minute: 06:00 daily + on quit if none today. Filename
`vidya-<school-slug>-YYYYMMDD-HHMM.vbak`. SQLCipher export keyed by the backup key → verify
(open, `integrity_check`, row counts, audit chain head matches) → temp name, fsync, rename
→ local `<AppData>/Vidya/backups` → upload to Drive `backups/` (resumable > 5 MB) → verify
checksum. Retention 30 daily + 12 monthly per destination (vidya-core
`select_for_deletion(list, today)` with tests). **Backups screen**: last verified band,
runs list (time, destination, size, Verified / Partial / Failed pill), Back up now,
"Copy to a USB drive" (export dialog), retention info. Drive unavailable → local only,
"Partial" + Home notice. Disk full → clean failure, nothing half-written. Backup success
time on Home comes ONLY from a verified run.

## PART C — RESTORE (Welcome → Recover an existing school)
Pick `.vbak` from file, or sign in to Google and list Drive `backups/` → recovery key
(3 wrong → 30 s wait) → verify → summary (school, backup date, students, payments, last
receipt no., audit chain OK) → warning if older than 1 day: "Receipts after <date>
recorded on this computer will be missing" → licence `/v1/transfer` (implement the app
side now; `cloud/licence` side in Phase 10 — until then dev mode returns success for
testing) → install as server: new DB key, new TLS cert, `server_epoch + 1`, relay
re-registration, every device `needs_rejoin` → Principal creates PIN → Home "Devices need
to join again" + "Invite all staff again" (bulk re-invite). Re-joined devices push their
unsent ops (same op_id/HLC). Restore staging: restore into a temp DB, verify, then swap
atomically; the previous DB is kept as a safety copy.

## PART D — NEW SESSION WIZARD (Settings → Session & terms)
Preview table: promotion Nursery→LKG→UKG→I→…→XII→"Passed out" (status left, reason
"Passed out"); per-student override (repeat class / leave); sections kept; unpaid dues →
one "Previous balance" due per student in the new Term 1 (linked to old dues, never
deleting them); new terms; new fee dues from heads; old session becomes read-only.
Commit in one transaction with an audit entry. Test with 1,500 students.

## PART E — SETTINGS (all sections, Principal)
School (name, address, board, UDISE, logo PNG/SVG ≤ 1 MB — shown on receipts/report
cards), Session & terms (+ wizard), Classes & subjects (incl. class teacher and
class-subject teacher assignment), Grade scale, Attendance cut-off time, Low-attendance
threshold, Appearance (accent swatches `#2F7479 / #1F4E8C / #5B4B8A`), Language (UI) +
Report language, Printing (A5 / 80 mm receipts), Security (auto-lock 1/5/15 min), Google
Drive, Connection (public address), Licence (plan, limits, status, "Check now"), **Storage**
(app size vs school data vs backups vs logs, in MB; "Clear temporary files"), About
(version, device id, audit chain head, licence id).

## PART F — HINDI
Translate every key naturally (simple Hindi used in Indian schools; keep ₹, receipt and
admission numbers, class names like VII-B, the product name "Vidya"). `:root[lang="hi"]`
swaps Newsreader headings to Noto Sans Devanagari 500 at the same size, no italics for
Devanagari; fixed 164px badges become `min-width: 164px` so Hindi never truncates. Hindi
logo lockups when language = Hindi. Receipts, report cards and reports in the chosen report
language. `scripts/check-i18n.mjs`: identical key sets, no `TODO-HI`, no empty values.
List any phrase you are unsure of for owner review.

## END-TO-END FLOWS (run and record)
1. Phone attendance → correction request → returned → resent → approved → register +
   teacher inbox.
2. Exam → phone marks per subject → submit → Principal prints class report cards.
3. 06:00 backup (debug time override) → verified locally and on Drive → restore on a
   second computer from Drive → staff re-invited → phones re-join → sync resumes → old PC
   shows "no longer the school server".
4. Session rollover with carry-forward.
5. Flow 1 of Phase 7 (admission → receipt) entirely in Hindi.

## VERIFICATION
Rust tests: retention selection, restore validation, promotion, carry-forward, backup
verify failure paths. Playwright phone screens at 390×844 and 360×800 (+130% font scale).
Screenshots in `docs/phase-notes/phase-8-screens/`. `grep -r "Coming in a later phase" src`
→ nothing. i18n check green.

## HANDOFF → `docs/phase-notes/phase-8.md`
Screens · backup/restore design as built · flows 1–5 results · translation notes + phrases
for review · known issues · questions.
