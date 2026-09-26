# Phase 14 handoff — Communication (UPI QR, email, WhatsApp, absence alerts, fee reminders, circulars)

> **Status: IN PROGRESS.** Steps 0, 1, the Step 2 **foundation**, **Step 3** core (MIME +
> send queue), **Step 4**, **Step 5**, and **Step 6** are built, tested and committed. Step 7
> (auto-WhatsApp), Step 2 (Android native), 3-live (real Gmail client, gated on the P12 OAuth
> spike), and the Playwright fidelity of Step 8 are **specced below and not yet built** — a
> scoped runway, the way P12/P13 handed off partial phases. Every committed increment is
> genuinely working and green; nothing is marked "done" that isn't.

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
  | `abe535f` | interim handoff + OWNER-DECISIONS #13 |
  | `863fc6b` | 5 — Fee dues screen + reminder sheet |
  | `be24f36` | 3 — email sender testable core (MIME + send queue) |
  | `8787320` | 4 — absence alerts (phone + SendAbsenceAlert) |
  | `c3a4aa2` | 6 — circulars & notices (compose, numbering, read tracking) |

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

### Step 3 — Email sender, Testing mode (owner #13) — CORE DONE (`be24f36`); live wiring gated
**Built + tested (`src-tauri/src/email.rs`, 8 tests):**
- `build_mime` — hand-built RFC 2822: **RFC-2047** `=?UTF-8?B?…?=` subject/from (Hindi/Telugu),
  base64 bodies, **`multipart/related`** with the inline QR PNG (`cid:qrcode`), **`multipart/mixed`**
  for attachments; `to_raw` = base64url for Gmail's `raw`. Date + boundary are passed in → output
  is deterministic and snapshot-testable.
- `GmailSender` trait + `FakeGmail` (mirrors `sync::drive::DriveApi`, so the live client and the
  fake are interchangeable). `GmailError` = RateLimited / DailyLimit / TokenRevoked / Io.
- `send_queued_emails` — drains `channel='email' status='queued'`: builds MIME, sends, marks
  **`sent`** (+ provider id) / **`failed`** (+ exact error), skips no-email / withdrawn-consent
  with a counted reason, honours the **daily cap** (default 400, < the ~500/day free-Gmail limit),
  stops on cap / rate-limit / revoked (leaving rows `queued`), and appends a status **op** so
  devices sync the honest status. `email_config` (from-name = school, from = connected sync Gmail,
  reply-to, cap from `settings_json.email_daily_cap`) + `drain_queue` (background-task entry;
  **no-op without a connected sync account**). `GMAIL_SEND_SCOPE` / `GMAIL_SEND_ENDPOINT` constants.

