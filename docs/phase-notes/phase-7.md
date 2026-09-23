# Phase 7 handoff — desktop screens, printing, receipts, day book, CSV, attendance register

Branch: `rebuild/p07` (from `rebuild/p06`). Start HEAD: `2c85808` (P06 close-out).
`git status` at start: clean. Tools: rustc/cargo 1.98.1, node v25.3.0, npm 11.7.0.

## Status summary

Phase 7 is a very large phase (≈20 desktop screens + a printing subsystem + a
report-card/receipt render layer + 7 report types + CSV + a large test suite).
This session built the **foundation + the whole finance/students/attendance path
+ the shared/accountant screens**, all tested and committed in green increments.
The remaining heavy academics block (**exams / marks entry / grade scale / report
cards**) and the **full multi-report Reports** screen are **not built** — they are
the largest remaining work and are documented under "What's left" with the schema
already present and a concrete plan. This matches the repo's multi-session phase
workflow.

**Done (build order):** AppShell foundation → Students (list/profile/admission/
transfer/leave) → CSV import/export → Fees (overview/structure/ledger) → Receipts
+ printing + WhatsApp → Day book → Attendance register (day + month) → Accountant
home + My requests + Inbox + global search + session switcher.

**Not built this session:** Exams + marks entry + status grid + per-subject lock;
Grade scale editor; Report cards (A4); Reports (daily/monthly attendance, exam
results, fee collection, dues/defaulters, admissions by month, audit-log viewer).

### Gate (all green)
- `cargo test --workspace` → **416 pass, 0 fail** (vidya lib 132 · drive_e2e 8 ·
  e2e_flows 4 · relay_e2e 5 · sync_e2e 13 · vidya-core 243 · no_floats 1 · doc 10;
  benchmark `#[ignore]`). +10 new vidya-lib tests this phase.
- `cargo clippy -p vidya --all-targets -- -D warnings` → **clean**.
- `npx tsc --noEmit` → clean. `npx vitest run` → **38 pass** (api ↔ commands.json
  consistency included). `npm run build` → OK (JS 388.86 kB / 95.24 kB gzip, CSS 16.81 kB).
- `node scripts/check-hex.mjs` → **OK** (new gate; every hex in shipped `src/`,
  excluding dev-only `src/dev/**`, is a `tokens.css` value — 42 tokens).
- `grep -r "Coming in a later phase" src/screens/desktop` → **nothing**.

### Command surface: 51 → **75** commands (24 new; consistency test green across
`commands.json` ↔ Rust `COMMANDS` ↔ `api.ts`).

## Screens (route · roles · mock pattern derived from, docs §6.2)

| Screen | Route(s) | Roles | Derived from |
|---|---|---|---|
| **AppShell** (chrome) | wraps all desktop routes | P/A | Main.dc.html sidebar (P), FeeCollection.dc.html flat list (A) + header |
| Students list | `/{role}/students` | P/A | Collect-fee table (10px headers, 14/24 rows, pills) |
| Student profile | `/{role}/students/:id` | P/A | PageTitle + cards; Sheet dialogs for transfer/leave |
| New admission | `/{role}/students/new` | P/A | Collect-fee Sheet (520px, footer band) |
| CSV import | `/{role}/students/import` | P/A | table + PageTitle |
| Fees (overview/structure) | `/principal/fees` | P | StatStrip/table + Sheet editor |
| Receipts | `/{role}/receipts` | P/A | Approvals list+detail pattern |
| Receipt (print) | `/print/receipt/:id` | P/A | A5 / 80 mm print doc |
| Day book | `/{role}/daybook` | P/A | StatStrip + table |
| Day book (print) | `/print/daybook?date=` | P/A | A4 print doc |
| Attendance register | `/principal/attendance` | P | AttendanceRow buttons (day) + matrix (month) |
| Accountant home | `/accountant/home` | A | Principal-Home stat + card pattern |
| My requests | `/{role}/requests` | P/A | list + pills |
| Inbox | `/inbox` | P/A | list |
| Global search (Cmd/Ctrl+K) | `/search` | P/A | command palette overlay |
| Session switcher | `/session` | P/A | card + gold read-only banner |

Existing P03/P04 screens (Approvals, Staff & access, Conflict review, Sync &
devices, Principal Home, Collect fee) now render **inside AppShell** with real
navigation. The two full mock screens gained a `chrome` prop: the fidelity
gallery renders `chrome=true` (pixel-unchanged); the real app renders
`chrome=false` and AppShell owns the sidebar/header. **Fidelity baselines are
therefore unchanged** (the standalone mock components are byte-identical).

## New commands + rules (business rules in vidya-core; commands thin)

- `get_school` — school name/address/board/phone + current session label +
  `read_only` (shell, receipt/report headers, session banner).
- `list_students_page` — filters (class/section/status) + FTS + real LIMIT/OFFSET
  + total count (no silent caps).
