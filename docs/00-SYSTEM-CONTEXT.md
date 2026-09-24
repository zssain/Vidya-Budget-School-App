# VIDYA BUDGET SCHOOL — SYSTEM CONTEXT (v2)

This file is the source of truth for every phase. Every phase prompt tells the agent to read
it first, in full. It is the v1 context with every decision from `docs/02-V2-CHANGES.md` merged
in (Phase 11). `02-V2-CHANGES.md` is kept as the decision record; this file now wins if they
ever differ. Open owner decisions live in `docs/OWNER-DECISIONS.md`.

- If code and this file disagree, this file wins.
- If this file is silent on something, STOP AND ASK the owner. Never invent features, APIs,
  libraries, numbers, legal wording or UI copy.
- Items marked **[OWNER]** are open business decisions. Build the stated default, keep it in
  one config place, and list it in the phase handoff.
- Items marked **[VERIFY]** are technical facts that must be proven with a real test before
  building on them.

Companion files (read them too):
- `docs/01-MOCK-SPEC.md` — every token, animation, asset and interaction of the five mock
  screens (see its "v2 amendments" section for the attendance change).
- `docs/03-PROTOTYPE-SPEC.md` + `design/prototype/VidyaPrototype.jsx` — the reference for
  **every new v2 screen** (same tokens/components/animations as the mock).
- `design/screens/*.dc.html` — the five mock screens (the visual contract).
- `design/assets/*.svg`, `design/brand-kit/` — the real logo and app icons.

---

## 0. Business, branding, zero-cost rule (v2)

- **Developed by Zuhair Hussain.** Credit everywhere (About screen, installers' publisher
  field, website, receipts footer, videos): **"Developed by Zuhair Hussain"**. Copyright line:
  **"© 2026 Zuhair Hussain"**.
- Support email `mohammedzuhairhussain28@gmail.com`; website `vidya.zuhairhussain.com`; address
  Hyderabad, Telangana. **No phone number anywhere.**
- **Business model:** one-time purchase, perpetual licence, no expiry, no limits. Price
  **[OWNER]** (shown as `[PRICE]`). Optional yearly support (AMC) is sold outside the app; the
  app never stops working without it. Bills to buyers: simple bill, **no GST line** (owner
  confirmed; owner to verify with a CA).
- **App identifier `in.vidyabudget.app`** — **[OWNER, must decide before the first public
  release]:** keep it or change to `com.zuhairhussain.vidya`. It can never change after release.
- **Zero-monthly-cost rule (top priority):** no core feature may depend on an always-on server
  run by the company. Allowed free pieces: the school's own PC, the school's own Google
  accounts, free static website hosting (GitHub/Cloudflare Pages), the owner's own laptop and
  Gmail. Consequences: the `cloud/relay` route becomes an optional, off-by-default module
  ("Instant sync"); `cloud/licence` (online activation) is replaced by offline licence files
  made on the owner's laptop (§10); the checkout website becomes a static site with a UPI QR
  (§10.5); Google Drive is promoted from a fallback to **the** internet route (§11).

## 1. Product in one paragraph

School-management software for small Indian schools (about 200–1,500 students, 5–60 staff).
A school buys a perpetual licence (one-time, by UPI) and receives an **offline licence file**
(`.vlic` / licence key). The Principal installs Vidya on the school's main PC, activates it
offline and sets up the school. That PC becomes the **school server** holding the official
record. Staff join by invitation. Teachers take attendance (Present/Absent) and enter marks
for their own classes; accountants handle admissions and fees; the Principal sees everything
and approves corrections. Phones and PCs reach the school server on the school LAN, and when the
server is off they exchange encrypted changes through the school's Google Drive; the Vidya relay
is an optional add-on for instant internet sync. With no internet at all, work is saved on the
device and sent later. Nothing is ever silently overwritten or lost. English, Hindi and Telugu.
Windows, macOS, Android.

## 2. Deliverables

| File | Platform | Hard limits |
|---|---|---|
| `Vidya_<ver>_x64-setup.exe` (NSIS) | Windows 10/11 x64 | download ≤ 40 MB, installed ≤ 50 MB |
| `Vidya_<ver>_universal.dmg` | macOS 12+, Intel + Apple Silicon | download ≤ 40 MB, installed ≤ 50 MB. If universal exceeds either limit, ship `_aarch64.dmg` + `_x64.dmg` instead |
| `Vidya_<ver>_arm64-v8a.apk`, `Vidya_<ver>_armeabi-v7a.apk` | Android 7+ (minSdk 24), 2 GB RAM phones | each download ≤ 40 MB, installed ≤ 50 MB |
| `SHA256SUMS.txt` | all | — |

