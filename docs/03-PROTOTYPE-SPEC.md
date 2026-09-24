# VIDYA — PROTOTYPE SPEC (design reference for every new screen)

`design/prototype/VidyaPrototype.jsx` is a single React file that renders every screen of the
v2 product in the approved design. It is a **visual and behavioural reference only**: it has
no real data, no rules and no storage. Never import it into the app.

## 1. How to open it
1. Create a throwaway Vite React app outside `src/` (e.g. `design/prototype/viewer/`, dev-only,
   excluded from builds), or reuse the dev-only mock server from Phase 1.
2. Load the fonts the app already bundles (Geist, Newsreader variable with optical sizes, Noto
   Sans Devanagari, Noto Sans Telugu) and set `window.VIDYA_ASSET_BASE` to `design/assets`.
3. Render the default export: a gallery with every screen and its states.
4. For screenshots: open with `?bare=1`, then call `window.__vidyaShow('<id>', <stateIndex>)`.
   The screen renders at 1:1 at the top-left (desktop 1440×1080 or 1440×960, phone 390×844).
   Pause/finish CSS animations before capturing (see `tests/e2e` from Phase 1).

## 2. Rules
- Tokens, fonts, sizes, radii, shadows, animations and copy in the prototype come from the
  mock. **Match them exactly** using the app's existing components.
- Every `data-hl="…"` attribute marks an element that matters (used for highlights in demo
  videos). Keep the same structure so those regions exist in the app.
- Sample names and numbers are fixtures: the demo seed should reproduce them so screens can be
  compared side by side.
- Telugu and Hindi strings in the prototype are samples pending native review.
- The QR code drawn by `FakeQR` is decorative. The app renders real QR codes.
- The share sheet's green circle is a generic icon; do not use WhatsApp's logo.

## 3. Screen map (prototype → app)

| Prototype id | States | App route / place | Roles | Phase |
|---|---|---|---|---|
| `welcome` | setup / join / recover | `#/welcome` (+ machine code for setup, §7.1 of v2 changes) | all | P12 |
| `activate` | entered / checking / active | becomes **Enter licence key / load .vlic** (same layout; "checking" = offline signature check) | Principal | P12 |
| `setup` | school / session / you / recovery / ready | setup wizard (+ UPI step or link to Settings → Payments) | Principal | exists; P14 adds UPI |
| `pin` | entering | PIN unlock | all | exists |
| `home` | today | Principal Home (cut-off copy removed) | Principal | P11 |
| `staff` | list / add / invitation | Staff & access (+ "change sync password" hint after removal) | Principal | exists; P12 |
| `approvals` | review / approved | Approvals | Principal | exists |
| `leave` | review / approved | Approvals → leave request detail | Principal | P17 |
| `settings` | UPI / Google Drive / languages & modules | Settings → Payments, Google Drive, Languages & modules | Principal | P11–P14 |
| `feesadmin` | instalments / dues / reminder | Fees → Fee structure, Dues, reminder sheet | Principal, Accountant | P14–P15 |
| `fee` | enter / too much / recorded | Collect fee (mock) | Accountant | exists |
| `receipt` | A5 preview (+ UPI QR block) | Receipt print route | Accountant | P14 |
| `accounts` | cash book / expense / profit | Accounts → Cash book, Record expense, Profit summary | Principal (+ Accountant: cash book, expenses) | P15 |
| `salary` | September | Accounts → Salary register | Principal | P15/P17 |
| `timetable` | week / substitute / assigned | Timetable, Substitutes sheet | Principal | P16 |
| `exams` | seating / hall tickets | Marks & exams → Seating, Hall tickets print | Principal | P16 |
| `reportcard` | print | Report card print route (+ remarks) | Principal, Teacher | P16 |
| `calendar` | September | Calendar | all (edit: Principal) | P13 (data) / P16 (screen) |
| `circulars` | write / sent | Circulars | Principal (Teacher drafts class notices) | P14 |
| `backups` | verified | Backups | Principal | exists (copy updated) |
| `store` | sale | School store (optional module) | Accountant, Principal | P15 |
| `join` | code / confirm / pin / syncing | Join flow (phone) — Principal signs the phone into the sync account during "confirm" | Teacher, Accountant | P12 |
| `thome` / `thomete` | today / Telugu | Teacher Home (en / hi / te) | Teacher | P13 |
| `attendance` | start → submitted | Attendance (P/A only) | Teacher | P11 |
| `absence` | list / sent | After attendance submit → Absence alerts | Teacher | P14 |
| `marks` | entering / complete / submitted | Marks (phone) | Teacher | exists |
| `correction` | form / sent | Correction request | Teacher | exists |
| `requests` | waiting / approved | My requests | all | exists |
| `notes` | write / share | Homework & notes (+ Android share sheet) | Teacher | P16 |
| `staffday` | checked in / leave form / leave sent / circular | My attendance, Apply for leave, Inbox (circular) | all staff | P14, P17 |

## 4. Fidelity check for new screens
Add `tests/e2e/prototype-fidelity.spec.ts`: for each row above that a phase builds, render the
prototype state (bare mode) and the app route with the demo seed at the same viewport, wait for
animations, compare with `toHaveScreenshot` (max diff 1% for new screens — dynamic data like
dates is pinned by the seed). Differences caused by real data the prototype fakes (e.g. real QR
codes) are masked with Playwright's `mask` option, listed in the handoff.
