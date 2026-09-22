# PHASE 4 of 10 — SCHOOL SERVER ON THE LAN, INVITATIONS, DEVICES, ROLE SCOPES, CONFLICTS

## ROLE
You are a senior Rust networking + Tauri engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies with the listed features. Anything else → STOP and ask.
   No CRDT libraries, no Firebase, no other tunnels, no axum-server.
4. Never invent features, rules, copy, numbers, API fields, library APIs or config keys.
   Check installed crate source/docs for every API. Unsure → STOP and ask.
5. The mock wins. Status copy EXACTLY as context §8.10. New screens only from existing
   components/tokens.
6. The server re-runs vidya-core for every op. Never duplicate rule logic in sync code.
7. No fake success anywhere (a status pill shows only what really happened).
8. Never delete/weaken tests. Never edit an earlier phase's migration.
9. Run every command you mention; paste real output. Separate environment vs code failures.
10. Work on branch `rebuild/p04`; small commits; no push/merge/tag unless asked.
11. Stop conditions are real.
12. Finish with the handoff file.

## OBJECTIVE
The Principal's PC becomes the school server on the school network. Staff join by link,
code or QR; phones and other PCs sync every op to it with confirmations, role-scoped data,
conflict detection and review; devices can be listed and revoked; Android keys move into
the Keystore. (Internet relay = Phase 5, Drive = Phase 6.)

## DONE MEANS
1. Phone on school Wi-Fi submits attendance while the server is off → turns server on →
   phone shows "Confirmed by school server" within 15 s.
2. Two phones edit the same student's address from the same version → one conflict on
   Principal Home ("… · address changed on two phones") → Principal resolves in Conflict
   review → both phones get the resolved value.
3. Revoked phone gets `DEVICE_REVOKED_OR_UNKNOWN` and shows "This phone was removed from
   <school>".
4. Teacher device database contains NO fee rows and no students outside their classes
   (checked by a test that opens the client DB).
5. Integration harness (1 server + 3 clients) green.

## STEPS

### Step 1 — Protocol types (`src-tauri/src/sync/protocol.rs`)
Serde types for every request/response below, with `protocol: 1` and `server_epoch` on
every response. Round-trip tests.

### Step 2 — TLS and server shell (`src-tauri/src/server/`)
- Certificate: `rcgen` self-signed, 10 years, created at setup, private key stored in the
  encrypted DB. Fingerprint = SHA-256 of the DER cert.
- Serve with `tokio-rustls` + `hyper-util` + `axum` (follow axum's official low-level
  rustls example for the accept loop). Bind `0.0.0.0:47650`, fall back 47651–47659,
  advertise the actual port **[OWNER: port default]**.
- Client verifier: a custom rustls `ServerCertVerifier` that accepts ONLY the pinned
  fingerprint (test: a different cert is rejected).
- Limits: body ≤ 5 MB; 30 requests / 10 s / device (429); bad or missing token → 401
  `DEVICE_REVOKED_OR_UNKNOWN`; protocol mismatch → 426; epoch mismatch → 409 `EPOCH_OLD`.
- Server runs in a background task started when the app starts in Server mode, even if the
  main window is closed to the tray/menu bar **[VERIFY how Tauri 2 keeps the process
  alive with the window hidden on Windows and macOS; document]**.

### Step 3 — Endpoints (`/v1`, JSON, `Authorization: Bearer <device token>` except join)
- `GET /hello` → `{school_id, school_name, server_time, protocol, server_epoch}`
- `POST /join {invite_code, device_name, platform, google_email?}` →
  `{device_id, device_token, staff, receipt_series, admission_series, audience_keys[],
  session_key, school, lease_expires_at, bootstrap_cursor}`
- `POST /sync/push {ops[≤200]}` → `{results[{op_id, status confirmed|rejected|conflict|
  flagged, server_seq?, record?, reason_code?}], server_time}`
- `GET /sync/pull?since=<cursor>&limit=500` → `{changes[], deletes[], next_cursor,
  has_more, server_time, lease_expires_at}`
- `POST /sync/snapshot` → streamed newline-delimited JSON of rows the role may see +
  cursor (resumable by table + last id)
- `POST /device/heartbeat {device_time, pending_count, pending_payment_paise}` →
  `{server_time, revoked, lease_expires_at}`
Applying an op = load the op's staff member's CURRENT actor → vidya-core decision →
`with_write` + `op_log` + `applied_ops` (idempotency by op_id) + audit. Payments use
`apply_synced_payment` (never rejected for amount). Revoked/suspended author →
`review_flag revoked_author`, status `flagged`.

### Step 4 — Scopes (`src-tauri/src/sync/scope.rs`, server-enforced)
Teacher → school, session/terms, own classes and class-subjects, their enrollments and
students (NO fee tables/columns; guardian address only if class teacher), their sheets,
exams for their subjects, own requests, staff names. Accountant → school, sessions,
classes, all students, all fee data, own requests, staff names; no attendance marks, no
marks. Principal on another device → everything. Losing an assignment → `deletes[]` on
next pull. Tests: per role, per table, assert exact visible rows AND columns.