Size accounting (decimal: 40 MB = 40,000,000 bytes):
- **Download** = the file above.
- **Installed** = everything Vidya puts on disk at install time (app folder / .app bundle /
  APK + extracted native libs), before any school data exists.
- **Not counted but disclosed**: the OS web engine (Microsoft WebView2 on Windows 10 if
  missing, Android System WebView), school data, backups, logs. The download page and
  INSTALL.md must say this.
- CI fails the release if any limit is exceeded. Running cost to the developer: **₹0/month.**

Company-side services (in this repo, never shipped inside the apps):
- `cloud/relay` — internet relay for reaching school servers. **Optional** (the paid
  "Instant sync" module, off by default). Release builds must not require `RELAY_URL`.
- `cloud/licence` — the P10 online licence/website service. **Retired in v2** (moves to
  `cloud/_archive/`, kept for reference, not built): replaced by offline licence files (§10)
  and a static site (`site/`, §10.5). `cloud/licence --dev` may still be used to sign dev
  licences during development.

## 3. Non-negotiable rules

1. **The UI is the mock and the prototype.** `design/screens/*.dc.html` define the five
   original screens exactly (colours, fonts, sizes, weights, spacing, radii, borders, shadows,
   copy, icon SVG paths, animations, interactions), checked by screenshot comparison
   (01-MOCK-SPEC §9). **Every new v2 screen is specified by `design/prototype/VidyaPrototype.jsx`**
   (03-PROTOTYPE-SPEC) and built from the app's existing components and tokens. shadcn/ui and
   Tailwind defaults NEVER win over the mock/prototype. A component the app lacks → build it once
   in `src/components/` matching the prototype, then reuse.
2. **Offline-first.** Every feature works with no network. Nothing is lost offline.
3. **Zero monthly cost / minimal dependencies.** No core feature depends on a company server
   (§0). Only the packages in §13. Anything else → stop and ask.
4. **Encrypted everywhere.** Encrypted at rest on every device; encrypted before anything
   reaches Google Drive or the relay. Google, the relay operator and the developer can never
   read school data. The school is the data fiduciary; Vidya never sends school data to the
   developer (§9 privacy).
5. **Money is permanent.** Payments are never edited or deleted. Mistakes are fixed by an
   approved reversal. Money actually received is never rejected by sync (§8.6). Every money
   movement also writes a balanced voucher (§7 v2 ledger).
6. **Locks.** Submitted attendance sheets and submitted marks sheets (per exam subject) are
   locked. Changes go through a correction request approved by the Principal.
7. **Audit.** Append-only, hash-chained, tamper-evident. Nobody can edit it, not even the
   Principal.
8. **Conflicts are flagged, never silently overwritten.**
9. **Rules live in `crates/vidya-core`** — pure Rust, no IO. The UI and the sync code never
   decide business rules; the server re-checks every change with vidya-core.
10. **Indian formats.** ₹ with lakh grouping (₹6,84,200). Dates "Wednesday, 23 September".
    Session April–March shown "2026–27" (en dash). Mobiles: 10 digits starting 6–9.
11. **English, Hindi and Telugu everywhere**, including receipts, report cards, reports,
    messages and circulars (§4a).
12. **Size and speed.** Limits in §2. Must run on a 4 GB RAM Windows PC and a 2 GB RAM
    Android phone.
13. **Honest status.** The UI never shows success for something that did not happen
    (no fake "synced", "backed up", "printed", "sent"). No behavioural tracking, advertising
    or analytics SDKs.

## 4. Stack (decided)

- Shell: **Tauri 2** (Rust). Desktop + Android from one repo.
- UI: **React 18 + TypeScript + Vite**, **Tailwind CSS v4** (`@tailwindcss/vite`),
  **shadcn/ui** components copied into `src/components/ui` and restyled to the mock.
- Animation: **CSS keyframes and transitions copied verbatim from the mock** (no animation
  library — this reproduces the mock exactly and keeps phones fast).
- Data: **SQLite + SQLCipher** via `rusqlite`, one encrypted database per device.
- Rules: `crates/vidya-core` — pure Rust, no IO, no async, no database.
- App crate `src-tauri`: database, commands, school server, sync, Drive, backups, printing.
- Android native bits: one in-repo Tauri plugin `src-tauri/plugins/vidya-android` (Kotlin):
  Keystore key wrapping, printing, Google sign-in bridge, share sheet, photo compression.
