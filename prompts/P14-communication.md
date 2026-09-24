# PHASE 14 — COMMUNICATION: UPI QR, EMAIL, WHATSAPP, ABSENCE ALERTS, FEE REMINDERS, CIRCULARS

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
14. Run every command you mention and paste real output. Branch `v2/p14`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-14.md`.


## OBJECTIVE
Let the school reach parents and staff at zero running cost: per-student UPI QR codes, email from the
school's own Gmail, one-tap WhatsApp sharing, absence alerts, fee reminders, circulars with staff read
tracking, and the optional school-paid automatic WhatsApp module.

## DONE MEANS
- Settings → Payments (prototype `settings` state 1) saves the UPI ID; receipts show the balance QR
  (prototype `receipt`); a real UPI app on a real phone scans the QR and shows the right payee,
  amount and note (manual test; record the app used — do NOT complete a payment).
- Email from the school PC arrives in a test inbox with the QR inline, in the guardian's language.
- Phone share sheet sends text + files to WhatsApp (group chosen by the user); desktop opens wa.me.
- Absence alerts (prototype `absence`), fee dues + reminder sheet (prototype `feesadmin` states 2–3),
  circulars (prototype `circulars`) and staff inbox read tracking (prototype `staffday` state 4) work.
- Automatic WhatsApp module can be configured and sends a template to a Meta test number (only if the
  owner provides a test setup; otherwise built, unit-tested and marked "not verified live").

## STEPS

### Step 0 — Two checks before building (report in `phase-14-checks.md`)
1. **Gmail API [VERIFY]:** confirm from Google's official docs: the send endpoint, the minimum scope
   for sending (`gmail.send`), whether that scope needs Google app verification for production use
   and what that involves, and the sending limits for a free Gmail account. Adding the scope means
   the sync account signs in again (re-consent). If verification is required, STOP after the report
   so the owner can start the verification process; continue with Steps 2, 4, 5 (non-email parts).
2. **UPI link [VERIFY]:** confirm the `upi://pay` parameters (`pa`, `pn`, `am`, `cu`, `tn`) and length
   limits from NPCI's published linking specification (or bank documentation if NPCI's isn't public);
   record the source.

### Step 1 — UPI settings and QR
- `school.upi_id`, `school.upi_name`, toggles (receipts / reminders / printed dues lists).
- vidya-core `upi.rs`: `validate_vpa`, `upi_uri(vpa, name, amount_paise, note)` (amount as rupees
  with 2 decimals, note trimmed to the verified limit, URL-encoded).
- QR rendering with the existing `qrcode` crate → SVG for screens and print, PNG bytes for email
  (render SVG to PNG in the webview canvas; no new crate).
- Setup wizard: optional "School UPI ID" field on the School step (skippable) + Settings → Payments.
- Receipt template: "Pay the balance by UPI" block when balance > 0 and the toggle is on.

### Step 2 — Tap-to-WhatsApp
- `plugins/vidya-android`: add `share({text, files[]})` using `Intent.ACTION_SEND` /
  `ACTION_SEND_MULTIPLE` with a `FileProvider`; files are copied to the app cache first and cleaned
  after 24 h.
- Desktop: `tauri-plugin-opener` → `https://wa.me/91<mobile>?text=<encoded>` (one parent) or
  `https://wa.me/?text=<encoded>` (user picks a chat/group). Files on desktop: "Save attachment"
  dialog + note "Attach this file in WhatsApp".
- Every tap writes a `message` row with channel `wa_tap`, status `tapped` (never "sent").

### Step 3 — Email sender (school PC only)
- Add the verified send scope to the sync account's OAuth request; Settings → Google Drive shows
  "Email: allowed / needs sign-in again".
- Sender service on the server: takes `queued` email messages (created on any device and synced),
  builds MIME by hand (UTF-8 subject encoded per RFC 2047, `multipart/related` with inline QR PNG,
  `multipart/mixed` for attachments ≤ 20 MB total), sends via the Gmail API, marks `sent` with the
  provider message id or `failed` with the exact error. Rate: 1 message / 2 s; daily cap setting
  (default 400, or the verified limit if lower); remaining queue continues next day with a visible
  note. Guardians without `messages` consent or email are skipped with reason.
