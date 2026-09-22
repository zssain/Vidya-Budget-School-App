# Vidya Budget School — Build Pack (start here)

This pack contains everything a coding agent needs to build Vidya exactly like the mock:

```
README-START-HERE.md            ← you are here
docs/00-SYSTEM-CONTEXT.md       ← source of truth (product, rules, stack, data, sync, security)
docs/01-MOCK-SPEC.md            ← every token, font, animation, asset, component of the mock
prompts/P01 … P10               ← one prompt per phase, in order
design/screens/*.dc.html        ← the 5 mock screens (the visual contract)
design/assets/*.svg             ← the 3 logo files the mock uses
design/brand-kit/               ← your full brand kit (icons for .exe/.dmg/.apk, guide)
design/reference/               ← put your exported mock screenshots here (see below)
```

## 1. How to run it

1. Copy `docs/` and `design/` into the root of your repository and commit them.
2. Open a fresh coding-agent session for each phase.
3. Paste this message, then the whole phase prompt file:

   > You are working in the Vidya repository. Before anything else, read
   > docs/00-SYSTEM-CONTEXT.md, docs/01-MOCK-SPEC.md and every file in docs/phase-notes/.
   > Then do exactly the phase below. Stop and ask me whenever the prompt says STOP.
   > [paste the phase prompt here]

4. When the agent finishes, read its `docs/phase-notes/phase-N.md` handoff. Check the
   DONE MEANS list yourself (try the app). Answer its questions. Only then start the next
   phase.
5. Never skip a phase. Phase 10 (company website + licence service) may run in parallel
   with Phases 4–8 in a separate session once Phase 3 is done.

| Phase | What you get at the end |
|---|---|
| P01 | App opens; the 5 mock screens look exactly like the mock; size budget proven |
| P02 | All business rules + encrypted database + audit chain (tested, no new UI) |
| P03 | Activation, setup wizard, PIN, recovery key; one PC runs a school on real data; Approvals |
| P04 | School server on Wi-Fi; staff join by invite; phones sync; conflicts; device control |
| P05 | Phones reach the school from anywhere (relay); old PCs fenced off |
| P06 | Google Drive fallback when the school PC is off (after two proof tests you approve) |
| P07 | Every desktop screen: students, import/export, register, marks, report cards, fees, receipts, day book, reports, printing |
| P08 | Every teacher phone screen, backups, restore to a new PC, new session, settings, storage, full Hindi |
| P09 | Security, speed, size gates, accessibility, signed installers from one git tag |
| P10 | Company website: purchase, activation codes, licence API, admin panel, downloads page |

## 2. Before Phase 1 — export reference screenshots (10 minutes)

Open the design canvas, press **Play** on each screen at 100% zoom, and save screenshots
into `design/reference/` with these names (exact mock size):

`welcome-setup.png`, `welcome-join.png`, `welcome-recover.png` (1440×960) ·
`principal-home.png` (1440×1080, wait 2 s for animations) ·
`collect-fee-default.png`, `collect-fee-over.png` (type 5000), `collect-fee-cash.png`,
`collect-fee-success.png` (1440×1080) · `teacher-home.png` (390×844) ·
`attendance-default.png`, `attendance-markall.png`, `attendance-submitted.png` (390×844).

The agent also builds an automatic mock renderer and compares pixels (≤ 0.1% difference).
Your screenshots are the human check.

## 3. Decisions only you can make (agents build a default and ask)

| # | Decision | Default until you decide | Needed by |
|---|---|---|---|
| 1 | Existing repo or fresh start | Keep the repo; old UI → `legacy/`; port tested Rust rules | P01 |
| 2 | Default grading scale / board | CBSE-style A1…E (editable) | P02 |
| 3 | Leave counts as absent in attendance %? | % = P ÷ (P+A+L) | P02 |
| 4 | Attendance by non-class-teachers? | Class teacher only | P02 |
| 5 | Relay secret issued by the licence service | Yes (contract change) | P05 |
| 6 | Relay hosting: provider, domain, monthly budget | Docker image, not deployed | P05 |
| 7 | Every staff member has a Google account? | Drive fallback off for those without | P06 |
| 8 | Drive scope result (after the spike) | — you choose from the report | P06 |
| 9 | Low-attendance threshold | 75% | P07 |
| 10 | Offline access lease | 30 days | P04 |
| 11 | Code signing: Windows cert, Apple Developer ID, Android keystore | Unsigned, clearly labelled | P09 |
| 12 | Payment provider + test keys, prices, GST/invoice, email for codes, admin 2FA | Placeholders, test mode | P10 |
| 13 | Company name, support contact, legal text | `[Company name]` placeholders | P07 / P10 |

