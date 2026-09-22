# VIDYA BUDGET SCHOOL — SYSTEM CONTEXT

This file is the source of truth for every phase. Every phase prompt tells the agent to read
it first, in full.

- If code and this file disagree, this file wins.
- If this file is silent on something, STOP AND ASK the owner. Never invent features, APIs,
  libraries, numbers, legal wording or UI copy.
- Items marked **[OWNER]** are open business decisions. Build the stated default, keep it in
  one config place, and list it in the phase handoff.
- Items marked **[VERIFY]** are technical facts that must be proven with a real test before
  building on them.

Companion files (read them too):
- `docs/01-MOCK-SPEC.md` — every token, animation, asset and interaction of the mock.
- `design/screens/*.dc.html` — the five mock screens (the visual contract).
- `design/assets/*.svg`, `design/brand-kit/` — the real logo and app icons.

---

## 1. Product in one paragraph

School-management software for small Indian schools (about 200–1,500 students, 5–60 staff).
A school buys a licence on the company website and receives an activation code. The
Principal installs Vidya on the school's main PC, activates it and sets up the school. That
PC becomes the **school server** holding the official record. Staff join by invitation.
Teachers take attendance and enter marks for their own classes; accountants handle
admissions and fees; the Principal sees everything and approves corrections. Phones and PCs
reach the school server on the school LAN or over the internet through the Vidya relay. If
the school server is off, devices exchange encrypted changes through the school's Google
Drive. With no internet at all, work is saved on the device and sent later. Nothing is ever
silently overwritten or lost. English and Hindi. Windows, macOS, Android.

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
- CI fails the release if any limit is exceeded.

Company-side services (in this repo, never shipped inside the apps):
- `cloud/relay` — internet relay for reaching school servers.
- `cloud/licence` — licence service: purchase webhook, licence register, activation API,
  company admin panel (Phase 10). `cloud/licence` in `--dev` mode replaces the old
  licence-dev server for development.

## 3. Non-negotiable rules

1. **The UI is the mock.** `design/screens/*.dc.html` define colours, fonts, font sizes,
   weights, letter-spacing, line-heights, spacing, radii, borders, shadows, copy, icon SVG
   paths, animations and interactions. The built screens must be visually indistinguishable
   from the mock at the mock's size (checked by screenshot comparison, see 01-MOCK-SPEC §9).
   shadcn/ui and Tailwind defaults NEVER win over the mock. New screens not in the mock are
   built only from the components and tokens extracted from the mock.
2. **Offline-first.** Every feature works with no network. Nothing is lost offline.
3. **Minimal dependencies.** Only the packages in §13. Anything else → stop and ask.
4. **Encrypted everywhere.** Encrypted at rest on every device; encrypted before anything
   reaches Google Drive or the relay. Google, the relay operator and the company can never
   read school data.
5. **Money is permanent.** Payments are never edited or deleted. Mistakes are fixed by an
   approved reversal. Money actually received is never rejected by sync (§8.6).
6. **Locks.** Submitted attendance sheets and submitted marks sheets (per exam subject) are
   locked. Changes go through a correction request approved by the Principal.
7. **Audit.** Append-only, hash-chained, tamper-evident. Nobody can edit it, not even the
   Principal.
8. **Conflicts are flagged, never silently overwritten.**
9. **Rules live in `crates/vidya-core`** — pure Rust, no IO. The UI and the sync code never
   decide business rules; the server re-checks every change with vidya-core.
10. **Indian formats.** ₹ with lakh grouping (₹6,84,200). Dates "Wednesday, 23 September".
    Session April–March shown "2026–27" (en dash). Mobiles: 10 digits starting 6–9.
11. **English + Hindi everywhere**, including receipts, report cards and reports.
12. **Size and speed.** Limits in §2. Must run on a 4 GB RAM Windows PC and a 2 GB RAM
    Android phone.
13. **Honest status.** The UI never shows success for something that did not happen
    (no fake "synced", "backed up", "printed", "sent").

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
  Keystore key wrapping, printing, Google sign-in bridge if needed.
- Networking: `axum` (server), `tokio-rustls` + `hyper-util` (TLS serving), `reqwest`
  (client), all TLS through **rustls with the `ring` provider only**.
- Google Drive REST v3 via `reqwest`. No Google SDK.

