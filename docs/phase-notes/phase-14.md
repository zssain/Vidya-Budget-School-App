# Phase 14 handoff — Communication (UPI QR, email, WhatsApp, absence alerts, fee reminders, circulars)

> **Status: IN PROGRESS.** Steps 0, 1 and the Step 2 **foundation** are built, tested
> and committed. Steps 2 (Android native), 3–7 and the Playwright fidelity of Step 8
> are **specced in detail below and not yet built** — a scoped runway, the way P12/P13
> handed off partial phases. Every committed increment is genuinely working and green;
> nothing is marked "done" that isn't.

## Start state / environment
- Branch `v2/p14`, cut from `v2/p13` @ `71b0658`. HEAD at write time `ab46f3f`.
- `git status`: clean; all work committed on `v2/p14` (no push/merge/tag).
- Tools: rustc/cargo/clippy **1.98.1**, node **v25.3.0**, npm **11.7.0**, pnpm 10.31.0,
  java (OpenJDK) **21**. `cargo-tauri` not on PATH (invoked via npm script). **No Android
  SDK/Gradle/emulator in this environment** — Android-native work can't be compiled or
  run here (see Step 2 / follow-ups).
- Commits this phase:
  | Commit | Step |
  |---|---|
  | `fcc6d30` | 0 — verification checks (`docs/phase-14-checks.md`) |
  | `bb6a20d` | 1 — UPI settings + per-student balance QR |
  | `ab46f3f` | 2 (part) — message outbox foundation + desktop tap-to-WhatsApp |

## Owner decisions taken this phase
- **#13 Gmail (`gmail.send`) → decided (interim): run the OAuth app in Testing mode now**,
  build + wire the sender for it; publish + sensitive-scope verification before general
  release. See `docs/OWNER-DECISIONS.md` #13 and the checks report.
- Build scope for the session: **full non-email build first, commit each**; parts that
  can't be verified in this sandbox (Android share plugin, live Gmail/WhatsApp, real-phone
  UPI scan, Playwright pixel fidelity) are documented as "built/specced, not verified here."

---

## Step 0 — verification checks (DONE) → `docs/phase-14-checks.md`
Both `[VERIFY]` gates closed against primary sources (URLs in the report).

- **Gmail `gmail.send`** is a Google **Sensitive** scope. Send endpoint
  `POST https://gmail.googleapis.com/gmail/v1/users/me/messages/send` (upload variant for
  attachments), body `{ "raw": base64url(MIME) }`. Publishing an app that uses it requires
  **OAuth app verification** (3–5 business days; live homepage + privacy policy + Search
  Console domain ownership + per-scope justification + unlisted demo video) — but **not**
  the restricted-scope security audit. **Testing mode** avoids verification (≤100 whitelisted
  accounts, 7-day token expiry) — fine for the owner's pilot, not for wide distribution.
  Free-Gmail sending cap **~500 recipients/day** (recipient-counted); Vidya's default 400 is
  safe. **STOP applied to the live sender only** (per Step 0); everything else proceeds.
- **UPI `upi://pay`** verified against **NPCI UPI Linking Spec v1.6 (Nov 2017)**: `pa`,`pn`
  mandatory; `am` decimal (editable if absent); `cu`=INR only; `tn` optional. **Two honest
  corrections:** NPCI sets **no length limit** on `tn`/`pn` (our ≤50-char `tn` cap is ours,
  documented as ours), and **no signing** is needed — a school paying into its own VPA uses a
  plain unsigned static QR, which the spec explicitly allows (`mode`/`sign`/`orgid` are only
  for merchant-initiated *signed* intents, which need an acquiring-bank RSA key). No STOP.

---

## Step 1 — UPI settings + per-student balance QR (DONE, committed `bb6a20d`)

**vidya-core `upi.rs`** (pure, 11 tests): `validate_vpa` (`^[A-Za-z0-9.\-_]{2,256}@[A-Za-z]{2,64}$`,
hand-rolled — no regex crate) and `upi_uri(vpa, name, amount_paise, note)` →
`upi://pay?pa=…&pn=…&am=<rupees.paise>&cu=INR&tn=…` with RFC-3986 percent-encoding
(space=`%20`, `@` kept literal in `pa`), `tn` capped to `MAX_NOTE_CHARS = 50` (our cap),
`am` omitted when ≤0 (amount-editable link).

**app `upi.rs`** (3 tests): `PaymentSettings::read` (from `school.settings_json`) with
`preview_link`; `qr_svg` (navy `#0C1B38` on white, via the existing `qrcode` crate — same
builder as the invite QR). No PNG in Rust — email will render SVG→PNG in the webview canvas.

