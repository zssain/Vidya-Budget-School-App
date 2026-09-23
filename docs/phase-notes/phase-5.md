# Phase 5 handoff — Internet access through the relay, sealed traffic, server fencing

Branch: `rebuild/p05` (from `rebuild/p04`).
Start HEAD: `50afdc4` (P04 close-out). `git status` at start: clean.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, node v25.3.0, Docker 29.1.5 (daemon not running — see Verification).

## Owner decision taken this phase (the STOP)
**Relay auth / §10 contract change.** The prompt made this a STOP ("ask the owner before
changing the contract"). Owner delegated the choice ("choose the best option"); I took the
prompt default with the simpler of the two verification options:
- `cloud/licence` `/v1/activate` now returns **`relay_secret = base64(HMAC-SHA256(RELAY_SHARED_KEY,
  school_id))`** (§10 updated).
- The **relay recomputes the same HMAC** to verify a tunnel, so it never calls `cloud/licence`
  and keeps no state (stateless relay — a Step 1 requirement). No `/v1/relay-auth` endpoint.
- `RELAY_SHARED_KEY` is a server-side secret shared ONLY between `cloud/licence` and
  `cloud/relay` (env). It is never in the app or on a device. Dev default matches on both
  services so `--dev` works out of the box.

## Dependency note (please confirm — §13 companion)
Added **`futures-util`** (`default-features=false, features=["std","sink"]`) to `src-tauri`.
It is a *required companion* of the §13-listed `tokio-tungstenite`: its `WebSocketStream` is a
`Stream + Sink` and cannot be driven without the futures extension traits. It was already in
the tree transitively (via tokio-tungstenite / hyper-util), so there is **zero new build cost**.
This mirrors P04's documented `hyper-util` `service`-feature addition ("as required by the
listed crate's usage"). Crypto gate is unaffected — `cargo tree -i aws-lc-rs` still prints
nothing; `ring` is the sole TLS provider. Flag for owner sign-off; trivially swappable.

The two `cloud/*` services are separate workspaces (never shipped in the app); their deps are
independent of §13. `cloud/relay` adds `axum[ws]`, `hmac`, `sha2` (+ dev-only `reqwest`,
`tokio-tungstenite`, `futures-util` for its tests). `cloud/licence` gained `hmac`, `sha2`.

## Status by step (verified vs runtime/deploy-pending)
| Step | State | Verified how |
|---|---|---|
| 1 Relay service (`cloud/relay`) | **done** | 7 tests: forward round-trip, 503 offline, bad-secret, epoch replacement, safe log, recipe |
| 1 School tunnel client | **done (unit) / runtime** | `process_frame` unit-tested; full socket path in `relay_e2e`; live wss reconnect against a hosted relay is deploy-pending |
| 2 End-to-end sealing | **done** | `sync::seal` (8 tests) + `server::sealed` dispatch (5) + `sync::relay` (3) |
| 2 Session-key persistence | **done** | migration 0003 + join stores it; sealed dispatch uses it |
| 2 Replay counter | **done** | monotonic per-device counter; replay rejected (dispatch + harness) |
| 3 Client route order LAN→relay→queue | **done** | `engine::Route` + `sync_once_routed` (fallthrough + label tests) |
| 3 Sync-screen route label | **backend done / UI pending** | `ROUTE_LAN`/`ROUTE_RELAY` + `app_kv.last_route`; React display is a remaining UI wire-up |
| 4 Optional direct public address | **not built (UI)** | see "What's left" — off-by-default Settings field + a `Route::PublicUrl(HttpsTransport)` slot |
| 5 Epoch fencing (client refuses lower) | **done** | `engine` fencing + relay sealer refusal + sealed-dispatch server fence |
| 5 `moved` → stop serving + read-only | **done (mechanism) / runtime** | `server_stop` Notify halts LAN+tunnel; `app_state` fires it on Moved; startup recheck persists moved/revoked |
| 5 One tunnel per school (newer epoch wins) | **done** | `cloud/relay` `newer_epoch_replaces_older_tunnel` |
| 6 In-process relay harness | **done** | `tests/relay_e2e.rs` — 5 socket-level scenarios |

## DONE MEANS — evidence
1. **Phone reachable only via relay records → "Confirmed by school server".**
   `relay_e2e::phone_via_relay_records_op_confirmed` drives the REAL client transport → an
   in-process relay → the REAL school tunnel client → `sealed::dispatch` → apply, over
   sockets. Asserts `OpStatus::Confirmed` + the exact copy "Confirmed by school server".
2. **Relay logs carry only ids/sizes/timings/status — no bodies/tokens/names/numbers.**
   `cloud/relay` builds a single body-free access line (`school method path req_bytes status
   resp_bytes ms`); `access_log_carries_only_safe_fields` asserts it excludes bodies/tokens/
   names. Bodies are sealed, so the relay physically cannot log plaintext.
3. **Tamper one byte → rejected (AEAD), no data change.** `seal::tampering_one_byte_is_rejected`,
   `server::sealed::tampered_envelope_is_rejected_and_changes_nothing`, and over the wire
   `relay_e2e::tampered_frame_in_transit_is_rejected` (HTTP 400, nothing applied).
4. **Relay down → queue; back → resumes.** `relay_e2e::relay_offline_then_online_resumes`
   (503 SCHOOL_OFFLINE → `Unreachable` → queue; tunnel up → confirmed). `engine`
   `all_routes_unreachable_keeps_the_queue`. (The Drive route is Phase 6.)
5. **Epoch test.** Lower-epoch server refused by devices: `engine::a_lower_epoch_server_is_refused`,
   `relay::client_refuses_a_lower_epoch_response`, `sealed::device_with_newer_epoch_fences_a_stale_server`.
   `moved` PC stops serving: `server_stop` wired through `net::serve` + `tunnel`, fired from
   `app_state`/startup recheck; `licence::persist_check` tested.

## Relay protocol
- **School → relay tunnel:** `GET RELAY_URL/tunnel/<school_id>?epoch=<n>` upgraded to a
  WebSocket, `Authorization: Bearer <relay_secret>`. One tunnel per school; a newer epoch
  replaces the older (older closed with `4409 EPOCH_OLD`); a stale (lower-epoch) connect is
  refused `409 EPOCH_OLD`. Idle ping every 25 s; school answers pings; reconnect backoff 1→30 s.
- **Device → relay:** `POST RELAY_URL/s/<school_id>/v1/sealed` with the sealed envelope as the
  body. The relay wraps it in a frame, forwards over the tunnel, streams the response back.
  `503 SCHOOL_OFFLINE` when no tunnel; `429` per-IP (240/10 s) and per-school (64 concurrent);
  5 MB body; 30 s forward timeout (`504`).
- **Tunnel frames (JSON over WS, byte-compatible in `cloud/relay` + `server::tunnel`):**
  `req  = {id, method, path, body_b64}` (relay→school), `resp = {id, status, body_b64}`
  (school→relay). `body_b64` is the raw HTTP body = the sealed envelope, so the relay never
  sees plaintext.

## Sealing design (Step 2)
- **Envelope** (`sync::protocol::SealedEnvelope`): `{device_id, method, path, server_epoch,
  sealed_b64}`. `sealed_b64 = base64(nonce(12) ‖ ChaCha20-Poly1305(dir_key, inner JSON))`,
  **AAD = `method \n path \n device_id \n server_epoch`** (any tampered field fails the open).
- **Keys** (`sync::seal`): the per-device **session key** agreed at join (Phase 4; now KEPT
  server-side — migration 0003 `device.session_key`) is expanded with an HKDF-style single
  32-byte block (`hmac`+`sha2`, no extra crate) into two per-direction keys — `c2s` (device
  seals requests, server opens) and `s2c` (server seals responses, device opens) — so the two
  directions never share a keystream. **Possession of the session key IS the device's auth
  over the relay** — no bearer token ever crosses the relay.
- **Inner** request `{counter, body}`, response `{status, body}`. `body` is the same DTO the
  LAN `/v1` endpoints use (`PushReq`, `{since,limit}`, `HeartbeatReq`, …). Request AAD uses the
  device's asserted epoch; response AAD uses the server's real epoch (drives fencing).
- **Replay defence** (migration 0003 `device.last_counter`): a strictly-monotonic per-device
  `counter` inside the sealed request; the server rejects `counter ≤ last seen`. A legitimate
  retry (relay dropped the response) uses a *fresh, higher* counter with the *same op_id*, so
  it is not a replay and applies exactly once (idempotent by op_id).
- **LAN unchanged:** the pinned-cert TLS to the school server stays for LAN; sealing is applied
  only on the relay route (TLS terminates at the relay there).

## Fencing rules (Step 5, §8.9)
- Every response carries `server_epoch`. The device stores the highest it has seen
  (`app_kv.known_epoch`) and refuses any response from a lower-epoch server (`EPOCH_OLD`,
  nothing applied) — enforced in `engine::sync_once`, `relay::RelaySealer::open_response`, and
  server-side in `sealed::dispatch` (a device asserting a newer epoch than this server holds
  gets a sealed `409 EPOCH_OLD` and nothing is applied).
- Licence `/v1/check` → `moved`/`revoked` ⇒ `persist_check` writes `licence.status`; the app
  fires `server_stop` (stops the LAN listener + the relay tunnel) and the state machine routes
  to the read-only "moved" screen. UI copy for the fenced screens is per the prompt
  ("This computer is no longer your school's server. Ask your Principal." / "This computer is
  no longer the school server").
- The relay accepts only one tunnel per school; a newer epoch replaces the older (older gets
  `4409 EPOCH_OLD`).

## Deployment notes (Docker / env / TLS)
- **`cloud/relay/Dockerfile`** — multi-stage → `gcr.io/distroless/cc-debian12` (non-root). The
  relay links `rustls(ring)`, not OpenSSL, so no system TLS libs at runtime. Native release
  binary = **2.49 MB** (`cargo build --release --locked`).
- **Env:** `RELAY_BIND` (default `0.0.0.0:8788`), `RELAY_SHARED_KEY` (must match `cloud/licence`).
  `cloud/licence` reads the same `RELAY_SHARED_KEY`.
- **TLS (both options documented, per Step 1):**
  1. *Reverse proxy (recommended, Dockerfile assumes it):* terminate TLS at Caddy/nginx/a
     managed LB and proxy to the relay's plain bind; WebSocket upgrades pass through.
  2. *In-process rustls:* a small follow-up — add `RELAY_TLS_CERT`/`RELAY_TLS_KEY` and serve
     with `tokio-rustls` using the same low-level pattern as the school server's `net.rs`. Not
     enabled now to avoid an unlisted PEM-parsing dep while hosting is undecided (STOP).

## Test results (real output)
- `cargo test --workspace` → **363 pass, 0 fail** (vidya lib 100 · e2e_flows 4 · **relay_e2e 5**
  · sync_e2e 8 · vidya-core 235 · no_floats 1 · doc 10; benchmark `#[ignore]`). Was 333 at P04.
- `cargo clippy --workspace --all-targets -- -D warnings` → **clean**.
- `cloud/licence`: **3 tests** pass, clippy clean. `cloud/relay`: **7 tests** pass, clippy clean.
- Crypto gate: `cargo tree -i aws-lc-rs` / `-i aws-lc-sys` → *no packages* (absent); `ring` is
  the sole provider.
- `npx tsc --noEmit` → clean. `npx vitest run` → **38 pass** (no JS changed this phase — the
  route label + Public-address UI are the remaining front-end wire-ups).

## Measured latency via relay
`relay_e2e::slow_network_still_confirms` injects 400 ms of one-way forward latency (3G-ish) and
still confirms: **round-trip 403 ms** → **≈3 ms** of client-seal + relay-forward + school
sealed-dispatch + apply + response-seal overhead (in-process, single machine). Real
wide-area latency is dominated by the physical RTT to the relay host (deploy-pending).

## Environment vs code failures (Rule 9)
- **Docker build did NOT run: the Docker daemon is not running in this environment**
  (`Cannot connect to the Docker daemon`). This is an environment gap, not a code failure —
  the Dockerfile's build step (`cargo build --release --locked`) is proven to compile natively
  (2.49 MB binary). Run `docker build -t vidya-relay cloud/relay` once Docker Desktop is up.
- Live TLS/wss reconnect against a *hosted* relay, and the window-closed lifecycle, need a
  running app + a deployed relay → deploy-pending (STOP: hosting undecided, do not deploy).

## What's left in Phase 5 (for the next session)
1. **Sync screen route display** — surface `app_kv.last_route` ("On school Wi-Fi" / "Over the
   internet") in the Sync & devices screen (backend ready; React display only).
2. **Step 4 — Settings → Connection "Public address (optional)"** — off-by-default field +
   storage, and a `Route::PublicUrl(HttpsTransport)` slot placed BEFORE the relay in the route
   order (HttpsTransport already takes an arbitrary base URL + pinned cert).
3. **Fenced-screen UI** — the read-only "no longer the school server" screen with the
   "export a copy of the data for the Principal" link (data export lands with P7/P8 reports).
4. **Runtime verification** — live device↔relay↔school over real wss once a relay is hosted;
   the school-side reconnect loop and `moved`→stop transition against the running app.
5. **In-process rustls for the relay** (optional) if not fronting it with a TLS proxy.

## [OWNER] open questions (host, domain, cost)
- **Relay hosting/domain/provider** — undecided (STOP). Built + tested locally + Docker; not
  deployed. `RELAY_URL` comes from build-config; dev = `ws://127.0.0.1:8788`.
- **`RELAY_SHARED_KEY` management** — a single shared secret between `cloud/licence` and
  `cloud/relay`. Confirm rotation/roll-out (env/secret-manager) when hosting is decided.
- **`futures-util`** §13 companion dependency — confirm (see Dependency note).
- Kept prior defaults: relay limits (5 MB / 64 concurrent / 240 per-IP-10s / 30 s / 25 s ping),
  reconnect backoff 1→30 s, lease 30 d, licence recheck 30 d.