### Step 5 — Invitations + join
- Staff & access screen (desktop, derived from Collect-fee table + Sheet): staff list with
  role pill, state, class/subject assignments (select controls, never free text), Add staff
  (name, role, mobile, Google email optional, assignments), Suspend, Remove, Resend invite,
  Revoke invite, effective-access preview (plain list: "Can take attendance for V-A",
  "Can enter marks: VI-B Maths"). Principal cannot create a second Principal here.
- Invite: 8-char Crockford code (hash stored), single use, 72 h expiry, revocable; QR (SVG
  via `qrcode`) + link `vidya://join?d=<base64url JSON {school_id, school_name, lan_addrs[],
  port, cert_sha256, relay_url, code}>`. Deep link handled by `tauri-plugin-deep-link`
  (payload ≤ 2 KB, schema-validated).
- Join flow (Welcome → Join my school, desktop + phone): paste link/code or scan QR (only
  if the scanner plugin survived the Phase 1 spike) → reach server (LAN now; relay in
  Phase 5) → show school name + first 8 fingerprint chars (also shown on the Principal's
  invite screen for code-only joins) → confirm → `/join` → create PIN → snapshot with
  progress → role home.
- Device series: A1 = server; A2, A3 … never reused.

### Step 6 — Client sync engine (`src-tauri/src/sync/engine.rs`)
Loop every 15 s while unlocked, immediately after each write, on resume and on network
change; exponential backoff 15 s → 5 min; routes: LAN (last address, then mDNS
`_vidya._tcp.local` filtered by school_id) → [relay: Phase 5] → [Drive: Phase 6] → queue.
Outbox sent in HLC order. Results: confirmed → replace with canonical record, `confirmed`;
rejected → `rejected` + Inbox item with reason copy + "Edit & resend" / "Discard";
conflict → "Waiting for the Principal to review"; flagged → "Sent to the Principal for a
check". Pull applies changes; local unsynced edits stay on top until acknowledged. Lease
refreshed on every contact; expired lease → context §8.8 behaviour. Server epoch lower than
known → refuse and show an error.

### Step 7 — mDNS
Server advertises `_vidya._tcp.local` with TXT `school_id`, `port`, `fp` (first 16 hex of
the fingerprint). Clients filter by school_id (two schools on one Wi-Fi). If the Phase 1
spike dropped `mdns-sd`, use last-known IP + manual address only and say so.

### Step 8 — Conflict review (Principal, derived)
List of open conflicts (Home "Needs attention" links here) + detail: field, value A and
value B side by side, who, which device, when (HLC order), buttons "Keep A", "Keep B",
"Edit". Resolution is a normal op. Never auto-resolved. Review flags (excess payment,
possible duplicate, revoked author) listed in the same screen with their own actions
(e.g. "Request reversal", "Mark as checked").

### Step 9 — Sync & devices screen
Server view: status, LAN addresses, port, fingerprint, devices (owner, platform, series,
last seen — labelled "last seen", pending counts reported by heartbeat), Revoke, "Replace
device" (creates a `device_replacement` request flow for staff; Principal can do it
directly). Client view (desktop + phone Sync tile): current route, last confirmed sync,
pending count and ₹ waiting, Sync now, manual server address. Revocation kills the token
immediately on the server and rotates affected audience keys (for Phase 6 bundles).

### Step 10 — `plugins/vidya-android` (Kotlin)
Create with the Tauri plugin template (`npx tauri plugin new vidya-android --android`,
verify flags with `--help`), inside `src-tauri/plugins/`. Implement Keystore: AES-GCM key
in AndroidKeyStore (non-exportable) wraps/unwraps the 32-byte DB key and other secrets.
Migrate the Phase 2 TEMP file (unwrap old → rewrap → securely delete file). Test on a real
device/emulator.

### Step 11 — Live updates
Server emits Tauri event `sync://changed` after applying ops; clients emit it after pulls;
screens subscribe and refresh their data without reload (no flicker, animations don't
replay).

### Step 12 — Integration harness
`src-tauri/tests/sync_e2e.rs`: in-process server + 3 clients (Principal PC, Accountant PC,
Teacher phone profile) with a controllable network (drop, delay, duplicate, reorder),
scripted scenarios: offline work then reconnect; same op sent twice; conflicting edits;
payments on two clients exceeding dues (both confirmed, excess flag); teacher removed from
class while holding a draft; device revoked while offline; 5,000 queued ops after a long
outage. Assert final data equal on server and on every client within that client's scope.

## EDGE CASES (test each)
Server asleep for days → no loss · DHCP address change → rediscovery · op for a deleted/left
student → rejected `NOT_FOUND` with clear copy · clock skew > 10 min → banner, HLC still
orders · snapshot interrupted → resumes, DB swap atomic · guest Wi-Fi with client isolation
→ LAN unreachable (Phase 5 relay covers it) · Windows firewall blocks the port → help text
in Sync & devices (installer rule in Phase 9) · curl without token → 401.

## STOP CONDITIONS
Keeping the server alive with the window closed is impossible on a platform → report
options. Scanner plugin missing and QR is required → ask.

## HANDOFF → `docs/phase-notes/phase-4.md`
Protocol reference · scope table per role · conflict rules as implemented · harness
instructions + results · network troubleshooting notes · sizes · questions.