**commands / DTOs** (`commands/logic.rs`, `mod.rs`, `commands.json`, `api.ts`):
- `setup_school` gained optional (validated) `upi_id`/`upi_name` on the School wizard step.
- `get_payment_settings` (any session) / `set_payment_settings` (Principal-only, audited,
  VPA validated, merges into `settings_json` keeping `phone`) / `qr_svg` (any session).
- `get_receipt` DTO gained `upi_link: Option<String>` — set only when a UPI id is configured,
  the "show QR on receipts" toggle is on, and the balance after the payment is > 0.

**frontend**:
- **Settings → Payments** = `src/screens/desktop/PaymentsSettings.tsx`, rendered as a **card**
  in the existing card-grid Settings (prototype `settings` state 1 *content*: UPI id + display
  name + 3 toggles + live preview QR). **Fidelity caveat:** the real Settings screen is the
  P11 single-page card grid, **not** the prototype's left-nav tabbed chrome; the Payments
  *content* matches, the tabbed layout does not (consistent with P11's Settings decision).
- **Receipt** "Pay the balance by UPI" block (`ReceiptDoc.tsx`) when `upi_link` is set.
- Setup wizard optional "School UPI ID" field (skippable). i18n en/hi added
  (`settings.pay.*`, `receipt.doc.upi*`, `setup.school.upi*`); `te` falls back to English.

**Manual test still owed (real phone, do NOT complete a payment):** scan a generated receipt
QR with a real UPI app and confirm payee + amount + note; record the app used.

---

## Step 2 — Tap-to-WhatsApp (FOUNDATION DONE `ab46f3f`; Android native = follow-up)

**Built + tested (desktop path + shared outbox):**
- **vidya-core `messages.rs`**: `wa_me_url(mobile, text)` → `https://wa.me/91<mobile>?text=…`
  (one guardian) or `https://wa.me/?text=…` (group); a malformed mobile falls back to the group
  form so a bad number never yields a broken direct link. Shared `urlencode` (RFC 3986). 8 tests.
- **app message outbox** (`commands/logic.rs`): `record_message_logic` — one transaction (row +
  audit + op). `wa_tap` → status **`tapped`** (never "sent"), `email`/`wa_auto` → **`queued`**.
  `action_for_message_kind` gates by purpose (`receipt_share → PrintShareReceipt`; unknown
  kind/channel rejected — a new purpose can't skip a permission check). `list_messages_logic`
  (Principal sees all; others only what they created; status + related filters). Commands
  `record_message` / `list_messages`; `MessageDto` / `RecordMessageInput` in `api.ts`.
- **sync/scope**: `message_template` visible to every role (Core reference data every device
  reads to compose); `message` rows scoped **Principal-all / creator-own** in both `snapshot`
  and `visible_row` (a teacher/accountant never receives another role's messages).
- **Desktop receipt "Share on WhatsApp"** (`ReceiptsScreen.tsx`) now records a `wa_tap` message
  row (status `tapped`) via `record_message` before opening `wa.me` through the existing
  `shareWhatsApp`/opener path. (The desktop `wa.me` opener itself already existed from P07.)

**Follow-up — Android native `share({text, files[]})` (needs an Android build env):**
The in-repo Tauri plugin `src-tauri/plugins/vidya-android` **does not exist** (00-SYSTEM-CONTEXT
§4 describes it, but only Tauri's generated scaffolding under `src-tauri/gen/android/` is
present — a Rule-3 doc/code discrepancy, reported). Step 2's phone share sheet therefore means
**creating** that plugin, which cannot be compiled or run in this sandbox. Design to implement:
1. New Tauri 2 Android plugin crate `src-tauri/plugins/vidya-android/` (Rust `#[tauri::command]`
   bridge + `android/` Kotlin `TauriPlugin`), registered in `src-tauri` via `tauri::plugin` and
   in `gen/android` gradle.
2. Kotlin `share(text: String, files: List<String>)`: copy each file into the app cache dir,
   expose via a `FileProvider` (`<provider>` in `AndroidManifest.xml`, `file_paths.xml`), build
   an `Intent.ACTION_SEND` (one file / no file) or `ACTION_SEND_MULTIPLE` (many), `type` =
   the files' MIME (or `text/plain`), `EXTRA_TEXT` = text, `EXTRA_STREAM` = the content URIs,
   `FLAG_GRANT_READ_URI_PERMISSION`, `startActivity(Intent.createChooser(...))`. Clean cached
   files after 24 h.
3. JS half `share({text, files})`; on desktop it no-ops to the `wa.me`/opener path already built;
   files on desktop → "Save attachment" dialog + note "Attach this file in WhatsApp".
4. Every phone tap records a `wa_tap` message row (reuse `record_message`).

---

## Steps 3–7 — SPECCED, NOT YET BUILT (design worked out; foundation is ready)

### Step 3 — Email sender (school PC only), Testing mode (owner #13)
- Add `https://www.googleapis.com/auth/gmail.send` to the **sync-account** OAuth request
  (currently `drive.file` only). Settings → Google Drive shows "Email: allowed / needs sign-in
  again" (re-consent). *(The live Drive/OAuth client itself is still spike-gated from P12 — see
  P12 handoff Step 0; the Gmail scope rides on the same client when it lands.)*
- **Sender service on the server** (a tokio loop like the Drive import loop in
  `server/start.rs`): take `message` rows with `channel='email'` + `status='queued'`, build MIME
  **by hand** (UTF-8 subject per RFC 2047 `=?UTF-8?B?…?=`; `multipart/related` with the QR PNG
  inline via `cid:`; `multipart/mixed` for attachments ≤ 20 MB total), base64url the message,
  `POST users/me/messages/send`, mark `sent` (+ `provider_ref` = returned id) or `failed` (+ the
  exact error). Rate **1 msg / 2 s**; daily cap setting (default **400**, never above the
  account's real limit); remaining queue continues next day with a visible note. Guardians with
  no `messages` consent or no email are **skipped with a reason** (`vidya_core::messages::can_message`).
- From-name = school name; reply-to = school email if set.
- **Tests (runnable here):** MIME builder unit + snapshot tests (Hindi/Telugu subjects — RFC-2047);
  daily-cap counter; consent-skip; an integration test with a **fake Gmail API** (a trait the
  sender calls, real impl = `reqwest`, test impl = in-memory) proving offline `queued` → synced →
  server sends → status back. **Not verifiable live here** (needs the real OAuth client) → mark
  "not verified live."
- QR-as-PNG: render the SVG to PNG in the webview canvas (no new crate) when composing the email,
  attach the PNG bytes (base64) to the `message` row (`attachments_json`).

### Step 4 — Absence alerts (phone + desktop register)
- New action `SendAbsenceAlert` (Core module; **class teacher of the target class + Principal**;
  Accountant never). Add to `Action`/`ALL`/`as_key`/`from_key`/`can`/`module_for`; the exhaustive
  `role_matrix` test auto-covers it. Extend `action_for_message_kind`: `absence_alert →
  SendAbsenceAlert`. Refine sync scope so a teacher also sees their own `absence_alert` messages
  (already creator-scoped) and the `absence_alert` template (already all-roles).
- After a sheet is **submitted**, the phone shows **Absent today** (prototype `absence`): each
  absent student + primary guardian, buttons Email / WhatsApp, "Email both parents", a preview in
  the guardian's language (template `absence_alert`, placeholders `student_name/date/school_name`).
  Email → `record_message(channel:email, kind:absence_alert, …)` (queued); WhatsApp → `wa_me_url` +
  `record_message(channel:wa_tap…)` + share sheet (phone) / opener (desktop).
- Desktop register: same action for a chosen date. Principal setting "who may send absence alerts"
  (class teacher default) stored in `settings_json`.

### Step 5 — Fee dues + reminders (desktop; highly testable, no Android)
- Fees → **Dues** screen (prototype `feesadmin` state 2): strip (total due, students with dues,
  overdue instalments, collected today), table (student + primary guardian, class, **fee head**
  as the "instalment" column until P15 adds real instalments, due-date pill, amount, Email /
  WhatsApp), bulk "**Email all N parents**" (queues one `email` message per eligible guardian,
  reporting counts skipped for no-email / no-consent), filters. Reuse the existing dues query
  (`student_dues`) + guardians.
- Reminder sheet (prototype `feesadmin` state 3): channel segmented (Email free / WhatsApp free /
  Automatic — shown only if `wa_auto` on), language chips, preview **with the UPI QR** (reuse
  `PaymentSettings.link` + `qr_svg`; gate by the `on_reminders` toggle), Send.
- New action `SendFeeReminder` (Core; **Accountant + Principal**; Teacher never). Extend
  `action_for_message_kind`: `fee_reminder → SendFeeReminder`. Template `fee_reminder`
  (placeholders incl. `upi_link`). Accountant + Principal only; teachers never see this screen.

### Step 6 — Circulars & notices (module `circulars`, on by default)
- **Migration 0018**: `circular(id, number, title, body, languages_json, audience_json,
  channels_json, attachments_json, status draft|pending_approval|sent, created_by, approved_by,
  sent_at, calendar_event_id NULL, + sync/`school_id` columns)`, `circular_read(circular_id,
  staff_id, read_at, …)`. Add both to `sync/scope::module_of_table` → `Some("circulars")` and to
  the role scopes (circular → all staff by audience; circular_read → own). Number assigned on the
  **server** at send via `vidya_core::numbering::next_no(Circular, session)` → `CIR/2026-27/014`.
- New actions `ManageCirculars` (Circulars module; Principal) + `DraftClassNotice` (Circulars;
  Teacher → a `class_notice` request the Principal approves — the request type already exists from
  P13). Compose (prototype `circulars` state 1): title, message, audience (whole school / classes /
  staff only), per-language bodies (hi/te optional), attachment, channels (staff app, parent
  WhatsApp groups via share, email, printed notice, automatic WhatsApp if on), optional "Add to
  calendar". Sent view (state 2): channel summary + staff "Read by N of M" + "Remind" (in-app
  notification). Staff phones Inbox (prototype `staffday` state 4) → "Mark as read" writes a
  `circular_read` op. Printed notice via the print engine (A4 letterhead, number, date, chosen
  language, optional tear-off slip). WhatsApp groups: one "Share" per class, each marked `tapped`.

### Step 7 — Automatic WhatsApp (module `wa_auto`, off by default)
- **[VERIFY still owed]** current Meta WhatsApp **Cloud API** send endpoint + Graph API version,
  utility-template payload, error codes, template rules — from Meta's official docs, recorded like
  the Step 0 checks **before** building on them (STOP if not confirmable). *(Not done this session.)*
- Settings → Languages & modules → Automatic WhatsApp setup page: phone-number id, access token
  (**stored encrypted**, like `drive_account.token_enc`), template names per purpose+language,
  "Send test message". Plain cost note: "Your school pays Meta per message (about ₹0.12 for
  reminders and alerts, plus GST). Vidya's developer charges nothing for this."
- Server sends `wa_auto` queued messages (rate-limited). **No webhook** (no public server) → final
  status **"accepted by WhatsApp"** with the returned message id, or the exact error. Gate every
  `wa_auto` send + the config commands by `require_module(enabled, Module::WaAuto)`. ADMIN-GUIDE
  section: step-by-step Meta setup (business account, phone number, templates).

---

## Sync / modules / permissions — current wiring
- `message` + `message_template` are **Core** (always synced), scoped per above. `circular` /
  `circular_read` will be `Some("circulars")` in `module_of_table` (Step 6). `wa_auto` gates
  sends, not tables.
- New comms **Actions** are added per step (Step 4 `SendAbsenceAlert`, Step 5 `SendFeeReminder`,
  Step 6 `ManageCirculars`/`DraftClassNotice`) — each must update `Action`/`ALL`/`as_key`/
  `from_key`/`can`/`module_for`; the exhaustive `role_matrix` + `every_action_maps_to_a_module`
  tests enforce completeness.
- Audience (`audience.rs`, P13): `message`/`message_template` mapped to `admin`; refine per
  recipient when server-side sending lands (Step 3/7).

## Tests (all real, this session)
- `cargo test -p vidya-core` — upi **11**, messages **8** (+ existing); `cargo test -p vidya --lib`
  → **218 passed / 0 failed** (was 215 at P13 tip; +3 messaging: wa_tap `tapped`+op, teacher
  forbidden, bad kind/channel).
- `cargo clippy -p vidya -p vidya-core --lib -- -D warnings` → clean.
- `npm run typecheck` clean; `check:i18n` **954 keys en/hi in sync**; `check:hex` OK (42 tokens,
  no new colours); `api.test.ts` command parity green (COMMANDS ↔ commands.json ↔ api.ts).
- **Owed once the later steps land:** MIME snapshot tests (Step 3), circular numbering test (Step
  6), the offline→sync→fake-Gmail→status integration (Step 3), and Playwright prototype fidelity
  for `settings`(1) / `receipt` / `absence` / `feesadmin`(2,3) / `circulars`(1,2) / `staffday`(4).
  **Playwright pixel fidelity can only be asserted on the canonical macOS baseline machine** (P11
  precedent — `tests/e2e/__screens__` PNGs are gitignored); `settings`(1) will be a
  content/interaction test, not a pixel match, because Settings keeps the card-grid layout.

## Templates needing language review (OWNER-DECISIONS #12)
The seeded `message_template` Hindi/Telugu bodies (P13) are **drafts** pending native review; the
new P14 en/hi UI strings (`settings.pay.*`, `receipt.doc.upi*`, `setup.school.upi*`) are en/hi with
English fallback for `te`. Flag both for the native-speaker pass.

## Owner decisions still open (see `docs/OWNER-DECISIONS.md`)
#1 app identifier · #2 price · #6 Telugu logo · #7 salary formula · #8 leave types · #9 remote
check-in · #10 retention · #11 owner's UPI QR image (website) · #12 native review of Hindi/Telugu
· #14 code signing · #15 GST. **#13 gmail.send → decided (interim): Testing mode now, verify
before general release.** Still needing an owner action before those parts ship: **Meta Cloud API
[VERIFY]** (Step 7) and the **live Drive/Gmail OAuth client** (P12 spike, gates Step 3 live).
