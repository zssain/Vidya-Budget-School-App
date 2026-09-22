# PHASE 7 of 10 — ALL REMAINING DESKTOP SCREENS, PRINTING, RECEIPTS, REPORT CARDS, REPORTS, CSV

## ROLE
You are a senior full-stack Tauri/React/Rust engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`. Reopen the mock files for every new screen.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies. No PDF, chart, date, form or spreadsheet libraries.
   Anything else → STOP and ask.
4. Never invent features, rules, copy, numbers, legal wording, API fields or library APIs.
   Unsure → STOP and ask. Not in scope anywhere: SMS, online parent payments, fines,
   concessions, discounts, scholarships, timetable, library, transport routes, payroll.
5. **The mock wins.** New screens use ONLY existing components and tokens (context §6.2
   derivation rules). No new colours, sizes, radii or shadows. A gap → STOP and ask with
   the smallest proposal.
6. Business rules only in vidya-core. Commands stay thin.
7. No fake success: "printed" only means the print dialog opened; exports only report
   success after the file is written.
8. Every visible string via `t()` in both `en.json` and `hi.json`.
9. Never delete/weaken tests. Never edit an earlier phase's migration.
10. Run every command you mention; paste real output. Work on branch `rebuild/p07`.
11. Stop conditions are real. Finish with the handoff file.

## OBJECTIVE
Every Principal and Accountant desktop screen works on real data with printing, receipts,
report cards, reports and CSV import/export. No "Coming in a later phase" left on desktop.

## BUILD ORDER
Students → admission / transfer / leave → CSV import/export → fees overview + structure +
ledger → receipts + printing + WhatsApp → day book → attendance register → exams + marks
entry + status grid → grade scale + report cards → reports → accountant home + accountant
screens → my requests + inbox + notifications → global search + session switcher.

## SCREENS AND EXACT BEHAVIOUR

### Principal (sidebar exactly as in `Main.dc.html`)
1. **Students** — table in the Collect-fee table pattern: Name + Adm. no. (or "Adm. no.
   pending"), Class, Roll, Guardian, status pill; filters: session, class, section, status
   (active/left); search (FTS); real pagination with total count (no silent limits).
   **Profile** (Sheet pattern or full page): details, enrollment history, attendance % this
   term, marks per exam, dues/payments/ledger (Principal + Accountant only), requests,
   audit history. Actions: Edit (Principal direct, audited with reason), Transfer section
   (closes enrollment, opens new; history intact), Mark as left (date + reason; FUTURE dues
   cancelled via `cancelled_at`, paid dues untouched).
2. **New admission** (Principal + Accountant) — Sheet form: name, DOB, gender, guardian
   name + mobile, address, class/section, roll (auto next, editable), transport, RTE,
   category (optional), Aadhaar status only (no number). Duplicate check (same name + DOB
   or same guardian mobile + similar name) shows candidates before saving. Offline → saved
   with provisional number (context §8.7). Dues generated per fee heads.
3. **CSV import** (Principal + Accountant) — "Download template" (CSV with header row, UTF-8
   BOM, one example row in the first language) → choose file → dry-run preview table:
   valid rows count, row-level errors (column + reason, using vidya-core validation),
   duplicate candidates → "Import N students" in ONE transaction → result file with any
   skipped rows. Max 5,000 rows / 5 MB; clear messages otherwise.
4. **CSV export** — students (scoped to the caller's permission; accountant export has no
   marks/attendance; teacher cannot export fee data), day book, dues, attendance register,
   exam results. Label buttons "Export CSV" (never "Excel"). Formula escaping via
   `vidya_core::csv::escape_formula`. Save via `tauri-plugin-dialog`; each export is
   audited.
5. **Attendance register** — class + date: desktop P/A/L grid (AttendanceRow buttons in a
   table layout); month view (days × students, same colours, totals per student and day,
   % with the owner-default formula); Principal direct corrections audited with reason.
6. **Marks & reports** — Exams list (name, term, dates, subjects, max marks per subject);
   status grid classes × subjects (Not started / Draft / Submitted pills); marks entry
   table (desktop): one row per student, Tab/Enter moves down, "AB" toggle, 0–max
   validation inline, Save draft / Submit subject (locks that subject only).
7. **Grade scale** (Settings) — edit bands with validation (no gaps/overlaps, 0–100).
8. **Report cards** — per student and whole class: logo lockup (English or Hindi per report
   language), school name, session, exam(s), student details, subjects × exams, total, %,
   grade, attendance % for the term (actual date range shown), class-teacher remark
   (typed, optional), "Principal" signature line. Incomplete marks → "Incomplete" banner
   instead of totals. A4, one student per page (CSS `page-break-after`). English or Hindi
   per school setting.
9. **Fees** — overview by class (students with dues, outstanding ₹); dues list filters
   (class, term, overdue only); student ledger (dues, allocations, payments, reversals,
   advance credit — running balance). **Fee structure**: heads CRUD (name, name_hi, amount,
   frequency, applies to); heads with allocations can only be deactivated; changing
   amounts shows a PREVIEW of affected unpaid dues before applying; paid dues never change.
10. **Receipts** — search by receipt no./student/date; open; reprint (marked "Duplicate
    copy"); request reversal (Accountant) / reverse (Principal, with reason — still creates
    the request + decision records for audit).
11. **Day book** — date picker; StatStrip: Cash / UPI / Cheque / Total / Reversals;
    chronological list (time, receipt, student, class, mode, reference last 4, amount,
    collected by, sync pill); provisional amounts shown separately ("waiting for server");
    print + export CSV.
12. **Reports** (role-based, printable A4, English/Hindi): daily attendance summary;
    monthly attendance by class/student (below 75% highlighted — **[OWNER default,
    setting]**); exam results by class (subject averages, grade distribution as CSS bars);
    fee collection by date range and mode; dues/defaulters by class; admissions by month;
    audit log viewer (filters staff/table/action/date, paginated, read-only, chain status;
    teachers never see others' entries).

### Accountant (sidebar exactly as in `FeeCollection.dc.html`)
Home (collected today, receipts today, waiting to send, my pending requests, recent
receipts) · Collect fee (mock) · Students & admissions (admit; edits → `student_details`
request) · Receipts · Day book · My requests · fee reports.

### Shared (desktop)
My requests (pills, timeline per request, returned → "Edit and resend" creates a new
revision, cancel while pending) · Inbox (decisions, rejected ops, conflict results,
Drive/relay notices; read/unread; deep links; teachers never receive financial details) ·
notifications bell (header) · global search Ctrl/Cmd+K (students, receipts, staff — scoped
by role) · session switcher in the header (past sessions read-only with gold banner "You
are viewing 2025–26 (read-only)"; all writes blocked with `SESSION_READ_ONLY`).

## PRINTING (exact)
- A hidden print route renders the document with print CSS (`@page` A4 / A5 / 80 mm).
- Desktop: use Tauri's webview print API (verify the exact method in the installed `tauri`
  crate docs, e.g. `Webview::print`); if unavailable on a platform, fall back to
  `window.print()` and test it on Windows AND macOS.
- Android: add `print(html)` to `plugins/vidya-android` using Android `PrintManager` +
  `WebView.createPrintDocumentAdapter` (Phase 8 uses it on phones; build it now).
- No printer / error → "Could not open printing on this computer." + "Save as HTML" via
  the dialog plugin.
- **Receipt** (A5 and 80 mm thermal, setting): logo, school name + address, "Fee receipt",
  receipt no., date/time, student, class, adm. no. (or provisional), heads paid (from
  allocations), advance credit if any, total in figures + words (vidya-core, receipt
  language), mode + reference, balance after, collected by, sync-state footnote
  ("Recorded on this computer — waiting for school server" when not confirmed),
  "Computer-generated receipt". No invented legal text.

## WHATSAPP SHARE
`tauri-plugin-opener` → `https://wa.me/91<guardian_mobile>?text=<urlencoded>` using en/hi
templates, e.g. "Saraswati Public School — Fee receipt R-A2-0419 · ₹1,000 received by UPI
for Kavya Singh (VI-B) on 23 Sep 2026. Balance due ₹2,100." No mobile → open WhatsApp
without a number. This shares text, not a file; say so in the UI help text.

## EDGE CASES
Long names → ellipsis in tables, full in profile · 60-student marks table keyboard-only ·
student with advance credit → next dues allocate from credit first (vidya-core rule; add
with test) · fee head change after payments → preview only · past session → writes blocked
· export with 0 rows → header-only file + message · import with Devanagari names ·
concurrent edit of the same student by Principal and Accountant → conflict flow.

## VERIFICATION
Rust tests for every new rule (words en/hi on receipts, allocation with credit, transfer,
leave cancels future dues, CSV import validation, report totals); command-level integration
tests for: admission → dues → partial payment → receipt → day book → dashboard; reversal
request → approval → dues restored → day book reversal; exam → marks → submit → report
cards. Playwright: each new screen at 1440×900 and 1280×720; screenshots in
`docs/phase-notes/phase-7-screens/`. `scripts/check-hex.mjs` (every hex in `src/` exists in
tokens.css) green. `grep -r "Coming in a later phase" src/screens/desktop` → nothing.

## HANDOFF → `docs/phase-notes/phase-7.md`
Screens (route, roles, mock pattern derived from) · new commands + rules · print results
per OS · CSV formats · [OWNER] defaults · known issues · questions.
