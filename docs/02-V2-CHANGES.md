# VIDYA — V2 CHANGES (binding)

> **Merged into `docs/00-SYSTEM-CONTEXT.md` in Phase 11.** This file is kept unchanged as the
> decision record (the "why"). From Phase 11 on, `00-SYSTEM-CONTEXT.md` is the single source of
> truth; where the two ever differ, 00-SYSTEM-CONTEXT wins. Open owner decisions now live in
> `docs/OWNER-DECISIONS.md`.

This file records every decision made after Phases 1–10 were built. **Where it disagrees with
`docs/00-SYSTEM-CONTEXT.md`, this file wins.** Phase 11 merges it into 00-SYSTEM-CONTEXT.md
so there is again one source of truth; until then, agents read both.

Markers as before: **[OWNER]** = business decision (build the stated default, keep it in one
config place, list it in the handoff). **[VERIFY]** = technical fact to prove with a real test
or official documentation before building on it.

---

## 1. Business, branding, contacts

- Credit everywhere (About screen, installers' publisher field, website, receipts footer,
  videos): **"Developed by Zuhair Hussain"**. Copyright line: "© 2026 Zuhair Hussain".
- Support email: `mohammedzuhairhussain28@gmail.com`. Website: `vidya.zuhairhussain.com`.
  Address: Hyderabad, Telangana. No phone number anywhere.
- Business model unchanged: **one-time purchase, perpetual licence, no expiry, no limits.**
  Price **[OWNER]**, shown as `[PRICE]` until provided. Optional yearly support (AMC) is sold
  outside the app; the app never stops working without it.
- Bills to buyers: simple bill, **no GST line** (owner confirmed; owner to verify with a CA).
- **App identifier [OWNER, must decide before the first public release]:** keep the current
  identifier, or change to `com.zuhairhussain.vidya`. It can never change after release.
  Phase 11 stops and asks if no answer is recorded.

## 2. The zero-monthly-cost rule (new, top priority)

No core feature may depend on an always-on server run by the company. Allowed free pieces:
the school's own PC, the school's own Google accounts, free static website hosting (GitHub
Pages or Cloudflare Pages), and the owner's own laptop and Gmail.

Consequences for what Phases 1–10 built:
| Built in P01–P10 | V2 status |
|---|---|
| `cloud/relay` + relay route | Kept as an **optional, off-by-default module "Instant sync"** (a paid add-on later). Release builds must not require `RELAY_URL`. |
| `cloud/licence` server, payment webhooks, admin panel | **Retired.** Replaced by offline licence files made on the owner's laptop (§7). Code moves to `cloud/_archive/` (kept for reference, not built). |
| Company website with checkout | Replaced by a **static site** (`site/`) with a UPI QR and instructions (§7.5). |
| Google Drive as a fallback | **Promoted to the internet route**: LAN first, then Drive (§6). |

## 3. Roles and attendance rules (changed)

- Roles stay Principal / Accountant / Teacher, now stored as **role rows with a permission
  set** (§8.10) so more role templates can be added later. No custom-role UI yet.
- **Attendance is Present / Absent only.** The Leave (L) mark is removed from new sheets.
  % present = P ÷ (P + A). Legacy `L` marks already stored stay untouched in history and are
  shown as "Leave (old)"; they count as absent in percentages **[OWNER default]**.
- Only the **class teacher** takes attendance for a class. Another teacher may take it only
  through an **attendance duty**: (a) the teacher requests it and the Principal approves, or
  (b) the Principal assigns a substitute (§10.4). Duty is limited to specific dates and ends
  automatically.
- **No attendance cut-off time.** Remove the cut-off setting and the "usually done by 10:30"
  copy; Home lists classes "not submitted yet today".
- Low attendance: **no thresholds or alerts.** Absent students' names are highlighted in the
  day list and register.
- Offline access lease stays 30 days.

## 4. Languages