- Networking: `axum` (server), `tokio-rustls` + `hyper-util` (TLS serving), `reqwest`
  (client), all TLS through **rustls with the `ring` provider only**.
- Google Drive REST v3 and Gmail API via `reqwest`. No Google SDK.

### 4a. Languages (v2)

- UI, receipts, report cards, reports, messages and circulars in **English, Hindi and Telugu**.
- Telugu font **Noto Sans Telugu** 400/500/600 (Fontsource, Telugu subset only), added to every
  font stack after Noto Sans Devanagari. Telugu headings use Noto Sans Telugu 500 at the same
  size, no italics (like Hindi). `vidya-core::words` gains `amount_in_words_te`; any Telugu
  number word in doubt → list for owner review, never guess.
- Each staff member picks their own UI language; receipts and report cards use the school's
  chosen language; each guardian has a preferred language for messages.
- Logo: brand kit has English + Hindi lockups only; **[OWNER]** to supply a Telugu lockup —
  until then show the English lockup in Telugu mode.
- Telugu (and Hindi) sample strings in the prototype are pending native-speaker review before
  release (flag in every handoff that touches them).

### 4.1 Existing repository

Phases 1–10 are built on the existing repo (`zssain/Vidya-Budget-School-App`): git history kept,
old plain-JS frontend under `legacy/` (not built), `crates/vidya-core` modules ported with their
tests. v2 extends this built product — read the actual code before changing it; never rebuild
what exists.

## 5. Roles and permissions

Enforced in `vidya-core::permissions::can(actor, action, target)`, checked in every Tauri
command, again on the school server for every op, and when building every data scope. Roles are
Principal / Accountant / Teacher, stored as **role rows with a permission set** (§7 v2:
`role` + `role_permission`, the three built-ins seeded) so more role templates can be added
later. No custom-role UI yet. The exhaustive permission-matrix test still passes.

**Principal** (owner): everything. Approves / rejects / returns requests. Invites, suspends
and removes staff and devices. Settings, licence, Google Drive, backups, restore, session
rollover, module switches. Direct edits to locked data are allowed but always audited with a
reason.

**Accountant**: admissions (create students, enroll, transfer section), student details
edits → request, fee dues view, record payments, print/share receipts, reversal → request,
day book / cash book / expenses, fee reports, student CSV import/export (no marks/attendance
columns). No marks. No attendance. No staff management.

**Teacher**: **attendance for classes where they are class teacher** — another teacher may take
it only through an **attendance duty** (the teacher requests it and the Principal approves, or
the Principal assigns a substitute; duty is limited to specific dates and ends automatically,
§10.4). Marks for their own class-subjects; view their students (no fee data at all; guardian
address only if class teacher); report cards for their classes (view/print/share); corrections
→ request.

**Everyone**: own profile, PIN, language, own requests, inbox, Sync screen.

Staff states: `invited`, `active`, `suspended` (cannot sign in or sync; data kept),
`removed`. Only the Principal changes states.

## 6. Screens

### 6.1 In the mock (build pixel-exact from `design/screens`)
1. **Welcome · Set up or join** (desktop 1440×960; phone: single column, navy panel hidden).
2. **Principal · Home** (desktop 1440×1080).
3. **Accountant · Collect fee** (desktop 1440×1080, sheet over the student list).
4. **Teacher · Home** (Android 390×844, 3×3 tiles).
5. **Teacher · Attendance** (Android 390×844) — **v2: Present/Absent only** (see §7 attendance
   and 01-MOCK-SPEC "v2 amendments").

### 6.2 New v2 screens (build from `design/prototype/VidyaPrototype.jsx`, 03-PROTOTYPE-SPEC)
Every prototype screen id maps to an app route/place in 03-PROTOTYPE-SPEC §3, with its phase.
Build each to match the prototype exactly, reusing the mock's components and tokens. The
Principal sidebar follows the prototype `NAV_PRINCIPAL` grouping: **Overview** (Home,
Approvals) · **Academics** (Students, Attendance, Marks & exams, Timetable, Calendar) ·
**Finance** (Fees, Accounts, School store) · **School** (Staff & access, Circulars, Backups,
Settings). Items whose module or screen does not exist yet are **hidden** (not placeholders).

Derivation rules for any screen a phase must build that the prototype does not draw:
- Desktop page = mock sidebar + header + `PageTitle` + cards/tables like Principal Home /
  Collect fee. Tables = the Collect fee table pattern. Side panels = the Collect fee sheet.
  Phone pages = the Attendance navy header + light list + bottom action bar. Success = the
  Collect fee navy success pattern.
