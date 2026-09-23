# Phase 7 handoff — all desktop screens, printing, receipts, report cards, reports, CSV

Branch: `rebuild/p07` (from `rebuild/p06`). Start HEAD: `2c85808` (P06 close-out).
`git status` at start: clean. Tools: rustc/cargo 1.98.1, node v25.3.0, npm 11.7.0.

## Status summary

**Every screen in the build order is built** — Principal + Accountant + shared —
on real data, with printing, receipts, report cards, reports and CSV. The whole
BUILD ORDER is done:

AppShell foundation → Students (list / profile / admission / transfer / leave) →
CSV import + export → Fees (overview / structure / ledger) → Receipts + printing
+ WhatsApp → Day book → Attendance register (day + month) → Exams + marks entry +
status grid + per-subject lock → Grade scale + report cards → Reports (audit /
admissions / fee collection / dues / exam results / attendance) → Accountant home
+ Students & admissions + Receipts + Day book + My requests → Inbox + global
search + session switcher.

`grep -r "Coming in a later phase" src/screens/desktop` → **nothing**.

### Gate (all green)
- `cargo test --workspace` → **420 pass, 0 fail** (vidya lib 136 · drive_e2e 8 ·
  e2e_flows 4 · relay_e2e 5 · sync_e2e 13 · vidya-core 243 · no_floats 1 · doc 10;
  benchmark `#[ignore]`). +14 new vidya-lib tests this phase.
- `cargo clippy -p vidya --all-targets -- -D warnings` → **clean**.
- `npx tsc --noEmit` → clean. `npx vitest run` → **38 pass** (api ↔ commands.json
  consistency included). `npm run build` → OK (JS 427 kB / 102 kB gzip, CSS 16.9 kB).
- `node scripts/check-hex.mjs` → **OK** (new gate: every hex in shipped `src/`,
  excluding dev-only `src/dev/**`, is a `tokens.css` value — 42 tokens).

### Command surface: 51 → **89** (+38; consistency test green across
`commands.json` ↔ Rust `COMMANDS` ↔ `api.ts`).

## Screens (route · roles · mock pattern derived from, docs §6.2)

| Screen | Route(s) | Roles | Derived from |
|---|---|---|---|
| **AppShell** (chrome) | wraps all desktop routes | P/A | Main.dc.html sidebar (P), FeeCollection.dc.html list (A) + header |
| Students list | `/{role}/students` | P/A | Collect-fee table |
| Student profile | `/{role}/students/:id` | P/A | PageTitle + cards; transfer/leave dialogs |
| New admission | `/{role}/students/new` | P/A | Collect-fee Sheet |
| CSV import | `/{role}/students/import` | P/A | table + PageTitle |
| Fees (overview/structure) | `/principal/fees` | P | StatStrip/table + Sheet editor |
| Receipts | `/{role}/receipts` | P/A | Approvals list+detail |
| Day book | `/{role}/daybook` | P/A | StatStrip + table |
| Attendance register | `/principal/attendance` | P | AttendanceRow (day) + matrix (month) |
| Marks & reports (exams/status grid/entry) | `/principal/marks` | P | table + status pills + entry grid |
| Grade scale | `/principal/grade-scale` | P | table editor (Settings) |
| Reports | `/principal/reports` | P | tabbed hub + tables/CSS bars |
| Accountant home | `/accountant/home` | A | Principal-Home stat/card pattern |
| My requests | `/{role}/requests` | P/A | list + pills |
| Inbox | `/inbox` | P/A | list |
| Global search (⌘/Ctrl+K) | `/search` | P/A | command palette overlay |
| Session switcher | `/session` | P/A | card + gold read-only banner |
| **Print routes** | `/print/receipt/:id` (A5/80mm), `/print/daybook`, `/print/reportcard/:id`, `/print/reportcards/:classId` (A4) | P/A | @page print docs |

The P03/P04 screens (Approvals, Staff & access, Conflict review, Sync & devices,
Principal Home, Collect fee) render **inside AppShell** with real navigation. The
two full mock screens gained a `chrome` prop: the fidelity gallery renders
`chrome=true` (pixel-unchanged), the app renders `chrome=false`. **Fidelity
baselines are unchanged.**