### 4.1 Existing repository
The owner has an existing repo (`zssain/Vidya-Budget-School-App`). Phase 1 step 0 decides:
- **If the repo exists**: keep git history; move the old plain-JS frontend to
  `legacy/` (not built, not shipped); **port** `crates/vidya-core` modules that already
  exist (money, fees, marks, dates, validation, hlc, permissions, roles) with their tests,
  adapting them to this file. Do not rewrite working, tested rules from scratch.
- **If starting fresh**: create the layout in §14.
Record which path was taken in the Phase 1 handoff.

## 5. Roles and permissions

Enforced in `vidya-core::permissions::can(actor, action, target)`, checked in every Tauri
command, again on the school server for every op, and when building every data scope.

**Principal** (owner): everything. Approves / rejects / returns requests. Invites, suspends
and removes staff and devices. Settings, licence, Google Drive, backups, restore, session
rollover. Direct edits to locked data are allowed but always audited with a reason.

**Accountant**: admissions (create students, enroll, transfer section), student details
edits → request, fee dues view, record payments, print/share receipts, reversal → request,
day book, fee reports, student CSV import/export (no marks/attendance columns). No marks.
No attendance. No staff management.

**Teacher**: attendance for classes where they are class teacher **[OWNER: or also
"attendance duty" assignment — default class teacher only]**; marks for their own
class-subjects; view their students (no fee data at all; guardian address only if class
teacher); report cards for their classes (view/print/share); corrections → request.

**Everyone**: own profile, PIN, language, own requests, inbox, Sync screen.

Staff states: `invited`, `active`, `suspended` (cannot sign in or sync; data kept),
`removed`. Only the Principal changes states.

## 6. Screens

### 6.1 In the mock (build pixel-exact from `design/screens`)
1. **Welcome · Set up or join** (desktop 1440×960; phone: single column, navy panel hidden).
2. **Principal · Home** (desktop 1440×1080).
3. **Accountant · Collect fee** (desktop 1440×1080, sheet over the student list).
4. **Teacher · Home** (Android 390×844, 3×3 tiles).
5. **Teacher · Attendance** (Android 390×844).

### 6.2 Not in the mock (build from the same components + tokens only)
Setup wizard · PIN create / unlock / user switch · Recovery key show + confirm · Join flow ·
Approvals list + detail · Students list / profile / admission / transfer / leave · Student
import (CSV, dry run) · Attendance register (desktop day + month) · Exams · Marks entry
(desktop grid + phone list) · Report cards · Fees overview · Fee structure · Receipts ·
Day book · Reports · My requests · Inbox · Staff & access · Sync & devices · Google Drive ·
Backups · Restore · Settings (all sections) · Storage · Profile · Conflict review ·
Licence · Accountant home · Teacher phone screens for every tile.

Derivation rules for non-mock screens:
- Desktop page = mock sidebar + mock header + `PageTitle` (eyebrow + serif h1 + italic
  sub-line) + cards/tables exactly like Principal Home and Collect fee.
