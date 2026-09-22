# PHASE 5 of 10 — INTERNET ACCESS THROUGH THE RELAY, SEALED TRAFFIC, SERVER FENCING

## ROLE
You are a senior Rust networking and security engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies with the listed features. Anything else → STOP and ask.
4. Never invent features, rules, copy, numbers, API fields, library APIs or config keys.
   Check installed crate source/docs for every API. Unsure → STOP and ask.
5. The mock wins. Status copy exactly as context §8.10.
6. The relay never sees plaintext and never stores school data.
7. No fake success anywhere.
8. Never delete/weaken tests. Never edit an earlier phase's migration.
9. Run every command you mention; paste real output. Separate environment vs code failures.
10. Work on branch `rebuild/p05`; small commits; no push/merge/tag/deploy unless asked.
11. Stop conditions are real.
12. Finish with the handoff file.

## OBJECTIVE
Staff can reach the school server from anywhere (mobile data, home Wi-Fi) through the Vidya
relay, without the school opening router ports. The relay can neither read nor change
traffic. A PC that is no longer the school server can never act as one.

## DONE MEANS
1. Phone on mobile data (not on school Wi-Fi) records attendance → "Confirmed by school
   server" via the relay.
2. Relay logs contain only school ids, sizes, timings and status codes — no bodies, tokens,
   names or numbers (checked by a test that captures relay logs).
3. Tampering one byte of a relayed request → rejected (AEAD failure), no data change.
4. Relay down → devices fall back to queue (Drive route comes in Phase 6); relay back →
   resumes automatically.
5. Epoch test: a server with a lower epoch is refused by devices; a PC told `moved` by the
   licence service stops serving.

## STEPS

### Step 1 — Relay service (`cloud/relay`, separate binary, never shipped in apps)
- axum (with `ws`) + tokio + rustls(ring). Stateless: no database, no disk writes except
  rotated logs.
- School side: the school server keeps ONE outbound WebSocket to
  `RELAY_URL/tunnel/<school_id>`, authenticated with a relay secret. **[OWNER/VERIFY]**
  Default: the relay secret is issued by `cloud/licence` at activation (add
  `relay_secret` to the activation response in both services and in context §10 — ask the
  owner to confirm before changing the contract). The relay verifies it by calling
  `cloud/licence` (`/v1/relay-auth`) or by an HMAC shared between the two services —
  choose the simpler one and document.
- Device side: devices call `https://RELAY_URL/s/<school_id>/v1/...`; the relay forwards
  each HTTP request as a frame over the tunnel and streams the response back.
- Limits: per-school concurrent requests, per-IP rate limit, 5 MB body, 30 s timeout,
  `503 SCHOOL_OFFLINE` when no tunnel, idle ping/pong every 25 s, tunnel reconnect with
  backoff on the school side.
- Dockerfile (distroless or alpine static binary), health endpoint, env config
  (`RELAY_BIND`, `RELAY_TLS_CERT/KEY` or run behind a TLS proxy — document both).

### Step 2 — End-to-end sealing
Every relayed body = `nonce(12) ‖ ChaCha20-Poly1305(session_key, request JSON)` with
associated data = method + path + device_id + server_epoch. Session key agreed at join
(Phase 4 already returns it; derive per-direction keys with HKDF-style SHA-256 expansion
using `hmac` + `sha2` — no extra crates). Responses sealed the same way. Replay defence:
monotonic request counter per device inside the sealed payload; the server rejects
counters ≤ last seen. The pinned-certificate TLS to the school server is kept for LAN;
over the relay, TLS terminates at the relay and the sealing provides end-to-end
confidentiality + integrity.

### Step 3 — Client routes
Route order becomes: LAN → relay (or school's configured public URL with pinned cert) →
queue. A route that fails is skipped for its backoff period; the Sync screen shows the
current route ("On school Wi-Fi", "Over the internet").

### Step 4 — Optional direct public address
Settings → Connection (server, advanced): "Public address (optional)" for schools with
their own port forward/DDNS. Clients use it with the pinned certificate before the relay.
Off by default; plain explanation text.

### Step 5 — Fencing (context §8.9)
- Every response includes `server_epoch`; devices store the highest seen and refuse
  lower (`EPOCH_OLD`, screen: "This computer is no longer your school's server. Ask your
  Principal.").
- Licence `/v1/check` returning `moved` → the old PC stops the server task and the relay
  tunnel, switches to read-only, shows "This computer is no longer the school server" with
  a link to export a copy of its data for the Principal.
- The relay accepts only one tunnel per school; a newer epoch's tunnel replaces the older
  one, the older gets `409 EPOCH_OLD`.

### Step 6 — Tests
Harness gains an in-process relay: scenarios — phone only reachable via relay; relay
drops mid-request (retry with same op_id, applied once); tampered frame; replayed frame;
two servers for one school with different epochs; relay restart; slow network (3G
profile: 400 ms RTT, 1% loss).

## STOP CONDITIONS
The licence contract must change (relay secret) → ask the owner first. Relay hosting
(domain, provider) is undecided → build and test locally + Docker; do not deploy.

## HANDOFF → `docs/phase-notes/phase-5.md`
Relay protocol · deployment notes (Docker, env, TLS) · sealing design · fencing rules ·
test results · measured latency via relay · open questions (host, domain, cost).