- From-name = school name; reply-to = school email if set.

### Step 4 — Absence alerts (phone + desktop register)
After a sheet is submitted, the phone shows **Absent today** (prototype `absence`): each absent
student with primary guardian, buttons Email / WhatsApp, "Email both parents", message preview in the
guardian's language (template `absence_alert`). Email → queued message (sent by the school PC when it
next syncs; status shown). WhatsApp → share sheet / wa.me. Desktop register has the same action for
a date. Principal setting: who may send absence alerts (class teacher default).

### Step 5 — Fee dues and reminders
- Fees → **Dues** screen (prototype `feesadmin` state 2): strip (total due, students with dues,
  overdue instalments, collected today), table (student + primary guardian, class, instalment, due
  date pill, amount, Email / WhatsApp buttons), bulk "Email all N parents" (queue, with count of
  skipped: no email / no consent), filters. (Instalments themselves arrive in Phase 15; until then a
  due's "instalment" column shows the fee head.)
- Reminder sheet (prototype `feesadmin` state 3): channel segmented (Email free / WhatsApp free /
  Automatic — shown only if the module is on), language chips, preview with QR, Send.
- Accountant and Principal can send; teachers never see this.

### Step 6 — Circulars & notices
- Tables: `circular(id, number, title, body, languages_json, audience_json, channels_json,
  attachments_json, status draft|pending_approval|sent, created_by, approved_by, sent_at,
  calendar_event_id NULL)`, `circular_read(circular_id, staff_id, read_at)`.
- Principal: Circulars list + compose (prototype `circulars` state 1): title, message, audience
  (whole school / classes / staff only), languages (per-language bodies; Hindi/Telugu optional),
  attachment, channels (staff app, parent WhatsApp groups via share, email, printed notice, automatic
  WhatsApp if on). Number assigned on the server when sent (`CIR/2026-27/014`). Optional "Add to
  calendar".
- Sent view (prototype `circulars` state 2): channel summary and staff "Read by N of M" list with
  "Remind" (in-app notification).
- Teachers: draft **class notices** → `class_notice` approval request → Principal approves → sent.
- Staff phones: Inbox shows circulars (prototype `staffday` state 4) → "Mark as read" syncs a read
  event.
- Printed notice: print engine, A4 letterhead, number, date, body in chosen language, optional
  tear-off slip ("I have read circular 14 · Parent's signature · Student · Class").
- WhatsApp groups: one "Share" per class; Vidya lists the classes and marks each `tapped`.

### Step 7 — Automatic WhatsApp (optional module `wa_auto`, off by default)
- **[VERIFY]** in Meta's official WhatsApp Cloud API docs: send-message endpoint and current Graph
  API version, template message payload, error codes, and rules for utility templates.
- Settings → Languages & modules → Automatic WhatsApp → setup page: phone number ID, access token
  (stored encrypted), template names per purpose and language, "Send test message" to a number.
  Plain explanation: "Your school pays Meta per message (about ₹0.12 for reminders and alerts, plus
  GST). Vidya's developer charges nothing for this."
- Server sends queued `wa_auto` messages (rate-limited). No webhook (no public server) → final status
  "accepted by WhatsApp" with the returned message id, or the exact error.
- ADMIN-GUIDE section: step-by-step Meta setup (business account, phone number, templates).

### Step 8 — Tests
vidya-core: VPA validation, UPI URI encoding, template placeholders per language, consent rule,
queue daily cap, circular numbering. Integration: phone absence email created offline → synced →
sent by server (fake Gmail API) → status back on phone. MIME snapshot tests (Hindi/Telugu subjects).
Playwright: prototype fidelity for `settings`(1), `receipt`, `absence`, `feesadmin`(2,3),
`circulars`(1,2), `staffday`(4).

## STOP CONDITIONS
Gmail verification required (after report). Meta API details can't be verified from official docs.
Any need for a public webhook or server.

## HANDOFF → `docs/phase-notes/phase-14.md`
Checks report · UPI format + source · email pipeline + limits · share plugin API · screens built ·
automatic WhatsApp status (live-verified or not) · templates needing language review · tests.