- Tables = the Collect fee student table pattern (10px uppercase headers, 14px 24px rows,
  top borders #E8EDEC, pills).
- Side panels / forms = the Collect fee sheet pattern (520px, radius 20, 16px inset,
  eyebrow + serif title, footer band #E2EAEB).
- Phone pages = the Attendance navy header pattern + light list body + bottom action bar.
- Success screens = the Collect fee navy success pattern.
- If a screen needs something these patterns can't express → STOP AND ASK with the
  smallest proposal. Never invent new colours, radii, shadows or font sizes.

## 7. Data model

SQLite, ids = UUIDv7 text, money = integer **paise** (i64), times = UTC ISO-8601 text,
dates = `YYYY-MM-DD`. Every synced table has: `id, version INTEGER, hlc TEXT, created_at,
updated_at, updated_by_staff, updated_by_device, sync_state` where sync_state ∈
`draft | on_device | shared_drive | confirmed | rejected | conflict`.

Tables:
- `school` (name, address, board, udise NULL, logo_blob NULL, backup_salt, server_epoch,
  settings_json)
- `licence` (licence_id, school_id, plan, issued_at, max_students NULL, max_devices NULL,
  signature, raw_json, last_check_at, status active|revoked|moved)
- `academic_session` (label "2026–27", starts_on, ends_on, is_current, read_only)
- `term` (session_id, name, starts_on, ends_on)
- `class` (name, section, display "VII-B", class_teacher_id, sort_order)
- `subject` (name, name_hi)
- `class_subject` (class_id, subject_id, teacher_id)
- `staff` (name, role principal|accountant|teacher, mobile, google_email NULL, pin_hash,
  state invited|active|suspended|removed)
- `device` (staff_id, platform windows|macos|android, name, token_hash, receipt_series,
  admission_series, last_seen_at, revoked_at, lease_expires_at, needs_rejoin)
- `invite` (code_hash, staff_id, expires_at, used_at, revoked_at)
- `student` (admission_no NULL until confirmed, provisional_no, name, dob, gender,
  guardian_name, guardian_mobile, address, transport, category NULL, rte bool,
  aadhaar_status none|submitted|verified, status active|left, left_on, left_reason)
- `enrollment` (student_id, class_id, session_id, roll_no, from_date, to_date NULL)
  — a section transfer closes one row and opens another; history is never rewritten
- `attendance_sheet` (class_id, date, status draft|submitted, submitted_by, submitted_at)
  UNIQUE(class_id, date)
- `attendance_mark` (sheet_id, student_id, mark P|A|L) UNIQUE(sheet_id, student_id)
- `exam` (session_id, term_id, name, starts_on, ends_on)
- `exam_subject` (exam_id, class_subject_id, max_marks)
- `marks_sheet` (exam_subject_id, status draft|submitted) — lock is PER SUBJECT
- `mark_entry` (sheet_id, student_id, marks INTEGER NULL, absent bool) — NULL = not entered,
  never treated as zero
- `grade_scale`, `grade_band` (min_pct, max_pct, grade, grade_point)
- `fee_head` (name, name_hi, amount_paise, frequency term|month|once, applies_to
  all|transport|class_ids_json, active)
- `fee_due` (student_id, fee_head_id, period, amount_paise, cancelled_at NULL)
- `payment` (receipt_no, student_id, amount_paise, mode cash|upi|cheque, reference,
  collected_by, collected_at, device_id, confirmed_at) — APPEND-ONLY (trigger)
- `payment_allocation` (payment_id, fee_due_id NULL, amount_paise, kind due|advance_credit)
- `reversal` (payment_id, reason, request_id, approved_by, applied_at) — APPEND-ONLY
- `request` (type, target_table, target_id, base_version, before_json, after_json, reason,
  requested_by, revision INTEGER, parent_request_id NULL, status
  pending|approved|rejected|returned|cancelled, decided_by, decided_at, note,
  apply_state not_applied|applied|failed|stale, applied_at)
  types: `marks_correction | attendance_correction | student_details | payment_reversal |
  access_change | device_replacement`
- `conflict` (table, record_id, field, value_a, hlc_a, device_a, staff_a, value_b, hlc_b,
  device_b, staff_b, status open|resolved, resolved_by, resolution_op_id)
- `review_flag` (kind excess_payment|possible_duplicate_payment|revoked_author|
  unknown_record, ref_table, ref_id, details_json, status open|resolved, resolved_by)
- `notification` (staff_id, kind, title_key, vars_json, link, read_at)
- `audit_log` (seq, at, staff_id, device_id, action, table, record_id, before_json,
  after_json, reason, op_id, prev_hash, hash) — APPEND-ONLY (trigger)
- `outbox`, `op_log` (server_seq), `applied_ops` (op_id PRIMARY KEY), `sync_cursor`,
  `drive_state`, `backup_run`, `schema_version`

## 8. Sync model

### 8.1 Ops
Every change is an **op**: `{op_id (UUIDv7), hlc, device_id, staff_id, audience, table,
record_id, kind insert|update|delete|action, payload, base_version, server_epoch}`.
HLC = `wall_ms (13 digits, zero-padded) + counter (4 digits) + device_id`; string compare =
total order across devices. HLC ordering lives in vidya-core. The HLC is for ordering and
conflict detection only — it never decides who is "right".

### 8.2 Write path (any device)
UI → Tauri command → vidya-core validates + checks permission → ONE SQLite transaction:
update row + append audit entry + append op to outbox → return DTO. Drafts
(`sync_state = draft`) never leave the device.

### 8.3 Delivery routes (automatic, in order)
1. **LAN**: HTTPS to the school server (last known address, then mDNS
   `_vidya._tcp.local`), certificate pinned by SHA-256 fingerprint.
2. **Internet**: HTTPS through the Vidya relay (works behind CGNAT), or a direct public
   address if the school configured one. Same pinned certificate; bodies end-to-end sealed.
3. **Google Drive fallback** (school server unreachable, internet available): encrypted op
   bundles uploaded to the device's own Drive folder → `shared_drive`. Devices also read
   other devices' bundles they are allowed to decrypt and show them as provisional.
4. **No network**: stay `on_device`; retry every 15 s while open, on resume and on network
   change; Drive checks every 20 s ± 3 s while the app is in the foreground (best effort in
   background; never promised).

### 8.4 School server
Re-validates every op with vidya-core as the op's staff member, using that staff member's
CURRENT permissions (revoked/suspended authors → `review_flag`, not applied). Applies ops in
HLC order, assigns `server_seq`, confirms, rejects with a reason code, or flags a conflict.
Idempotent by `op_id` (same op via LAN and Drive → applied once). On start and every 5 min
it imports pending Drive bundles in HLC order, writes ack files, archives processed bundles.

### 8.5 Conflicts
An update whose `base_version` is older than current AND touches a field changed since by
another device → keep the current value, create `conflict` rows, notify the Principal
("Riya Verma · address changed on two phones"). Different fields of the same record merge.
Attendance: two devices submit the same sheet → per-student comparison; equal marks merge,
different marks → conflict per student. Marks: same cell edited from the same base → conflict.

### 8.6 Money in sync (must never lose money)
- On the collecting device, vidya-core requires `0 < amount ≤ total due now` (the Collect
  fee screen's red error). That is the ONLY place this limit applies.
- On the server, a payment op from a device is **always accepted and confirmed** if the
  student exists and the author was allowed to collect fees when it was recorded. If it
  exceeds what is due at apply time, the excess is allocated as `advance_credit` and a
  `review_flag excess_payment` is raised for the Principal. The receipt stands.
- Possible duplicates (same student + same amount + same UPI/cheque reference, or same
  student + same amount within 10 minutes on different devices) → `review_flag
  possible_duplicate_payment`; both payments stay until the Principal requests a reversal.
- Payments never produce `conflict` rows.

### 8.7 Numbers issued offline
- **Receipts**: each device gets a series at join (A1 = school server, A2, A3 …, never
  reused). Receipts are `R-<series>-<4-digit seq>` (R-A2-0419), shown immediately,
  permanent.
- **Admissions**: a device creates the student with `provisional_no = P-<series>-<seq>`.
  The school server assigns the official `admission_no` (`YYYY/NNNN`, sequential per
  admission year) when it confirms the op. Screens show "Adm. no. pending" until then.

### 8.8 Scopes and keys
- The server sends each device only what its role may see (§5).
- Drive bundles are sealed per **audience**: `admin` (Principal devices), `finance`
  (Principal + accountants), and one `class:<class_id>` audience per class (Principal +
  teachers assigned to that class). A device receives keys only for its audiences.
  Changing assignments rotates the affected class key (new version); old bundles stay
  readable only to devices that already had the old key.
- Offline access lease: each device has `lease_expires_at` = last server contact + 30 days
  **[OWNER]**. After expiry the app still opens and keeps unsent work, but shows "Connect to
  your school to continue" and hides school data until the device reaches the school server.
- Revocation takes effect when the device next contacts the server or reads Drive; remote
  wipe of an offline phone is impossible and must not be promised.

### 8.9 Server epoch (fencing)
`school.server_epoch` starts at 1 and increases on restore to a new PC or licence transfer.
Every response carries the epoch. Devices refuse a server with a lower epoch than they know.
When the licence service reports `moved`, the old PC stops serving, becomes read-only and
shows "This computer is no longer the school server".

### 8.10 Exact status copy
"Draft saved on this phone" · "Offline · saved on this phone" · "Offline · N waiting to
send" · "Submitted, saved on this phone." · "Shared through school Drive · waiting for
school" · "Confirmed by school server" · "All changes confirmed · 12 s ago" · "Up to date ·
synced 2 min ago" · "₹2,400 more waiting for server" · "Adm. no. pending".
(Desktop wording "saved on this computer" replaces "phone" on desktop.)

## 9. Security

- DB key: 32 random bytes. Desktop → OS keychain (`keyring`, service
  `in.vidyabudget.app`, account `db-key`). Android → wrapped by an AES-GCM key in
  AndroidKeyStore via `plugins/vidya-android`.
- PIN per user, 4–6 digits, Argon2id (m=19 MiB, t=2, p=1). Auto-lock after 5 min in
  background (setting 1/5/15). 5 wrong → 30 s wait, doubling, max 1 h. The PIN is a local
  unlock, never a network credential.
- School server TLS: self-signed certificate (`rcgen`, 10 years); clients pin its SHA-256
  fingerprint received in the invitation. No public CA.
- Device auth: random 32-byte token per device; server stores only SHA-256 hash; constant-
  time compare; revocable.
- Relay: sees only sealed envelopes (ChaCha20-Poly1305 with a per-device session key agreed
  at join). It cannot read or alter them.
- Drive: bundles and backups encrypted before upload.
- Recovery key: shown once at setup, 30 Crockford base32 chars in 6 groups of 5.
  Backup key = Argon2id(recovery key, salt in `school`, m=64 MiB, t=3, p=1).
- Audit: `hash = SHA-256(prev_hash ‖ canonical JSON of entry)`; SQLite triggers
  `RAISE(ABORT)` on UPDATE/DELETE of `audit_log`, `payment`, `reversal`; chain verified
  daily and before every backup; chain head shown in Settings → About and stored with each
  backup. ADMIN-GUIDE must state honestly: a person with full control of the PC and the
  keys could rebuild the chain; backups and Drive copies make that detectable.
- Logs never contain tokens, keys, PINs, OAuth tokens, recovery keys or full mobile numbers
  (last 4 digits only).

## 10. Licence **[OWNER: API shape confirmed in Phase 10]**

Default business model: **one-time purchase, perpetual licence, no expiry.** Do not add
expiry, subscriptions, trials or prices.

API (implemented by `cloud/licence`, called by the app):
- `POST {LICENCE_API}/v1/activate {code, school_name, machine_id, app_version}` →
  `200 {licence: base64(JSON), signature: base64(ed25519)}`.
  Licence JSON = `{licence_id, school_id, plan, max_students|null, max_devices|null,
  issued_at, server_machine_id}`.
  Errors: `404 CODE_NOT_FOUND`, `409 CODE_ALREADY_USED` (used by another school),
  `200` with the same licence if the SAME machine retries (idempotent),
  `503` when the service is down ("Activation needs internet once — please try again").
- `POST /v1/transfer {licence_id, recovery_proof, new_machine_id}` → same response;
  old machine becomes `moved`. `recovery_proof` = HMAC of the request with a key derived
  from the recovery key **[VERIFY design in Phase 10 with the owner]**.
- `POST /v1/check {licence_id, machine_id}` → `{status active|revoked|moved}`.

App behaviour: verify the signature offline with the public key from build config. Check
every 30 days when online. If the service is unreachable the licence stays active
indefinitely (perpetual). Only an explicit `revoked`/`moved` answer changes status.
Limits: if `max_students`/`max_devices` are non-null, vidya-core enforces them
(`LICENCE_LIMIT`); null = unlimited.
`LICENCE_API`, `RELAY_URL`, licence public key and Google OAuth client ids come from ONE
build-time config file (`src-tauri/build-config/<env>.json`), never hard-coded elsewhere.
Release builds fail if any value is missing.

## 11. Google Drive

- **[VERIFY — Phase 6 spike, stop if false]** OAuth scope `drive.file` lets staff devices
  read files created by other users of the same OAuth client inside a folder shared with
  them. If false, the alternatives (restricted `drive` scope + Google verification, or
  Principal-account-only Drive) go to the owner before building.
- **[VERIFY — Phase 6 spike]** Android sign-in method. Google restricts custom-scheme
  redirects for Android OAuth clients. Test (a) loopback/app-link redirect, (b) Android
  Credential Manager / AuthorizationClient through `plugins/vidya-android`. Pick what
  works; report.
- Desktop: OAuth 2.0 installed-app flow with PKCE, system browser + loopback redirect
  `127.0.0.1:<random port>`, implemented with `reqwest`.
- Folder layout, created and OWNED by the Principal's Google account from the server PC:
  ```
  My Drive/Vidya/<school name> (<school_id>)/          (NOT shared)
    backups/                                            (NOT shared — Principal only)
    exchange/                                           (shared READER with all active staff)
      ops-<device_id>/                                  (shared WRITER only with that staff)
      acks/                                             (server writes; staff read)
  ```
  So a teacher can add files only to their own folder and can never delete backups or
  other people's files.
- Bundle file: `exchange/ops-<device_id>/<hlc>-<audience>.vop` =
  `nonce(12) ‖ ChaCha20-Poly1305(audience_key, JSON array of ops)`. ≤ 500 ops or 1 MB.
  Upload with a temp name then rename; never a half file.
- Acks: `exchange/acks/<device_id>.json` = `{last_hlc, results[]}` sealed with the device's
  session key.
- Tokens: refresh token stored encrypted in the local DB. Never copied between devices.
- Quota full / token revoked / folder deleted → Home "Needs attention" with the exact fix.

## 12. Backups & restore

Daily at 06:00 on the school server (and on quit if none today): SQLCipher export keyed by
the backup key → verify (open, `PRAGMA integrity_check`, row counts, audit chain head) →
temp name, fsync, rename → keep in `<AppData>/Vidya/backups` → upload to Drive `backups/`
(resumable upload > 5 MB) → verify checksum. Retention: 30 daily + 12 monthly per
destination. Home shows "Verified today at 6:02 AM · this PC and school Drive" only after
verification succeeds. Failure → Home "Needs attention".

Restore (Welcome → Recover): choose backup (file, or list from Drive after Google sign-in) →
recovery key → verify → summary (school, backup date, students, payments, last receipt
no., audit chain OK) → licence transfer → new DB key, new TLS cert, `server_epoch + 1`,
relay re-registration → every device `needs_rejoin` → Principal creates PIN → Home shows
"Devices need to join again" + "Invite all staff again". Devices keep unsent ops and push
them after re-joining (ops keep op_id + HLC; server treats them as normal ops, applying §8.4).

## 13. Allowed dependencies (complete list)

**Rust** (with these features ONLY; `cargo tree -i aws-lc-rs` must print nothing):
- `tauri`, `tauri-build`
- `serde` (derive), `serde_json`
- `tokio` (`rt-multi-thread, net, time, sync, macros, fs, io-util`) — never `full`
- `axum` (`default-features = false`, `http1, json, query, tokio, ws`)
- `hyper`, `hyper-util` (`server-auto`/`tokio` as required by axum's low-level TLS example)
- `tokio-rustls`, `rustls` (`default-features = false`, `ring, std, tls12, logging`)
- `rcgen` (`default-features = false`, `ring, pem`)
- `reqwest` (`default-features = false`, `json, rustls-tls`) — no http2, gzip, cookies
- `tokio-tungstenite` (rustls with ring) — relay tunnel only
- `rusqlite` (`bundled-sqlcipher-vendored-openssl`)
- `uuid` (`v7`), `argon2`, `rand`, `sha2`, `hmac`, `base64`, `chacha20poly1305`,
  `ed25519-dalek`, `thiserror`, `time` (`formatting, parsing, macros`)
- `mdns-sd` (desktop server + clients; may be dropped if the Phase 1 size spike says so)
- `keyring` (desktop only; platform-native features only)
- `tracing`, `tracing-subscriber` (`default-features = false`, `fmt, std`)
- `qrcode` (`default-features = false`, `svg`) — invitation QR codes
- Build-time only: `tauri-build`

**Tauri plugins**: `tauri-plugin-dialog`, `tauri-plugin-opener`, `tauri-plugin-deep-link`,
`tauri-plugin-barcode-scanner` (Android only, **kept only if the Phase 1 size spike shows
≤ 2 MB added per APK**; otherwise invites are opened by link or 8-character code).
In-repo: `plugins/vidya-android`.

**JS runtime**: `react`, `react-dom`, `@tauri-apps/api`, JS halves of the plugins above,
`class-variance-authority`, `clsx`, `tailwind-merge`, `@radix-ui/react-slot`,
`@radix-ui/react-dialog`, `@radix-ui/react-dropdown-menu`, `@radix-ui/react-select`,
`@radix-ui/react-tabs`, `@radix-ui/react-radio-group`, `@radix-ui/react-checkbox`.
(Add `@radix-ui/react-popover` / `-tooltip` / `-switch` only when a screen needs it —
ask first.)

**JS dev**: `vite`, `@vitejs/plugin-react`, `typescript`, `tailwindcss`,
`@tailwindcss/vite`, `vitest`, `@playwright/test`, `@tauri-apps/cli`, `@types/react`,
`@types/react-dom`.

**Fonts** (bundled, never loaded from the web; exact package names verified on npm in
Phase 1): Geist 400/500/600/700 (Fontsource static), **Newsreader variable with the
optical-size axis** (Fontsource variable package, `opsz` + `opsz-italic` files — required
to match the mock's big headings, see 01-MOCK-SPEC §3), Noto Sans Devanagari 400/500/600.
Latin + Devanagari subsets only.

**NOT allowed**: any animation library (motion, framer-motion), lucide-react or icon
packs, react-router, zustand/redux, react-query, date-fns/dayjs/moment, chart libraries,
sonner, react-hook-form, zod, i18next, any Google SDK, any PDF library, any spreadsheet
library, axum-server, aws-lc-rs, openssl crate (except inside rusqlite's vendored
SQLCipher), Electron, analytics/telemetry SDKs.

Built by hand: hash router, state via React context + `useSyncExternalStore`, `t()` i18n,
Intl formatting, CSS bar charts, inline SVG icons from the mock, form validation mirroring
vidya-core error codes, SQL migrations, CSV reader/writer.

## 14. Repository layout

```
vidya/
  docs/00-SYSTEM-CONTEXT.md, 01-MOCK-SPEC.md, phase-notes/, INSTALL.md, ADMIN-GUIDE.md,
       RELEASE.md, CHANGELOG.md
  design/screens/*.dc.html, design/assets/*.svg, design/brand-kit/, design/reference/*.png
  crates/vidya-core/src/{lib,money,fees,grades,permissions,attendance,marks,requests,
        conflicts,hlc,receipts,admissions,licence,validation,words,csv,errors}.rs
  src-tauri/src/{main.rs,lib.rs,config.rs,state.rs,commands/,db/{mod.rs,migrations/},
        server/,sync/{engine,lan,relay,drive,outbox,apply,scope}.rs,security/,backup/,
        licence/,print/}
  src-tauri/plugins/vidya-android/
  src-tauri/build-config/{dev.json,release.json.example}
  src/ main.tsx, App.tsx, styles/{tokens.css,keyframes.css,app.css},
       lib/{router,api,format,theme,icons,platform,store}.ts, lib/i18n/{index.ts,en.json,hi.json},
       components/ui/ (shadcn, restyled), components/ (see 01-MOCK-SPEC §7),
       screens/{desktop,phone,shared}/, dev/{Gallery.tsx,fixtures/}
  cloud/relay/, cloud/licence/
  scripts/{set-version.mjs,size-report.mjs,check-deps.mjs,check-hex.mjs,check-i18n.mjs}
  tests/e2e/, .github/workflows/{ci.yml,release.yml}
  legacy/ (old frontend, if the repo existed; never built)
```

## 15. Design tokens

Exactly the values in `docs/01-MOCK-SPEC.md §2`. No other colours may appear in `src/`
(enforced by `scripts/check-hex.mjs`).

## 16. Glossary

School server = the Principal's PC install holding the official record. Op = one recorded
change. Confirmed = accepted by the school server. Request = change to locked data needing
approval. Series = a device's receipt/admission prefix. HLC = ordering timestamp. Audience
= who may decrypt a Drive bundle. Session = April–March year. Term = part of a session.
Epoch = which PC is the school server now. Lease = how long a device works without
contacting the school.

## 17. Open owner decisions (build the default; list in every handoff)

1. Licence backend: does the website already exist? (Default: build `cloud/licence`.)
2. Payment provider for purchases (Default: provider adapter + test provider; Razorpay
   adapter only after the owner confirms and supplies test keys.)
3. Relay host + domain (Default: Docker image; `RELAY_URL` from config.)
4. Default grading scale / board (Default in Phase 2, editable in Settings.)
5. Does every staff member have a Google account? (Without one, Drive fallback is off for
   that device, with a notice.)
6. Code signing: Windows certificate, Apple Developer ID, Android keystore.
7. Offline access lease length (Default 30 days.)
8. Attendance duty for non-class-teachers (Default: class teacher only.)
9. Whether Leave counts as absent in attendance % (Default: % = P ÷ (P + A + L).)
10. Company name, support contact and legal text for receipts/website (Default:
    placeholders "[Company name]" — never invented.)