## New commands + rules (rules in vidya-core; commands thin, audited)

- **Shell/identity:** `get_school`.
- **Students:** `list_students_page` (filters + FTS + real pagination + total),
  `get_student_profile` (details + enrollment history + term attendance %),
  `create_student` (full fields, provisional offline / official `YYYY/NNNN` in
  server mode, dues via `fees::generate_dues`), `check_duplicate_students`,
  `transfer_student` (close+open enrollment), `mark_student_left` (cancels future
  UNPAID dues; paid untouched).
- **CSV:** `export_csv` (students/dues/daybook; `escape_formula`; BOM; 0→header),
  `students_csv_template`, `import_students_dry_run`, `import_students_commit`
  (valid rows + enrollment + dues in ONE transaction).
- **Fees:** `fees_overview`, `list/create/update/deactivate_fee_head`,
  `preview_fee_head_change` (amount change → UNPAID dues only; paid never change;
  in-use head is deactivate-only). Principal-only.
- **Receipts:** `get_receipt` (heads from allocations, advance credit, figures +
  words en/hi, balance after, sync state, reversed), `search_receipts`,
  `reverse_payment` (Principal → request + decision + reversal), `print_page`
  (Tauri 2 `WebviewWindow::print`, verified present).
- **Day book:** `day_book` (mode totals + reversals + provisional split + list).
- **Attendance:** `attendance_month` (matrix + %), `correct_attendance`
  (Principal direct, audited; others forbidden).
- **Academics:** `list_class_subjects`, `list_exams`, `create_exam` (Principal),
  `get_marks_sheet`, `save_marks_draft`, `submit_marks` (0..=max validation,
  submit locks that subject only; teacher own class-subject / Principal),
  `list_grade_bands`, `update_grade_bands` (contiguous 0..=100, no gaps/overlaps),
  `get_report_card` (subjects × marks + total/%/grade via `grades::grade_report`;
  NULL → Incomplete; absent excluded), `class_student_ids`.
- **Reports:** `list_audit` (filters + pagination + chain status; teachers own
  entries only), `admissions_by_month`, `fee_collection_report(from,to)`,
  `exam_results(exam)` (subject averages + grade distribution).

New Rust tests (14, `commands::logic::tests`): students pagination/FTS; admission
dues + official/provisional number; transfer opens one enrollment; leave cancels
unpaid keeps paid; CSV export BOM + dry-run/commit valid-only; fee-head amount
change unpaid-only + deactivate + accountant-forbidden; receipt words + Principal
reverse; attendance month + correction + accountant-forbidden; marks range +
submit-lock; report card grades + Incomplete on NULL; grade bands reject gaps;
reports audit scoping (teacher < principal) + chain-ok + exam averages.

## PRINTING (results per OS)

- **Hidden print routes** render only the document with `@page` CSS: receipt
  A5 / 80 mm (`/print/receipt/:id?size=&duplicate=&auto=`), day book A4, report
  cards A4 (single or whole-class batch, one student per page via
  `page-break-after`). Reprints marked "Duplicate copy"; sync footnote; no
  invented legal text.
- **Report screens** print in-place: AppShell chrome + in-screen controls carry
  `data-appshell-chrome`, hidden by `@media print` in `app.css`.
- **Desktop mechanism**: `WebviewWindow::print()` — **verified present** in the
  installed `tauri 2.11.5` (`src/webview/mod.rs:1485`); `print_page` command +
  `window.print()` fallback. "Printed" = the dialog opened only (§7).
- **NOT verified at runtime** (tauri-driver = Linux/Windows-only; no Windows
  here): the actual print dialog on Windows/macOS — verify on both.
- **Android `PrintManager` plugin: NOT built** — no `src-tauri/plugins/` dir;
  scaffolding needs the tauri CLI + Java/NDK/device (unavailable, see P06). Android
  printing (P8) needs this plugin.
- **"Save as HTML"** no-printer fallback: not implemented (a small
  `save_text_file` command + wiring). The dialog/`window.print()` path covers the
  normal case.

## CSV formats