## 4. What changed from the earlier prompts (and why)

- **Mock fidelity is enforced by a pixel test**, not just "match the mock": a small dev-only
  renderer shows the real mock files next to the app, and Playwright fails on > 0.1%
  difference.
- **No animation library**: the mock's animations are CSS keyframes, so the app copies
  them verbatim (exact match, lighter on phones). Every per-screen keyframe variant is
  listed in `01-MOCK-SPEC.md §5`.
- **Newsreader variable font with optical sizes** is bundled, because a static font would
  make the big headings look different from the mock.
- **Size proven in Phase 1** with every native library linked in; download ≤ 40 MB and
  installed ≤ 50 MB checked in CI.
- **Smaller dependencies**: `ring` only (no aws-lc-rs), no axum-server, trimmed tokio /
  reqwest / tracing features, lucide removed, QR scanner kept only if cheap, fonts subset.
- **Licence is one-time and perpetual** (no expiry or grace period), as agreed.
- **Money can never disappear in sync**: the server always accepts real payments and flags
  overpayments for review.
- **Offline admissions** get a provisional number; the school server issues the official one.
- **Drive is locked down**: backups are never shared; each staff member can write only to
  their own folder; per-class keys so teachers can't read other classes.
- **Requests** gained cancel, revisions, stale checks, "applied" state, access-change and
  device-replacement types.
- **Added**: offline access lease, server epoch fencing (old PC can't act as server),
  section transfer, CSV import/export with formula protection, storage screen, Android
  printing plugin, Android Google sign-in spike, company website/licence/admin phase.
- **Fixed**: platform detection (Tauri 2 has no `api/os`), asset names now match your
  brand kit, app icons taken from the brand kit instead of generated.

## 5. Feature map (every agreed feature → phase)

| Feature | Phase |
|---|---|
| Purchase, payment verification, activation codes, licence register, admin panel, downloads page | P10 (+ app side P03) |
| Setup wizard (school, session, classes, Principal, recovery key) | P03 |
| PIN unlock, auto-lock, user switch | P03 |
| Staff & access, invitations (link / code / QR), suspend/remove, device series | P04 |
| RBAC in core, commands, server, sync scopes, Drive keys, exports | P02, P04, P06, P07 |
| School server on LAN, over internet (relay), Drive fallback, offline queue | P04, P05, P06 |
| Exact save/sync status labels | P04–P06 |
| Conflict review + review flags (overpayment, duplicates, revoked authors) | P04 |
| Offline lease, revocation, server epoch fencing | P04, P05 |
| Students: list, profile, admission (offline provisional no.), transfer, leave | P07 |
| CSV import (dry run) + CSV export (scoped, formula-safe) | P07 |
| Attendance: phone (mock), desktop register, lock, corrections | P01/P03, P07, P08 |
| Exams, marks (desktop + phone), per-subject lock, corrections | P07, P08 |
| Grade scale, report cards (A4, class batch, Hindi) | P07 |
| Fees: structure, dues, collect (mock), receipts, reversals, advance credit, ledger | P03, P07 |
| Day book, reports, audit viewer | P07 |
| Approvals (all request types), My requests, Inbox, notifications | P03, P07, P08 |
| Printing (desktop + Android), WhatsApp share | P07 |
| Backups (daily, verified, local + Drive, USB copy), restore to new PC | P08 |
| New session rollover with carry-forward | P08 |
| Settings (all), storage screen, accent colours | P08 |
| English + Hindi everywhere (UI, receipts, report cards, logo) | P01 keys → P08 complete |
| Security review, performance, accessibility, size gates, signed builds, docs | P09 |