- If a screen needs something these patterns can't express → STOP AND ASK with the smallest
  proposal. Never invent new colours, radii, shadows or font sizes.

## 7. Data model

SQLite, ids = UUIDv7 text, money = integer **paise** (i64), times = UTC ISO-8601 text,
dates = `YYYY-MM-DD`. Every synced table has: `id, version INTEGER, hlc TEXT, created_at,
updated_at, updated_by_staff, updated_by_device, sync_state` where sync_state ∈
`draft | on_device | shared_drive | confirmed | rejected | conflict`. Migrations are additive
and numbered; never edit or delete a committed migration; never delete financial or audit rows.
**v2 adds `school_id` on every table** (single value now; ready for a future multi-branch add-on).

Tables (v1 core, unchanged unless noted):
- `school` (name, address, board, udise NULL, logo_blob NULL, backup_salt, server_epoch,
  settings_json)
- `licence` (licence_id, school_id, plan, issued_at, max_students NULL, max_devices NULL,
  signature, raw_json, last_check_at, status active|revoked|moved) — v2 licences are verified
  offline (§10); no online check required.
- `academic_session` (label "2026–27", starts_on, ends_on, is_current, read_only)
- `term` (session_id, name, starts_on, ends_on)
- `class` (name, section, display "VII-B", class_teacher_id, sort_order)
- `subject` (name, name_hi, **name_te**)
- `class_subject` (class_id, subject_id, teacher_id)
- `staff` (name, role principal|accountant|teacher, mobile, google_email NULL, pin_hash,
  state invited|active|suspended|removed, **language en|hi|te**)
- `device` (staff_id, platform windows|macos|android, name, token_hash, receipt_series,
  admission_series, last_seen_at, revoked_at, lease_expires_at, needs_rejoin)
- `invite` (code_hash, staff_id, expires_at, used_at, revoked_at)
- `student` (admission_no NULL until confirmed, provisional_no, name, dob, gender,
  guardian_name, guardian_mobile, address, transport, category NULL, rte bool,
  aadhaar_status none|submitted|verified, status active|left, left_on, left_reason) — the inline
  `guardian_*` columns become read-only once the v2 `guardian` table exists (dropped in P18).
- `enrollment` (student_id, class_id, session_id, roll_no, from_date, to_date NULL)
- `attendance_sheet` (class_id, date, status draft|submitted, submitted_by, submitted_at)
  UNIQUE(class_id, date)
- `attendance_mark` (sheet_id, student_id, mark P|A|L) UNIQUE(sheet_id, student_id)
  — **v2: new marks are P or A only; `L` is retained solely to read/keep legacy rows** (§7a).
- `exam` (session_id, term_id, name, starts_on, ends_on)
- `exam_subject` (exam_id, class_subject_id, max_marks)
- `marks_sheet` (exam_subject_id, status draft|submitted) — lock is PER SUBJECT
- `mark_entry` (sheet_id, student_id, marks INTEGER NULL, absent bool)
- `grade_scale`, `grade_band` (min_pct, max_pct, grade, grade_point)
- `fee_head` (name, name_hi, amount_paise, frequency term|month|once, applies_to
  all|transport|class_ids_json, active)
- `fee_due` (student_id, fee_head_id, period, amount_paise, cancelled_at NULL) — v2 fees add
  per-instalment dues (`instalment_no`, `due_date`, §10.2).
- `payment` (receipt_no, student_id, amount_paise, mode cash|upi|cheque, reference,
  collected_by, collected_at, device_id, confirmed_at) — APPEND-ONLY (trigger)
- `payment_allocation` (payment_id, fee_due_id NULL, amount_paise, kind due|advance_credit)
- `reversal` (payment_id, reason, request_id, approved_by, applied_at) — APPEND-ONLY
- `request` (type, target_table, target_id, base_version, before_json, after_json, reason,
  requested_by, revision, parent_request_id NULL, status, decided_by, decided_at, note,
  apply_state, applied_at) — v2 adds types `leave`, `attendance_duty`, `class_notice`
  through the approval handler registry.
- `conflict`, `review_flag`, `notification`
- `audit_log` (seq, at, staff_id, device_id, action, table, record_id, before_json,
  after_json, reason, op_id, prev_hash, hash) — APPEND-ONLY (trigger)