- `get_student_profile` — details + enrollment history + attendance % for the
  **current term** (actual date range), via `vidya_core::attendance::percent_present`.
- `create_student` (enriched) — full fields; **provisional number offline**
  (`vidya_core::admissions::provisional_no`) / **official `YYYY/NNNN` in server
  mode** (`next_admission_no`); **dues generated** from active heads via
  `vidya_core::fees::generate_dues`; audited (`with_write`).
- `check_duplicate_students` — same guardian mobile OR same DOB + first-name FTS.
- `transfer_student` — closes the open enrollment, opens a new one (history intact).
- `mark_student_left` — status=left + reason; **cancels future UNPAID dues**
  (`cancelled_at`); **paid dues untouched**.
- `export_csv(kind, path, arg?)` — students / dues / daybook; scoped by permission;
  every cell via `vidya_core::csv::escape_formula`; UTF-8 BOM; 0 rows → header-only;
  audited. `students_csv_template(path)`.
- `import_students_dry_run` — per-row validation (column + reason, vidya-core) +
  duplicate candidates; max 5,000 rows / 5 MB. `import_students_commit` — valid
  rows + enrollment + dues in **ONE transaction**, one summary audit entry.
- `fees_overview` — students-with-dues + outstanding ₹ per class.
- `list/create/update/deactivate_fee_head`, `preview_fee_head_change` —
  Principal-only, audited; a head with allocations is **deactivate-only**; an
  amount change updates **UNPAID dues only** (paid never change), previewed first.
- `get_receipt` — heads from allocations, advance credit, **figures + words en&hi**
  (`vidya_core::words`), balance after, collected by, sync state, reversed flag.
  `search_receipts`. `reverse_payment` (Principal) — opens a `payment_reversal`
  request and approves it → request + decision + reversal row (all audited).
- `print_page` — Tauri 2 `WebviewWindow::print` (verified present in tauri 2.11.5);
  frontend falls back to `window.print()`.
- `day_book(date)` — totals by mode + reversals + **provisional (unsent) shown
  separately** + chronological entries.
- `attendance_month(class_id, month)` — days × students matrix + per-student P/A/L
  and % + per-day totals. `correct_attendance` — Principal direct correction of a
  submitted sheet (EditSubmittedAttendance, audited + op); teachers/accountants forbidden.

New Rust tests (10, in `commands::logic::tests`): students pagination/FTS;
admission dues + official number; offline provisional; transfer opens exactly one
new enrollment; leave cancels unpaid but keeps paid; CSV export BOM; CSV dry-run
flags an unknown-class row then commit imports valid only (with dues); fee-head
amount change touches unpaid only (paid untouched) + deactivate + accountant
forbidden; receipt figures+words + Principal reverse creates one reversal;
attendance month totals + Principal correction + accountant forbidden.

## PRINTING (results per OS)

- **Hidden print routes** render only the document with `@page` CSS: receipt A5 /
  80 mm thermal (`/print/receipt/:id?size=&duplicate=&auto=`), day book A4
  (`/print/daybook?date=`). Reprints are marked "Duplicate copy". Sync footnote
  ("Recorded on this computer — waiting for school server") + "Computer-generated
  receipt"; no invented legal text.
- **Desktop mechanism**: `WebviewWindow::print()` — **verified present** in the
  installed `tauri 2.11.5` (`src/webview/mod.rs:1485`, `webview_window.rs:2306`);
  wired via the `print_page` command with a `window.print()` fallback. "Printed"
  means only that the print dialog opened (§7).
- **NOT verified at runtime** (environment: tauri-driver is Linux/Windows-only, no
  Windows here): the actual print dialog on Windows/macOS. Verify on both OSes.
- **Android `PrintManager` plugin: NOT built** — there is no `src-tauri/plugins/`
  dir (the plugin was never scaffolded; P04 deferred the Keystore plugin too).
  Scaffolding needs the tauri CLI + Java/NDK/device, none of which are available
  here (see P06 notes). Android printing (P8) still needs this plugin.
- **"Save as HTML" fallback** (for no-printer error) is **not implemented** — a
  small `save_text_file` command + wiring is the follow-up; the print
  dialog/`window.print()` path covers the normal case.

## CSV formats

- **Student import template / accepted columns** (fixed order): `Name, Class,
  Roll, Guardian, Guardian mobile, Date of birth, Gender, Transport, RTE,
  Category, Aadhaar status`. UTF-8 BOM, one English example row. Class matched by
  its `display` (e.g. `I-A`). Transport/RTE accept yes/y/true/1.
- **Exports**: students (`Name, Admission no., Class, Roll, Guardian, Guardian
  mobile, Date of birth, Gender, Status, Outstanding (Rs)`), dues (`Name, Class,
  Outstanding (Rs)`), daybook (`Time, Receipt, Student, Class, Mode, Reference,
  Amount (Rs), Collected by, Status`). Amounts exported as whole rupees; "Rs" used
  in export headers to avoid a ₹ glyph in raw CSV. Every cell formula-escaped.