- UI, receipts, report cards, reports, messages and circulars in **English, Hindi and Telugu**.
- Telugu font: **Noto Sans Telugu** 400/500/600 (Fontsource, Telugu subset only), added to
  every font stack after Noto Sans Devanagari. Like Hindi, Telugu headings use Noto Sans
  Telugu 500 at the same size, with no italics.
- `vidya-core::words` gains `amount_in_words_te`. Any Telugu number word you are unsure of →
  list for owner review; do not guess silently.
- Logo: the brand kit has English and Hindi lockups only. **[OWNER]** to supply a Telugu
  lockup; until then show the English lockup in Telugu mode.
- Each staff member picks their own language; receipts and report cards use the school's
  chosen language; each guardian has a preferred language for messages.
- The Telugu strings in `design/prototype/VidyaPrototype.jsx` are the owner's samples and must
  be reviewed by a native speaker before release (flag in the handoff).

## 5. Design references (new screens)

- The five original mock screens stay the pixel contract (01-MOCK-SPEC).
- **Every new screen is specified by `design/prototype/VidyaPrototype.jsx`** (see
  `docs/03-PROTOTYPE-SPEC.md`). It uses the same tokens, components and animations. Build
  new screens to match it exactly, using the existing app components; a component the app
  lacks → build it once in `src/components/` matching the prototype, then reuse.
- Attendance mock change: remove the L button, the Leave count and the Leave bar segment
  (the prototype shows the result). Update the Phase 1 fidelity baseline for Attendance with
  owner approval.

## 6. Google accounts and the Drive route (changed)

### 6.1 Two accounts
| Account | Owner | Holds | Who signs in |
|---|---|---|---|
| **School sync account** (new free Gmail, e.g. `vidya.<school>@gmail.com`) | The school | Encrypted change files, confirmation files, the epoch marker, class notes (not encrypted), outgoing email | School PC and every staff device. The Principal types the password on each device during joining; teachers are never told it |
| **Backup account** | The Principal personally | Encrypted daily backups | School PC only |

Replaces the v1 design of sharing folders with each staff member's own Google account.
Staff personal Google accounts are no longer used.

### 6.2 Folder layout
```
Sync account's Drive
  Vidya/<school name> (<school_id>)/
    exchange/
      ops-<device_id>/          sealed change bundles from that device
      acks/<device_id>.json     sealed confirmations from the school PC
      epoch.json                signed: {school_id, server_epoch, server_machine_code, at}
    notes/<class>/<yyyy-mm>/     homework & notes attachments (study material, NOT encrypted)
Backup account's Drive
  Vidya/<school name> (<school_id>)/backups/
```

### 6.3 Behaviour
- Route order: **LAN (direct, instant) → Drive (≈1–2 minutes) → queue on device.** The relay
  route exists only if the optional Instant-sync module is on.
- School PC checks Drive every **30 s ± 5 s** while running (not every 5 minutes). Devices check
  every 20 s ± 3 s in the foreground.
- Every device does its **own** OAuth sign-in to the sync account (its own refresh token).
  Tokens are never copied between devices.
- Removing a device: the server rotates audience keys; the Principal may also change the
  sync account password (the Staff & access screen shows this step).
- Encryption, audiences (`admin`, `finance`, `class:<id>`) and bundle format unchanged.
- **[VERIFY — first step of Phase 12]** with one real sync account and 3 devices: `drive.file`
  scope lets every device (same account, same OAuth client) list, read and write the files
  other devices created; Android sign-in method works (from the Phase 6 spike).