- `outbox`, `op_log` (server_seq), `applied_ops`, `sync_cursor`, `drive_state`, `backup_run`,
  `schema_version`, **`schema_meta`** (KV; holds the v2 attendance cutover time).

### 7a. v2 tables to be added (by the phase that owns them)
- **`module_setting`** (key, enabled, changed_by, changed_at) — Phase 11 (this phase); §11.
- **School calendar** (weekly offs, holidays, exam days, events; `is_working_day`) — P13.
- **`guardian`** + `student_guardian` (one primary) — P13.
- **General ledger**: `ledger_account`, `voucher`, `ledger_entry` (append-only, balanced per
  voucher) — P15; backfilled for existing payments/reversals.
- **`number_series`** (receipts, vouchers, store sales, circulars, hall tickets) — P14/P15.
- **Messaging**: `message` outbox + `message_template` (en/hi/te) — P14.
- **`role` + `role_permission`** (built-ins seeded) — P13/P17.
- **Custom fields** for students/staff — later.
- **Privacy**: `consent`, export/erase, retention setting, `incident_log` — P13 (§9).
- Communication/fees/accounts/classroom/HR tables per §10 — P14–P17.

## 8. Sync model

### 8.1 Ops
Every change is an **op**: `{op_id (UUIDv7), hlc, device_id, staff_id, audience, table,
record_id, kind insert|update|delete|action, payload, base_version, server_epoch}`.
HLC = `wall_ms (13) + counter (4) + device_id`; string compare = total order. HLC ordering
lives in vidya-core; it orders and detects conflicts only, never decides who is "right".

### 8.2 Write path (any device)
UI → Tauri command → vidya-core validates + checks permission **and the module switch** → ONE
SQLite transaction: update row + append audit entry + append op to outbox (+ voucher & ledger
entries for money) → return DTO. Drafts (`sync_state = draft`) never leave the device.

### 8.3 Delivery routes (automatic, in order)
1. **LAN**: HTTPS to the school server (last address, then mDNS `_vidya._tcp.local`),
   certificate pinned by SHA-256 fingerprint. Instant.
2. **Google Drive** (school server unreachable, internet available): encrypted op bundles
   uploaded to the sync account's Drive → `shared_drive`; devices read bundles they can decrypt
   and show them as provisional. ≈ 1–2 minutes. **This is the internet route in v2.**
3. **Instant sync (relay)** — only if the optional `instant_sync` module is on: HTTPS/WSS
   through the Vidya relay (works behind CGNAT), pinned cert, bodies end-to-end sealed.
4. **No network**: stay `on_device`; retry every 15 s while open, on resume and on network
   change. School PC checks Drive every 30 s ± 5 s while running; devices every 20 s ± 3 s in
   the foreground (best effort in background; never promised).