- **Import template / columns** (fixed order): `Name, Class, Roll, Guardian,
  Guardian mobile, Date of birth, Gender, Transport, RTE, Category, Aadhaar
  status`. UTF-8 BOM + one English example. Class matched by `display` (e.g. `I-A`).
- **Exports**: students, dues, daybook (headers in the code). Amounts whole
  rupees; "Rs" in export headers (avoid a ₹ glyph in raw CSV). Every cell
  formula-escaped.

## WhatsApp share

`lib/files.ts shareWhatsApp` → `https://wa.me/91<mobile>?text=<urlencoded>` via
`tauri-plugin-opener`. No mobile → WhatsApp without a number. UI says it shares
text, not a file.

## [OWNER] defaults used (please confirm)

- **Admission dues**: generated for each **term of the current session** (term
  heads), the **current month** (month heads) and **once** (once heads). Full
  monthly-across-the-year scheduling is out of scope (a scheduler/rollover concern).
- **Leave cancels FUTURE dues** = every dues row with **no allocation** (fully
  unpaid); paid/partly-paid dues kept (no per-due date exists).
- **Fee structure Principal-only**; **grade scale Principal-only**.
- **Receipt "balance after"** = the student's **current** outstanding (no
  historical snapshot).
- **Reports lives under Academics** in the Principal sidebar — `Main.dc.html`
  has **no Reports item** (nor Receipts for the Principal; Receipts is reached via
  the global search / Fees / Day book). Confirm the placement or tell me where
  Reports should sit; this was the one flagged sidebar gap and I took the smallest
  faithful choice.
- Kept prior defaults (attendance % = P÷(P+A+L); default grade scale seeded;
  low-attendance 75% [OWNER] shown as the report note; lease 30 d; accent
  `#2F7479`; `[Company name]` placeholders).
- Demo seed gained (additive): the default grade scale + one **Half-Yearly exam**
  (VI-B Maths) with marks (one absent, one not-entered) so exams/marks/report
  cards have data.

## Known issues / gaps (all documented, none faked)

1. **Runtime/pixel verification** of the new screens was not done — a Tauri-driven
   Playwright harness (Linux/Windows) or a real window is needed; the browser
   `invoke` is unavailable, so data-driven screens show loading in a plain
   browser. Screens are **tsc/build-verified**; their **rules are Rust-tested**.
   Playwright screenshots per screen at 1440×900 / 1280×720 (into
   `docs/phase-notes/phase-7-screens/`) are a follow-up on a Tauri-capable OS.
2. **Android print plugin** + **"Save as HTML"** fallback (see PRINTING).
3. **Inbox** surfaces decided-requests + review-flags/conflicts (pragmatic); the
   dedicated `notification` feed isn't emitted into yet — a `list_notifications`
   /`mark_read` pair + emit-sites are a follow-up.
4. **Session switcher** shows the current session + read-only banner; multi-session
   switching needs Phase-8 rollover (only one session exists until then). Blocking
   *writes* app-wide on a read-only session (`SESSION_READ_ONLY`) is enforced on
   the row + surfaced by the banner; an app-wide write guard is a follow-up.
5. **`student_details` direct Edit** on the profile (Principal, audited) is not
   wired (only Transfer + Leave). Accountant edits already route through the
   `student_details` request (P03 Approvals). Needs an `update_student` command +
   edit form.
6. **Fee structure / marks / grade-scale sync**: `fee_head`, `grade_scale`,
   `grade_band` have no sync columns in `0001_init` (rule 9: don't edit earlier
   migrations), so those edits are local + audited but not synced across devices.
   Multi-device sync needs a new migration (P8/later). Marks/attendance corrections
   DO emit ops.
7. **Report card** shows a single exam's subjects (the schema supports subjects ×
   multiple exams; a multi-exam card is a small extension). Class-teacher remark is
   a blank signed line (not persisted); the prompt's "typed" remark can be added
   with a `report_remark` store later.

## Questions for the owner

1. **Reports placement** — confirm "Reports under Academics" or move it.
2. Confirm the admission-dues scheduling + leave "unpaid = future" + receipt
   "balance after = current outstanding" interpretations.
3. Prioritise the follow-ups: Android print plugin, notification feed, session
   rollover (P8), `student_details` direct edit, and the Tauri-window Playwright
   screenshot pass.