## WhatsApp share

`lib/files.ts shareWhatsApp` → `https://wa.me/91<mobile>?text=<urlencoded>` via
`tauri-plugin-opener` `openUrl` (browser `window.open` fallback). No mobile → opens
WhatsApp without a number. UI says it shares text, not a file. Message includes
receipt no. + amount + student + class + balance after.

## [OWNER] defaults used (please confirm)

- **Admission dues scheduling**: on admission, dues are generated for **each term
  of the current session** (term heads), the **current month** (month heads) and
  **once** (once heads). Full monthly-across-the-year scheduling is out of scope
  here (a scheduler/rollover concern) — confirm this is the intended admission
  behaviour.
- **Leave cancels FUTURE dues** = every dues row with **no allocation** (fully
  unpaid); paid/partly-paid dues are kept. (There is no per-due date, so "future"
  is interpreted as "unpaid".) Confirm.
- **Fee structure is Principal-only** (accountant is forbidden). The mock/permission
  table has no explicit "manage fee structure" action; gated on role.
- **Receipt "balance after"** = the student's **current** outstanding (no historical
  snapshot exists); confirm this is acceptable vs. a point-in-time balance.
- **Reports nav**: the Main.dc.html sidebar has **no "Reports" item** (nor
  "Receipts" for the Principal). Receipts is reached via Fees/Day book context and
  the global search; Reports will need a home (Settings sub-section, or a nav item
  the owner approves). **STOP/ask**: where should Reports live in the Principal
  sidebar, given the mock doesn't list it?
- Kept prior defaults (attendance % = P÷(P+A+L); grade scale; lease 30 d; accent
  `#2F7479`; `[Company name]` placeholders).

## Known issues / gaps

1. **Not built this session (largest remaining):** Exams (list + status grid) +
   marks entry grid (Tab/Enter, AB toggle, 0–max, per-subject submit lock) + Grade
   scale editor + Report cards (A4, per student + class batch, Incomplete banner).
   The schema is present (`exam`, `exam_subject`, `marks_sheet`, `mark_entry`,
   `grade_scale`, `grade_band`) and vidya-core has `marks` + `grades` +
   `words`; this is UI + thin commands + a report-card render layer (reuse the
   `/print/*` route pattern + `get_student_profile` attendance).
2. **Reports** (daily/monthly attendance, exam results, fee collection by range,
   dues/defaulters, admissions by month, **audit-log viewer** with chain status &
   "teachers never see others' entries") — not built. Much of the data already
   exists (`attendance_month`, `day_book`, `fees_overview`, `verify_audit_chain`);
   needs a Reports hub + printable A4 renders + an `list_audit` read.
3. **Runtime/pixel verification of the new screens** was not done — a Tauri-driven
   Playwright harness (tauri-driver = Linux/Windows) or a real window is needed;
   the browser `invoke` is unavailable, so data-driven screens show loading in a
   plain browser. The new screens are **tsc/build-verified + their rules are
   Rust-tested**. Playwright screenshots per screen at 1440×900 / 1280×720 (into
   `docs/phase-notes/phase-7-screens/`) are a follow-up on a Tauri-capable OS.
4. **Android print plugin** + **"Save as HTML"** fallback (see PRINTING).
5. **Inbox** surfaces decided-requests + review-flags/conflicts (pragmatic); the
   dedicated `notification` feed isn't emitted into yet (no writer). A
   `list_notifications`/`mark_read` pair + emit-sites are the follow-up.
6. **Session switcher** shows the current session + read-only banner; multi-session
   switching needs Phase 8 rollover (only one session exists until then). The
   `SESSION_READ_ONLY` write-block is enforced on the session row + surfaced by the
   banner; blocking *writes* app-wide on a read-only session is a follow-up guard.
7. **`student_details` direct Edit** (Principal, audited) on the profile is not
   wired (only Transfer + Leave). Accountant edits already route through the
   `student_details` request (P03 Approvals). Needs an `update_student` command +
   edit form.
8. **Fee structure sync**: `fee_head` has no sync columns in `0001_init` (rule 9:
   don't edit earlier migrations), so structure edits are local + audited but not
   synced. Multi-device fee-structure sync needs a new migration (P8/later).

## Questions for the owner

1. **Reports placement** (see [OWNER] above) — the mock sidebar has no Reports
   item. Where should it live?
2. Confirm the admission-dues scheduling + leave "unpaid = future" + receipt
   "balance after = current outstanding" interpretations.
3. OK to proceed next session with the **Exams/Marks/Grade-scale/Report-cards**
   block, then **Reports** (audit viewer first, it's self-contained)?