### 6.4 Fencing without a licence server
`exchange/epoch.json` is written (signed with the school's server key) whenever the server
epoch changes. A PC whose epoch is lower than the file's → becomes read-only with "This
computer is no longer the school server". Devices refuse any server with a lower epoch.

## 7. Licence v2: offline licence files

### 7.1 Machine code
Shown on the Welcome → Set up screen: 12 Crockford base32 characters in 3 groups plus a
1-character checksum, e.g. `7KQ2-M9XD-4TRA-P`. Derived from the stable `machine_id` in the OS
keychain (SHA-256 → first 60 bits → Crockford) so it stays the same across app updates.

### 7.2 Licence
Licence JSON `{v:2, licence_id, school_name, machine_code, issued_at, plan:"perpetual",
modules:["core", …]}` signed with ed25519. Delivered as a **licence key** (base64url of
payload‖signature, shown in groups of 5 for pasting) **or** a `.vlic` file. The app verifies
offline with the public key in build config. No expiry, no online check, no remote revoke.

### 7.3 Licence maker (owner's laptop only, never shipped)
`tools/licence-maker/` — a small Rust CLI (its own Cargo.toml, not in the app workspace):
`init` (creates the keypair in a folder the owner chooses; prints the public key),
`issue --school "…" --machine 7KQ2-M9XD-4TRA-P --utr 4265… --email …` (validates the checksum,
writes the licence key + `.vlic`, appends to `register.csv`), `transfer --licence <id>
--machine <new>` (issues a new licence linked to the old one; register marks old "moved"),
`list`. The owner backs up the private key and register in two places (README says how).

### 7.4 Moving to a new PC
Restore (Recover) on the new PC → shows its machine code → the owner issues a transfer
licence → server epoch + 1 → `epoch.json` updated → the old PC fences itself when it next
sees Drive.

### 7.5 Static website (`site/`)
Plain HTML/CSS (brand tokens, Welcome-screen look): what Vidya is, platforms, honest size
text, `[PRICE]`, **how to buy**: pay by UPI (the owner's UPI QR image, `[OWNER]` file), then
send UTR + school name + machine code via a prefilled `mailto:` to the support email.
Downloads section reads `releases.json` produced by the release workflow. Privacy, terms and
refund pages (drafts marked for owner review). Hosting: GitHub Pages or Cloudflare Pages.

## 8. Foundation upgrades (build before new modules)

1. **School calendar backbone:** weekly offs (default Sunday), holidays, exam days and events;
   `is_working_day(date)` in vidya-core; used by attendance %, fee due dates, salary days,
   leave days, timetable.
2. **Guardians:** separate `guardian` table (name, relation, mobile, email, language,
   whatsapp_ok) linked to students (`student_guardian`, one primary). Migrate existing
   guardian fields; keep the old columns read-only until Phase 18 drops them.
3. **General ledger:** `ledger_account`, `voucher`, `ledger_entry` (append-only, balanced
   debits = credits per voucher, checked in vidya-core). Every payment, reversal, expense,
   salary payment, advance and store sale creates one voucher in the same transaction.
   Backfill vouchers for existing payments/reversals (derived rows, originals untouched).
4. **Numbering engine:** one `number_series` for receipts `R-A2-0419`, vouchers `V-A2-0088`,
   store sales `S-A2-0031`, circulars `CIR/2026-27/014` (server only), hall tickets.
5. **Approval engine:** the request table serves every type through a handler registry in
   vidya-core. New types: `leave`, `attendance_duty`, `class_notice`.
6. **Messaging engine:** `message` outbox (channel `email | wa_tap | wa_auto | app`, template,
   language, recipient, status `draft | queued | sent | tapped | failed | read`, honest error
   text) + `message_template` in en/hi/te with named placeholders validated in vidya-core.
7. **Print templates:** one print engine (letterhead, language, A4/A5/80 mm) for receipts,
   report cards, hall tickets, seating charts, salary slips, notices, registers.
8. **Module switches:** `module_setting` rows; defaults in §11. Disabled modules hide their
   screens, reject their commands, and don't sync their tables to devices.
9. **Custom fields** for students and staff (text, number, date, choice; label in en/hi/te).
10. **Roles as data:** `role` + `role_permission`; the three built-ins are seeded; permission
    checks read them (the exhaustive matrix test still passes).
11. **Telugu** everywhere (§4).
12. **`school_id` on every table** (single value now; ready for a future multi-branch add-on).

## 9. Privacy features (India's DPDP law)

The school is the data fiduciary; Vidya never sends school data to the developer. Add:
- **Consent record** per student: guardian, purpose (`school_records`, `messages`), method
  (`signed_form | in_person`), recorded by, date, withdrawn date. Messages are not sent to a
  guardian without `messages` consent.
- **Export one student's data** (JSON + readable PDF) and **erase on request** where the law
  allows (financial and audit records are kept; personal fields replaced by a tombstone;
  audited).
- **Retention setting** for students who left (default: keep **[OWNER/legal]**; no automatic
  deletion without Principal confirmation).
- **Incident log** (date, what happened, action taken, reported to board yes/no/date) with a
  reminder of the 72-hour reporting duty. Admin guide: "confirm your duties with a lawyer".
- No behavioural tracking or advertising; no analytics SDKs (already a rule).

## 10. New modules

### 10.1 Communication
- **UPI:** `school.upi_id` (validated `^[a-zA-Z0-9.\-_]{2,256}@[a-zA-Z]{2,64}$`) and display name
  set in Setup/Settings. Per-student QR = `upi://pay?pa=<id>&pn=<name>&am=<rupees.paise>&cu=INR&tn=<≤50 chars>`
  **[VERIFY against NPCI's UPI linking spec]**, rendered with the existing `qrcode` crate. Shown
  on receipts (balance), fee reminders, and printed dues lists (toggles). Vidya never handles
  money; the accountant records payments as today.
- **Email (free):** sent **only by the school PC** through the Gmail API as the sync account
  (`users/me/messages/send`, MIME built by hand, base64url). **[VERIFY]** scope
  (`gmail.send`), its Google verification category, and daily sending limits; if sensitive-
  scope verification is required for production, STOP and tell the owner the steps and time.
  Devices create `message` rows; the server sends when online; rate limit and a daily cap
  (setting, default 400); status shown honestly.
- **Tap-to-WhatsApp (free):** phone → Android share sheet via `plugins/vidya-android`
  (`share({text, files[]})`, FileProvider); desktop → `https://wa.me/91<mobile>?text=…` for one
  parent, `https://wa.me/?text=…` to pick a chat/group. Status `tapped` (Vidya cannot know if
  it was actually sent).
- **Automatic WhatsApp (optional module, school pays Meta):** WhatsApp Cloud API directly from
  the school PC with the school's own phone number ID, access token and pre-approved utility
  templates per language. **[VERIFY]** current Graph API endpoint/version and template rules
  in Meta's docs. No webhook (no public server) → status stops at "accepted by WhatsApp". Setup
  guide in ADMIN-GUIDE. Tokens stored encrypted.
- **Absence alerts:** after a sheet is submitted, list absent students → email (queued) or
  WhatsApp tap per guardian, in the guardian's language.
- **Fee reminders:** dues list → email all (queued) or WhatsApp one by one; message includes
  amount, instalment, due date and the UPI QR (email: inline image; WhatsApp: text + QR image
  via share on phone, text-only link on desktop).
- **Circulars & notices:** title, body, attachments, audience (whole school / classes / staff
  only), languages, channels (staff app, WhatsApp groups via share, email, printed notice with
  tear-off slip, automatic WhatsApp if on). Numbered on the server. Teacher-drafted class
  notices need Principal approval. Staff read tracking (read events sync). Optional calendar
  link. Archive with search.

### 10.2 Fees
- **Instalments:** each fee head may define instalments (count, amounts, due dates). Dues are
  generated per instalment (`fee_due.instalment_no`, `due_date`). Changing a plan previews
  affected unpaid dues first; paid dues never change.
- Out of scope (owner decision): concessions, sibling discounts, late fees, certificates (TC,
  bonafide, character), admission enquiries, student photos, ID cards.

### 10.3 School accounts
- **Expenses:** category (ledger expense account), amount, paid via cash/UPI/bank, details,
  optional bill photo (compressed; stored in the local attachment store and backed up;
  not in Drive notes). Voucher number on save. Mistakes → reversal voucher, never edits.
- **Cash book:** per day: opening balance, money in, money out, cash + bank in hand; from
  ledger entries on cash/bank accounts. Opening balance set once by the Principal (opening
  voucher).
- **Profit summary:** income vs expense accounts per month and year to date, plus "fees still
  due".
- **Salary register:** monthly salary per staff (effective dates), days present from staff
  attendance + approved leave, unpaid-leave deduction = monthly ÷ working days in that month
  × unpaid days, rounded half-up to the rupee **[OWNER default]**, advance recovery, net pay;
  "Pay" creates salary vouchers; salary slips print.
- **Optional School store:** items with price and stock, sales with receipts (S- series),
  stock moves, low-stock warning, ledger store income.

### 10.4 Classroom
- **Timetable:** period times per session; slots (class, day, period, class-subject, teacher);
  vidya-core rejects clashes (teacher or class double-booked). Teachers see "My timetable".
- **Substitutes:** from an approved leave or manually: list the teacher's slots that day and
  free, present teachers → assign → substitution records → substitute notified; if the absent
  teacher is a class teacher, the substitute gets that day's attendance duty.
- **Homework & notes:** class + subject, homework or notes, text, photos (compressed on
  device) and PDFs (≤ 10 MB each, ≤ 20 MB total); stored in the sync account's `notes/` folder;
  shared by WhatsApp (share sheet) or email; history per class. Warning: study material only.
- **Report card remarks:** per student per exam, templates in en/hi/te, printed.
- **Exams — seating & hall tickets:** rooms (name, rows × columns, invigilator), pair two
  classes per room alternating, fill deterministically, print seating charts and hall tickets
  (4 per A4).
- **Calendar screen:** month view, holidays/exams/events, share to WhatsApp, add from circulars.

### 10.5 Staff HR
- **Staff attendance:** check-in/out on the phone. Default: check-in counts as "at school"
  only when the device reaches the school PC over LAN; otherwise it is recorded as "away"
  for the Principal to accept **[OWNER setting]**. Late after school start time (setting,
  default 09:00).
- **Leave:** types with yearly quotas **[OWNER defaults: Casual 12 paid, Sick 6 paid, Unpaid]**,
  balances, request on phone → Principal approves (approval engine) → leave record, salary
  days, substitute suggestion.

## 11. Module switches (defaults)
Core (always on) · School accounts **on** · Classroom **on** · Staff HR **on** · Circulars
**on** · Automatic WhatsApp **off** · School store **off** · Instant sync (relay) **off**.

## 12. Dependencies
No new crates or npm packages are needed for v2:
- QR: existing `qrcode`.
- Photo compression: on Android via the `vidya-android` plugin (Bitmap → JPEG quality 70,
  long edge ≤ 1600 px); on desktop via a canvas in the webview before sending to Rust.
- Email MIME: built by hand with existing `base64`.
- Share sheet: in-repo `vidya-android` plugin (Kotlin), no new dependency.
- Telugu font: `@fontsource/noto-sans-telugu` (fonts are an allowed category; add to §13).
Anything else → STOP and ask.

## 13. Size and cost limits (unchanged)
Each download ≤ 40,000,000 bytes, installed ≤ 50,000,000 bytes. Running cost to the developer:
₹0 per month.

## 14. Owner decisions still open
1. App identifier (before first public release). 2. Price. 3. Telugu logo lockup.
4. Salary deduction formula (default in §10.3). 5. Leave types/quotas. 6. Remote staff
check-in allowed? 7. Retention for students who left. 8. Owner's UPI QR image for the website.
9. Native-speaker review of Hindi and Telugu. 10. Google verification for `gmail.send` (if
required). 11. Code signing (still optional).