**NOT built / not verified live (gated on the P12 OAuth spike):** the real `reqwest` Gmail client
(a `GmailSender` impl that POSTs to `GMAIL_SEND_ENDPOINT` with the sync account's bearer token), the
**timer** in `server/start.rs` that calls `drain_queue` at 1 msg / 2 s when online, adding
`gmail.send` to the sync-account **OAuth consent** (re-consent), and the Settings → Google Drive
"Email: allowed / needs sign-in again" indicator (deferred with the P12 Drive UI). The queued rows
Step 5 creates therefore accumulate honestly as `queued` until that client lands.

**Enhancement:** inline-QR emails need the webview to render the SVG→PNG (no new crate) and put the
PNG bytes in `message.attachments_json`; the MIME builder already accepts `qr_png`, so this is a
compose-side wire-up.

### Step 4 — Absence alerts (DONE, committed `8787320`) — phone screen built
- **vidya-core**: new action **`SendAbsenceAlert`** (Core; class teacher of `target.class_id` +
  Principal; Accountant never) via `class_owned` targeting. `ALL` 41→42 (test updated to 42).
- **app**: `action_for_message_kind` now returns `(Action, TargetKind)`; `record_message` derives
  the absence **target class from the related student server-side**, so a teacher can only alert
  their OWN class's absentees (a `cls-2a` student via `stf-meena` → FORBIDDEN, tested).
  `list_absent` (submitted sheet only) → absent students + primary guardian + consent +
  `absence_alert` preview in the guardian's language. Command `list_absent`.
- **frontend**: phone **AbsenceContainer** (prototype `absence`), shown after attendance submit
  (route `/teacher/absence/:classId/:date`, navigated from `AttendanceContainer.onSubmit`). Email →
  **queued** (honest — the school PC sends later; **deviates from the prototype's optimistic "Email
  sent" copy per rule 13**, noted); WhatsApp → `wa_tap` + `wa.me`; "Email all parents". i18n en/hi.
  Action buttons are **text-only** (the mock icon set has no mail/chat icon — rule 6).
- **Deferred:** the desktop-register variant of the same action, the Principal "who may send absence
  alerts" setting (class-teacher default), and phone pixel-fidelity (canonical machine only).

### Step 5 — Fee dues + reminders (DONE, committed `863fc6b`)
- **vidya-core**: new action **`SendFeeReminder`** (Core; Accountant + Principal; Teacher never),
  wired through `Action`/`ALL`/`as_key`/`from_key`/`can`/`module_for`/`target_kind`. **Changed test
  expectation:** `action_all_covers_every_variant…` now asserts `ALL.len() == 41` (was 40) — the
  only P14 behaviour change to an existing test so far; the exhaustive `role_matrix` derives the
  new row automatically.
- **app**: `list_dues_logic` (strip: total due / students with dues / unpaid dues / collected
  today; one row per outstanding fee due with the student's primary guardian + `messages` consent
  + fee head + balance; optional class filter). `render_fee_reminder` fills the `fee_reminder`
  template in the guardian's language with `{amount}` (Indian ₹ grouping, `inr`), `{instalment}`
  (fee heads until P15 due dates), `{upi_link}` (gated by the reminders toggle). `preview_fee_reminder`
  (sheet) + `queue_fee_reminders` (bulk "Email all", queues one `email` message per **eligible**
  guardian = has email AND consent, counts skipped no-email / no-consent). Message insert refactored
  into `insert_message_in_tx` (shared single + bulk). Commands `list_dues` / `preview_fee_reminder`
  / `queue_fee_reminders`.
- **frontend**: Fees → **Dues** tab (`FeesDues.tsx`, prototype `feesadmin` 2): strip + table
  (Email / WhatsApp per row) + "Email all N parents" with a skip summary + the reminder sheet
  (`feesadmin` 3: channel segmented Email/WhatsApp, language chips, preview **with the UPI QR**,
  Send). Email → queued; WhatsApp → `wa_tap` row + `wa.me`. i18n en/hi.
- **Seed gap (noted):** the demo seed inserts students with **no guardian rows** (real schools
  create guardians inline via `create_student`), so with the demo seed the Dues table shows no
  guardian names and 0 emailable. A seed guardian backfill is a small follow-up (also helps the
  Playwright `feesadmin` fidelity + a richer demo). Real data is unaffected.

### Step 6 — Circulars & notices (DONE, committed `c3a4aa2`) — core built
- **Migration 0018**: `circular` (status draft|pending_approval|sent, `number`, audience/channels/
  languages_json, `calendar_event_id`, + sync/`school_id`) + `circular_read` (UNIQUE per staff).
  Both under the `circulars` module in `scope::module_of_table`.
- **vidya-core**: `ManageCirculars` (Principal) + `DraftClassNotice` (Teacher →
  `NeedsRequest(ClassNotice)` — the `class_notice` request type exists from P13). Accountant denied
  both. `ALL` 42→44 (test updated). `module_for` → `Module::Circulars`.
- **app**: `list` / `save` (draft) / `send` / `mark_circular_read`. **Send assigns the server
  number `CIR/<session>/NNN` atomically** (`numbering::next_no(Circular)`), sets `status=sent`;
  `mark_circular_read` idempotent. All gated by `require_module(circulars)` + permission. Scope:
  Principal all; other staff only **sent** circulars (their inbox); `circular_read` own.
- **frontend**: Circulars nav item (School group, `inbox` icon — no circulars icon in the mock
  set, rule 6) + `CircularsScreen` (list + compose: title/message/audience/languages/channels +
  Save draft / Send; Sent view: channel summary + "Read by N of M" bar). i18n bundle en/hi.
- **Deferred (documented):** the **teacher class-notice approval flow** (`DraftClassNotice` →
  `class_notice` request → Principal approves → sent), **send-time channel fan-out** (queue guardian
  emails / wa-group taps / printed notice / `wa_auto`), the **per-staff read list + "Remind"**, the
  **printed notice with tear-off slip**, **"Add to calendar"**, and the **staff phone Inbox screen**
  (`staffday` 4 — the `mark_circular_read` backend is done, the phone UI is not). Pixel fidelity on
  the canonical machine.

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
- `cargo test -p vidya-core --lib` → **362 passed** (upi **11**, messages **8**, +permissions
  SendFeeReminder/SendAbsenceAlert/ManageCirculars/DraftClassNotice); `cargo test -p vidya --lib`
  → **234 passed / 0 failed** (was 215 at P13 tip; +3 messaging, +5 Step 5, +8 Step 3 email, +2
  Step 4 absence, +2 Step 6 circulars). `migrations_apply_and_are_idempotent` green (0018).
- **Changed existing test expectation (P14):** `permissions::…action_all_covers…` `ALL.len()`
  40→**44** (added `SendFeeReminder`, `SendAbsenceAlert`, `ManageCirculars`, `DraftClassNotice`).
  No other existing test value changed.
- `cargo clippy -p vidya -p vidya-core --lib -- -D warnings` → clean.
- `npm run typecheck` clean; `check:i18n` **990 keys en/hi in sync**; `check:hex` OK (42 tokens,
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