### 8.4 School server
Re-validates every op with vidya-core as the op's staff member, using their CURRENT permissions
(revoked/suspended authors → `review_flag`, not applied) **and the current module switches (a
disabled module's ops are rejected)**. Applies ops in HLC order, assigns `server_seq`, confirms,
rejects with a reason code, or flags a conflict. Idempotent by `op_id`. On start and periodically
it imports pending Drive bundles in HLC order, writes ack files, archives processed bundles.

### 8.5 Conflicts
Update whose `base_version` is older AND touches a field changed since by another device → keep
current, create `conflict` rows, notify the Principal. Different fields merge. Attendance: two
devices submit the same sheet → per-student comparison; equal marks merge, different marks →
conflict per student. Marks: same cell from the same base → conflict.

### 8.6 Money in sync (must never lose money)
- On the collecting device, vidya-core requires `0 < amount ≤ total due now` (Collect fee red
  error). That is the ONLY place this limit applies.
- On the server, a payment op is **always accepted and confirmed** if the student exists and the
  author was allowed to collect fees when it was recorded. Excess → `advance_credit` +
  `review_flag excess_payment`. The receipt stands.
- Possible duplicates → `review_flag possible_duplicate_payment`; both stay until the Principal
  requests a reversal. Payments never produce `conflict` rows.

### 8.7 Numbers issued offline
Receipts: per-device series at join (`R-A2-0419`), permanent. Admissions: `provisional_no` until
the server assigns the official `admission_no` (`YYYY/NNNN`).

### 8.8 Scopes, audiences, lease
- The server sends each device only what its role may see (§5) **and only tables of enabled
  modules**.
- Drive bundles are sealed per audience: `admin`, `finance`, `class:<id>`. A device gets keys
  only for its audiences; changing assignments rotates the affected class key.
- Offline access lease: `lease_expires_at` = last server contact + **30 days**. After expiry the
  app opens, keeps unsent work, but hides school data until the device reaches the school server.
- Revocation takes effect at next server contact / Drive read; remote wipe of an offline phone
  is impossible and never promised. Removing a device rotates audience keys; the Principal may
  also change the sync-account password (Staff & access shows this step).

### 8.9 Server epoch (fencing)
`school.server_epoch` starts at 1, increases on restore-to-new-PC or licence transfer. Every
response carries the epoch. Devices refuse a server with a lower epoch. Without a licence server,
`exchange/epoch.json` (signed with the school's server key) fences a demoted PC to read-only
("This computer is no longer the school server").

### 8.10 Exact status copy
"Draft saved on this phone" · "Offline · saved on this phone" · "Offline · N waiting to send" ·
"Submitted, saved on this phone." · "Shared through school Drive · waiting for school" ·
"Confirmed by school server" · "All changes confirmed · 12 s ago" · "Up to date · synced 2 min
ago" · "₹2,400 more waiting for server" · "Adm. no. pending". (Desktop: "saved on this computer".)

## 9. Security & privacy (DPDP)

- DB key 32 random bytes: desktop → OS keychain (`keyring`, service `in.vidyabudget.app`,
  account `db-key`); Android → AndroidKeyStore-wrapped via `plugins/vidya-android`.
- PIN per user, 4–6 digits, Argon2id (m=19 MiB, t=2, p=1). Auto-lock after 5 min (setting
  1/5/15). 5 wrong → 30 s doubling, max 1 h. Local unlock only, never a network credential.
- School server TLS: self-signed `rcgen` cert (10 y); clients pin its SHA-256 fingerprint from
  the invitation. Device auth: random 32-byte token; server stores only the SHA-256 hash;
  constant-time compare; revocable. Relay sees only sealed envelopes. Drive bundles/backups
  encrypted before upload.
- Recovery key: shown once, 30 Crockford base32 chars in 6 groups. Backup key =
  Argon2id(recovery key, salt in `school`, m=64 MiB, t=3, p=1).
- Audit: `hash = SHA-256(prev_hash ‖ canonical JSON)`; triggers `RAISE(ABORT)` on
  UPDATE/DELETE of `audit_log`, `payment`, `reversal`; chain verified daily and before every
  backup; chain head in Settings → About and with each backup.
- Logs never contain tokens, keys, PINs, OAuth tokens, recovery keys or full mobile numbers.
- **Privacy (v2, the school is the data fiduciary; Vidya never sends school data to the
  developer):** consent record per student (guardian, purpose, method, recorded by, date,
  withdrawn); messages need `messages` consent. Export one student's data (JSON + PDF) and erase
  on request where the law allows (financial/audit kept; personal fields tombstoned; audited).
  Retention setting for students who left (default keep **[OWNER/legal]**; no auto-deletion
  without Principal confirmation). Incident log + 72-hour reporting reminder. No analytics SDKs.

## 10. Licence v2 (offline files)

Business model: **one-time purchase, perpetual, no expiry, no online check, no remote revoke.**
- **Machine code** on Welcome → Set up: 12 Crockford base32 chars in 3 groups + a 1-char
  checksum (e.g. `7KQ2-M9XD-4TRA-P`), derived from the OS-keychain `machine_id`
  (SHA-256 → first 60 bits → Crockford), stable across app updates.
- **Licence JSON** `{v:2, licence_id, school_name, machine_code, issued_at, plan:"perpetual",
  modules:["core", …]}` signed ed25519. Delivered as a **licence key** (base64url of
  payload‖signature, grouped by 5) **or** a `.vlic` file. The app verifies offline with the
  public key in build config.
- **Licence maker** (`tools/licence-maker/`, owner's laptop only, never shipped): a small Rust
  CLI (`init` / `issue` / `transfer` / `list`) that mints keypairs, validates the machine-code
  checksum, writes the licence key + `.vlic`, appends to `register.csv`.
- **Moving to a new PC:** Recover on the new PC → machine code → owner issues a transfer licence
  → `server_epoch + 1` → `epoch.json` updated → old PC fences itself.
- Limits: if `max_students`/`max_devices` are non-null, vidya-core enforces them
  (`LICENCE_LIMIT`); null = unlimited. Build-time config file `src-tauri/build-config/<env>.json`
  holds the licence public key, Google OAuth client ids, and (only when the Instant-sync module
  ships) an optional `relay_url`; release builds fail if a **required** value is missing.

### 10.5 Static website (`site/`)
Plain HTML/CSS (brand tokens, Welcome look): what Vidya is, platforms, honest size text,
`[PRICE]`, **how to buy** (pay the owner's UPI QR `[OWNER]`, then send UTR + school name +
machine code via a prefilled `mailto:` to support). Downloads read `releases.json` from the
release workflow. Privacy/terms/refund drafts (owner review). Host on GitHub/Cloudflare Pages.

## 11. Google accounts & Drive (v2)

Two accounts: a **school sync account** (new free Gmail; holds encrypted change files, acks, the
epoch marker, class notes, outgoing email; every staff device does its own OAuth sign-in — the
Principal types the password on each device at join, teachers never learn it) and a **backup
account** (the Principal's own; encrypted daily backups; School PC only). Staff personal Google
accounts are no longer used.

Folder layout (owned by the sync account, from the server PC):
```
Sync account's Drive: Vidya/<school name> (<school_id>)/
  exchange/ ops-<device_id>/  acks/<device_id>.json  epoch.json
  notes/<class>/<yyyy-mm>/   (homework & notes attachments, NOT encrypted)
Backup account's Drive: Vidya/<school name> (<school_id>)/backups/
```
Bundle `exchange/ops-<device_id>/<hlc>-<audience>.vop` = `nonce(12) ‖ ChaCha20-Poly1305(
audience_key, JSON array of ops)`, ≤ 500 ops or 1 MB, uploaded to a temp name then renamed.
Refresh tokens stored encrypted per device, never copied between devices.

- Desktop OAuth: installed-app PKCE flow, system browser + loopback `127.0.0.1:<port>`, `reqwest`.
- **[VERIFY — first step of Phase 12]** with one real sync account + 3 devices: `drive.file`
  scope lets every device (same account, same OAuth client) list/read/write the files other
  devices created; and the Android sign-in method works (Phase 6 spike). If `drive.file` is
  insufficient, the alternatives go to the owner before building.
- Quota full / token revoked / folder deleted → Home "Needs attention" with the exact fix.

## 12. Backups & restore

Daily at 06:00 on the school server (and on quit if none today): SQLCipher export keyed by the
backup key → verify (open, `PRAGMA integrity_check`, row counts, audit chain head) → temp name,
fsync, rename → keep in `<AppData>/Vidya/backups` → upload to the **backup account's** Drive
`backups/` (resumable > 5 MB) → verify checksum. Retention: 30 daily + 12 monthly per
destination. Home shows "Verified today at 6:02 AM · this PC and school Drive" only after
verification. Failure → Home "Needs attention".

Restore (Welcome → Recover): choose backup → recovery key → verify → summary → licence transfer
→ new DB key, new TLS cert, `server_epoch + 1` → every device `needs_rejoin` → Principal creates
PIN → Home "Devices need to join again". Devices keep unsent ops and push them after re-joining.

## 13. Allowed dependencies (complete list)

**Rust** (features as before; `cargo tree -i aws-lc-rs` must print nothing): `tauri`,
`tauri-build`, `serde`, `serde_json`, `tokio` (`rt-multi-thread,net,time,sync,macros,fs,io-util`),
`axum` (`http1,json,query,tokio,ws`), `hyper`, `hyper-util`, `tokio-rustls`, `rustls`
(`ring,std,tls12,logging`), `rcgen` (`ring,pem`), `reqwest` (`json,rustls-tls`),
`tokio-tungstenite` (rustls/ring), `futures-util` (`std,sink`), `rusqlite`
(`bundled-sqlcipher-vendored-openssl`), `uuid` (v7), `argon2`, `rand`, `sha2`, `hmac`, `base64`,
`chacha20poly1305`, `ed25519-dalek`, `thiserror`, `time`, `mdns-sd`, `keyring` (desktop),
`tracing`, `tracing-subscriber` (`fmt,std`), `qrcode` (`svg`).

**Tauri plugins**: `tauri-plugin-dialog`, `tauri-plugin-opener`, `tauri-plugin-deep-link`,
`tauri-plugin-barcode-scanner` (Android, if ≤ 2 MB). In-repo: `plugins/vidya-android`.

**JS runtime**: `react`, `react-dom`, `@tauri-apps/api`, JS halves of the plugins,
`class-variance-authority`, `clsx`, `tailwind-merge`, `@radix-ui/react-slot`,
`@radix-ui/react-dialog`, `-dropdown-menu`, `-select`, `-tabs`, `-radio-group`, `-checkbox`.
(`-popover`/`-tooltip`/`-switch` only when a screen needs it — ask first.)

**JS dev**: `vite`, `@vitejs/plugin-react`, `typescript`, `tailwindcss`, `@tailwindcss/vite`,
`vitest`, `@playwright/test`, `@tauri-apps/cli`, `@types/*`.

**Fonts** (bundled, never from the web): Geist 400/500/600/700, **Newsreader variable (opsz
axis)**, Noto Sans Devanagari 400/500/600, **`@fontsource/noto-sans-telugu` 400/500/600 (Telugu
subset)**. Latin + Devanagari + Telugu subsets only.

**v2 dependency note:** no new crates or npm packages beyond the above are needed. QR = `qrcode`;
photo compression via `vidya-android` (Android) / a webview canvas (desktop); email MIME by hand
with `base64`; share sheet via `vidya-android`; Telugu font `@fontsource/noto-sans-telugu`.
Anything else → STOP and ask.

**NOT allowed**: any animation library, lucide-react/icon packs, react-router, zustand/redux,
react-query, date-fns/dayjs/moment, chart libraries, sonner, react-hook-form, zod, i18next,
any Google SDK, any PDF library, any spreadsheet library, axum-server, aws-lc-rs, the openssl
crate (except inside rusqlite's vendored SQLCipher), Electron, analytics/telemetry SDKs.

Built by hand: hash router, state via React context + `useSyncExternalStore`, `t()` i18n, Intl
formatting, CSS bar charts, inline SVG icons, form validation mirroring vidya-core error codes,
SQL migrations, CSV reader/writer.

## 14. Modules (v2)

`module_setting` rows switch modules on/off. Defaults (§11 of 02-V2-CHANGES):
Core (always on, no key) · School accounts **on** · Classroom **on** · Staff HR **on** ·
Circulars **on** · Automatic WhatsApp **off** · School store **off** · Instant sync (relay)
**off**. Keys: `accounts, classroom, hr, circulars, wa_auto, store, instant_sync`.
Disabled modules hide their screens, reject their commands and server ops (`MODULE_OFF{module}`),
and don't sync their tables to devices. Turning a module off never deletes data; turning it on
again shows it. `vidya-core::modules::module_for(action)` maps every action to a module (core by
default); `require_module(enabled_set, action)` enforces it; every command and server endpoint
checks it via one helper.

The v2 modules and their features (built by the phases in 03-PROTOTYPE-SPEC §3):
- **Communication** (P14): per-student UPI QR (`upi://pay?…` **[VERIFY NPCI]**), free email via
  the sync account's Gmail API (school PC only, **[VERIFY gmail.send]**, daily cap default 400),
  tap-to-WhatsApp (share sheet / `wa.me`), optional automatic WhatsApp (Cloud API, school pays
  Meta), absence alerts, fee reminders, circulars & notices.
- **Fees** (P15): per-fee-head instalments; dues per instalment. Out of scope (owner): concessions,
  sibling discounts, late fees, TC/bonafide/character certificates, enquiries, photos, ID cards.
- **School accounts** (P15): expenses (voucher on save; reversal never edit), cash book, profit
  summary, salary register (deduction = monthly ÷ working days × unpaid days, half-up **[OWNER]**),
  optional School store.
- **Classroom** (P16): timetable (core rejects clashes), substitutes, homework & notes, report-card
  remarks, exams seating & hall tickets, calendar screen.
- **Staff HR** (P17): staff attendance (check-in "at school" only over LAN by default **[OWNER]**;
  late after 09:00 default), leave types/quotas **[OWNER]**, approvals, salary/substitute links.

## 15. Design tokens
Exactly the values in `docs/01-MOCK-SPEC.md §2`. No other colours may appear in `src/`
(enforced by `scripts/check-hex.mjs`).

## 16. Glossary
School server = the Principal's PC install holding the official record. Op = one recorded
change. Confirmed = accepted by the school server. Request = change to locked data needing
approval. Series = a device's receipt/admission prefix. HLC = ordering timestamp. Audience =
who may decrypt a Drive bundle. Session = April–March year. Term = part of a session. Epoch =
which PC is the school server now. Lease = how long a device works without contacting the school.
Module = a switchable feature area. Machine code = the offline-licence bind for a PC.

## 17. Open owner decisions
Tracked in **`docs/OWNER-DECISIONS.md`** (decision · default in use · where the default lives ·
status · owner's answer). Build the stated default; list every open decision in each handoff.
</content>
