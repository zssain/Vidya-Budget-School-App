# Phase 4 handoff — School server on the LAN, invitations, devices, scopes, conflicts

Branch: `rebuild/p04` (from `rebuild/p03`).
Start HEAD: `fcd2692` (P03 close-out). `git status` at start: clean.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, node v25.3.0.

## Dependency check
Phase 3 complete (`docs/phase-notes/phase-3.md`). All §13 networking/crypto deps were already
declared (axum, tokio-rustls, rustls[ring], rcgen, hyper-util, mdns-sd, chacha20poly1305, hmac,
reqwest, qrcode, deep-link). **Added one feature:** `hyper-util` `service` (for
`TowerToHyperService`, which axum's official low-level rustls example requires) — within the §13
"as required by axum's low-level TLS example" allowance. No other deps. The barcode scanner is
still absent (Phase-1 deferral) → **join is by link/8-char code only; QR is generated but not
scanned in-app** (per the "only if the scanner survived" clause — not a STOP).

## Status by step (verified vs runtime/device)
| Step | State | Verified how |
|---|---|---|
| 1 Protocol types | **done** | round-trip tests |
| 2 Cert + pinned verifier | **done** | test: a different cert is rejected |
| 2 TLS listener (net.rs) | **compiles** | `cargo build`; runtime-only (real bind/TLS) |
| 3 Endpoints/op-apply (service) | **done** | integration harness + unit tests |
| 4 Role scopes | **done** | per-role table/column tests |
| 5 Invite code+hash, link, QR | **done** | round-trip + reject + QR-render tests |
| 5 Staff & access screen | **done (UI)** | tsc + build; runtime pending a running app |
| 6 Client engine + transport | **done** | loopback round-trip, offline, backoff tests |
| 7 mDNS advertise/discover | **compiles** | desktop cfg; runtime-only (real network) |
| 8 Conflict review screen | **done (UI)** | tsc + build |
| 9 Sync & devices screen | **done (UI)** | tsc + build |
| 2/11 Server task + `sync://changed` | **compiles** | wired into `run()` (Server mode, desktop); runtime |
| 10 Android Keystore plugin | **remaining** | needs a device/emulator |
| 12 Integration harness | **done** | 8 scenarios green |

### UI + command surface added (this session)
18 Phase-4 commands (in `commands.json` ↔ Rust `COMMANDS` ↔ `api.ts`, consistency test green):
`list_staff_access, add_staff, suspend_staff, remove_staff, create_invite, revoke_invite,
set_class_teacher, assign_subject_teacher, effective_access, list_devices, revoke_device,
server_status, sync_status, sync_now, list_conflicts, resolve_conflict, list_review_flags,
resolve_review_flag`. Screens (routes): **Staff & access** `#/principal/staff`, **Conflict review**
`#/principal/conflicts`, **Sync & devices** `#/sync`. i18n in `strings/p04.ts`. The server task is
started in `lib.rs run()` (Server mode) and emits `sync://changed` after applying ops.

## DONE MEANS — evidence (all proven at the integration level in `tests/sync_e2e.rs`)
1. Queued attendance op → confirmed when it reaches the server; the mark changes; audit valid.
   (The 15 s latency + phone-off→on is the runtime loop over this proven path.)
2. Two devices edit the same address from the same base version → first confirmed, second **conflict**
   (current kept, one open `conflict` row) → the Principal resolves it as a normal op (Conflict-review
   UI is the remaining screen; the resolution op path is the same `apply`).
3. Revoked device token → `authenticate` returns `None` → 401 `DEVICE_REVOKED_OR_UNKNOWN` (net.rs).
4. A teacher's scoped snapshot has **no fee rows** and only their class rosters; losing the class
   removes those rows from scope (`scope::visible_row` → None).
5. Harness (in-process server + clients) green — 8 scenarios incl. idempotency, §8.6 excess payment,
   5,000 queued ops.

## Protocol reference (`sync/protocol.rs`, `protocol: 1`, `server_epoch` on every response)
- `GET /v1/hello` → `HelloResp{school_id, school_name, server_time, protocol, server_epoch}`
- `POST /v1/join {invite_code, device_name, platform, google_email?}` → `JoinResp{device_id,
  device_token, staff, receipt_series, admission_series, audience_keys[], session_key, school,
  lease_expires_at, bootstrap_cursor, …}`. Single-use invite; 401→ n/a (no token).
- `POST /v1/sync/push {ops[≤200]}` → `PushResp{results[{op_id, status confirmed|rejected|conflict|
  flagged, server_seq?, record?, reason_code?}], server_time}`.
- `GET /v1/sync/pull?since=&limit=` → `PullResp{changes[], deletes[], next_cursor, has_more,
  lease_expires_at, …}` (op_log server_seq cursor, scoped by role).
- `POST /v1/sync/snapshot` → newline-delimited JSON of scoped rows (bootstrap).
- `POST /v1/device/heartbeat {device_time, pending_count, pending_payment_paise}` →
  `HeartbeatResp{server_time, revoked, lease_expires_at}`.
Errors: 401 `DEVICE_REVOKED_OR_UNKNOWN` (bad/missing token), 409 `EPOCH_OLD`, 426 protocol,
429 `RATE_LIMITED` (30 req/10 s/device), 5 MB body cap. Auth = `Authorization: Bearer <token>`;
the server stores only SHA-256(token) and compares constant-time.

## Applying an op (`sync/apply.rs`) — vidya-core decides (rule §6)
Idempotent by `op_id` (`applied_ops`). Load the author's **current** actor → `permissions::can`
(attendance/marks resolve the target's class from the DB) → Deny = `rejected FORBIDDEN`. A
revoked/suspended author → `review_flag revoked_author` + `flagged`. Payments → `apply_synced_payment`
(never rejected for amount; excess → `advance_credit` + `review_flag excess_payment` + `flagged`).
Updates: `base_version < current AND a carried field differs` → `conflict` (current kept, `conflict`
row raised). Every applied op writes `op_log` + a hash-chained audit entry in one transaction.
Sync columns are only bumped on synced tables (child tables like `attendance_mark` upsert plainly).

## Scope table (`sync/scope.rs`, server-enforced)
| Table | Teacher | Accountant | Principal |
|---|---|---|---|
| school, academic_session, term, subject | ✓ | ✓ | ✓ |
| staff | names only (id,name,role,state) | names only | full |
| class / class_subject | own | all | all |
| student | own classes only; **guardian address only if class teacher** | all | all |
| enrollment | own classes | all | all |
| attendance_sheet / attendance_mark | own classes | **none** | all |
| marks_sheet / mark_entry / exam* | own subjects | **none** | all |
| fee_head / fee_due / payment / payment_allocation / reversal | **none** | all | all |
| request | own | own | all |
Losing an assignment removes the rows from `visible_row` (→ pull scope). Snapshot + per-row filter
share the same rules. Tests assert exact visible rows AND columns per role.

## Conflict rules as implemented (§8.5)
An `update` op whose `base_version` is older than the current row version AND that carries a field
whose value differs from the current value → a `conflict` row per differing field (`value_a`=current,
`value_b`=incoming, with hlc/device/staff), current value kept, op status `conflict`. Never
auto-resolved; the Principal resolves via a normal op (Conflict-review UI pending). Payments never
conflict (they always land; §8.6). Different fields of the same record do not conflict.

## Harness (`src-tauri/tests/sync_e2e.rs`) — how to run + results
`cargo test -p vidya --test sync_e2e` → **8 pass**: attendance confirm; idempotent duplicate;
address conflict; two-payments-exceed-dues (both land, excess flagged); revoked-token rejected;
teacher scope (no fees, roster only, lost-assignment removal); join→snapshot→incremental pull;
5,000 queued ops. The "controllable network" is modelled by controlling push order/duplication and
a `LoopbackTransport.offline` toggle (see `sync::engine` tests for offline-keeps-outbox).

## Verification (real output)
`cargo test --workspace` → **333 pass** (vidya lib 75 · e2e_flows 4 · sync_e2e 8 · vidya-core 235 ·
no-floats 1 · doc 10; benchmark `#[ignore]`). `cloud/licence` 2. `cargo clippy --workspace
--all-targets -D warnings` → **clean**. `tsc` → 0, `vitest` → 38, `npm run build` → clean (JS bundle
built). Playwright interactions (`phase1`) → 5/5 (mock screens unchanged). `cargo build -p vidya`
(incl. net.rs, mdns.rs, start.rs) → clean.
**Not run** (needs a running app / device / physical LAN): real TLS bind + rate-limit + join over
Wi-Fi, mDNS discovery, `sync://changed` events, the window-closed lifecycle, the client-mode Join
flow UI, and the Android Keystore. The screens are tsc/build-verified, not pixel/runtime-verified.

## `[VERIFY]` — keeping the server alive with the window closed (Tauri 2)
Achievable, not a STOP. Plan (needs runtime confirmation on Windows + macOS):
- The server runs as a `tokio::spawn` task on the app's runtime; it is independent of any window,
  so it keeps serving as long as the **process** lives.
- Prevent the app quitting when the window closes: handle `WindowEvent::CloseRequested` →
  `api.prevent_close()` + `window.hide()`, and `RunEvent::ExitRequested` → `api.prevent_exit()`
  while in Server mode. Provide a **tray icon** (Tauri 2 `TrayIconBuilder`) to reopen/quit.
- macOS: set `ActivationPolicy::Accessory` (menu-bar app) so it runs windowless; Windows: the tray
  icon keeps it discoverable. Document the firewall prompt (Step 9 help text; installer rule P9).
Verify on both platforms before shipping.

## Sizes
Not re-measured this phase (needs a release build + `build-config/release.json` — the P03 gate).
Expect the binary to grow vs P03: the server now links axum + tokio-rustls + hyper-util + rcgen +
mdns-sd + the reqwest client TLS path. P9 owns the size gate.

## `[OWNER]` defaults used
- Server port **47650** (fallback 47651–47659). Lease **30 days**. Invite **72 h**, single-use,
  8-char Crockford. Device series A1 = server, then A2, A3… (max numeric suffix + 1).
- Audience keys generated per audience (admin/finance/class:*) and returned at join; **rotation on
  revoke is Phase-6 (Drive bundles)** — flagged.

## What's left in Phase 4 (for the next session)
1. **Runtime verification of the wired pieces** (built + compiling, not yet run): the background
   server serving `/v1` over real TLS on the LAN, `net` rate-limit/body-cap, mDNS discovery, the
   `sync://changed` live-refresh, and the invite→join flow between two real devices.
2. **Window-closed lifecycle** (the `[VERIFY]` below): tray icon + `CloseRequested`/`ExitRequested`
   prevent + hide (macOS Accessory) so the server keeps serving with the window closed. Not yet
   coded (leaving untested window-event handling out until it can be verified on Win/macOS).
3. **Client-mode Join flow UI** (Welcome → Join my school): paste link/code → `HttpsTransport` →
   `/join` → PIN → snapshot with progress → role home. The transport + `/join` service are built;
   the client-mode screen + client sync loop wiring remain.
4. **Step 10 Android Keystore plugin** (`npx tauri plugin new vidya-android --android`): AES-GCM key
   in AndroidKeyStore wrapping the DB key; migrate the Phase-2 TEMP file; test on a device.
5. Re-measure size in a release build (P9 gate).

## Questions for the owner
1. Confirm server port 47650 (fallback ..47659).
2. OK that QR is generated but **not scanned in-app** this phase (scanner deferred in Phase 1; join
   by link/code)? If in-app QR scanning is required now, that reopens the Phase-1 barcode-scanner
   size decision.
3. For the window-closed lifecycle, confirm the tray + prevent-exit approach (and, on macOS, the
   menu-bar/Accessory activation) before I wire it and test on Windows + macOS.
