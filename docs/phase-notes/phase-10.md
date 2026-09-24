# Phase 10 handoff — company side: purchase, licence service, admin panel, downloads

Branch: `rebuild/p10` (from `rebuild/p09` @ `8d4de2e`). Start `git status`: clean.
The owner committed the Fly.io deploy scaffolding (`cloud/licence/Dockerfile`,
`cloud/licence/fly.toml`, `docs/DEPLOY-FLY.md`) onto this branch during the session
(HEAD `42751f8` at write time); this phase's service code is on top of that.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, Python 3.14 (for the smoke script only).

Everything is a **single, separate crate** at `cloud/licence/` (its own
`[workspace]`, `exclude`d from the app workspace in the root `Cargo.toml`), so
none of it can ever be pulled into the shipped apps. ~4,000 lines of Rust.

## Owner decisions taken this phase (asked in Step 1)

1. **Purchase = manual UPI (no payment provider / no webhook).** Owner: "i will
   just add my qr code and they scan and pay." So the website shows the company UPI
   QR, the buyer pays, and a **company admin verifies the payment in the admin panel
   and issues the licence.** The prompt's safety invariants are preserved under this
   model: **only an authenticated admin verification issues a licence** (replaces
   "only a verified webhook marks paid"); a browser action (submitting a UPI
   reference) **never** grants a licence; **one licence per order** even if verified
   twice. New order status `awaiting_verification` sits between `created` and `paid`.
2. **Activation codes stored ENCRYPTED, re-viewable** (ChaCha20-Poly1305 with a
   server key) so the Account page can re-display them. `code_hash` (SHA-256) is the
   activation lookup key; `code_enc` is the re-viewable ciphertext.
3. **Transfer = admin-manual + self-service.** Self-service uses a recovery-key
   proof; the server side is built and the small app-side contract addition is
   documented below (a [VERIFY] item).
4. **Payment provider** — n/a (superseded by #1).

## What was built (files)

| Area | Files |
|---|---|
| Schema | `migrations/0001_init.sql` — all Step-2 tables (`order`→`purchase_order`) |
| Core | `config.rs` · `db.rs` · `keys.rs` · `crypto.rs` · `audit.rs` · `ratelimit.rs` · `error.rs` · `state.rs` |
| Domain | `domain.rs` — issue / activate / check / transfer (pure over a `&Connection`) |
| Licence API | `api.rs` — `/v1/activate|check|transfer`, `/healthz`, rate limits |
| Website | `web.rs` + `ui.rs` (tokens/Welcome look) + `releases.rs` (downloads) |
| Admin | `admin.rs` — login/session/CSRF + all Step-5 screens & actions |
| Binary/CLI | `main.rs` + `lib.rs` |
| Ops | `Dockerfile` · `.dockerignore` · `.env.example` · `releases.example.json` |
| Tests | `tests/domain_tests.rs` · `tests/http_tests.rs` · `tests/no_key_leak.rs` |

## Data model (Step 2)

Plain SQLite (`rusqlite bundled`) — operational data, not school data, so no
SQLCipher; backups are encrypted at the ops layer (below). Times = ISO-8601 UTC;
money = integer paise; ids = UUIDv7.

Tables: `customer`, **`purchase_order`** (Step-2 `order`; ORDER is a SQL keyword),
`payment_event` (`provider_event_id` UNIQUE → idempotent; kinds
`buyer_claim|admin_verify|admin_reject|refund`), `licence` (adds `server_epoch`,
`recovery_verifier_enc`, `app_version_last_seen`, `last_check_at`, `revoked_*`),
`activation_code` (`code_hash` UNIQUE + `code_enc`), `transfer`, `admin_user`
(Argon2id, lockout), `admin_session` (cookie-hash id + per-session CSRF token),
`admin_audit` (append-only, hash-chained, UPDATE/DELETE blocked by triggers),
`relay_secret`. Migrations tracked in `schema_migrations`.

## API reference (docs §10)

- `POST /v1/activate {code, school_name, machine_id, app_version, recovery_verifier?}`
  → `200 {licence, signature, relay_secret}`. Binds the code to `machine_id` on
  first use; **idempotent** for the currently-bound machine; `409 CODE_ALREADY_USED`
  for another machine; `403 REVOKED`; `404 CODE_NOT_FOUND`. `licence` = the exact §10
  JSON, ed25519-signed; `relay_secret = base64(HMAC-SHA256(RELAY_SHARED_KEY,
  school_id))` (P05 recipe, byte-identical to `cloud/relay`).
- `POST /v1/check {licence_id, machine_id}` → `{status: active|revoked|moved}`
  (`moved` when another machine now holds it). `404 CODE_NOT_FOUND`.
- `POST /v1/transfer {licence_id, new_machine_id, recovery_proof, timestamp}` →
  same success body; `server_epoch + 1`, old machine → `moved`, recorded in
  `transfer`. **Idempotent (no proof)** when `new_machine_id` is already bound (also
  the delivery path after an admin-approved rebind). Otherwise a fresh valid
  `recovery_proof` is required. `403 PROOF_INVALID` / `403 TRANSFER_UNAVAILABLE`.
- `GET /healthz` → `200 ok` / `503`.

Errors are generic (`{"error": CODE}`), never revealing whether other schools
exist. Rate limits: per IP (120/60 s) and per code/licence (8/600 s) → `429
RATE_LIMITED`.

### Transfer recovery-proof design ([VERIFY] — app wiring needed)

Self-service transfer verifies `recovery_proof = base64(HMAC-SHA256(TVK,
licence_id ‖ '\n' ‖ new_machine_id ‖ '\n' ‖ timestamp))`, where **TVK** is a
32-byte "transfer verifier key" the app derives from the school recovery key and
**registers at activation** via the new optional `recovery_verifier` field. The
server stores TVK **encrypted at rest** (`recovery_verifier_enc`), so a DB-only leak
cannot forge a proof; the timestamp must be within ±10 min to bound replay. **App
side still to wire (Phase 3/8 owners):** (a) derive TVK with a domain-separated KDF
from the recovery key and send it as `recovery_verifier` on `/v1/activate`; (b) at
restore, compute the proof and call `/v1/transfer`. Until that lands, transfers use
the **admin-approved** path (support verifies the purchaser → rebind in the panel →
the new PC fetches its licence via the idempotent `/v1/transfer` branch). Both paths
are built and tested here.

## Purchase flow (Step 3) — manual UPI

Website (server-rendered, app tokens + Welcome look, brand logo): **Home**,
**Pricing** (`[Price]` placeholder), **Checkout** (name/email/phone/school → creates
an order), **Pay** (shows the UPI QR + `[UPI ID]`; buyer taps "I've paid" → records a
**claim**, status `awaiting_verification`), **Account** (order id + email → status;
re-shows the code once issued), **Downloads**, **Support**. "We also emailed it" is
never claimed (email isn't configured — honest, rule 13). Admin verification issues
the licence + code in one transaction (`domain::verify_payment_and_issue`),
idempotent by "one licence per order".

## Admin panel (Step 5)

`/admin`, company staff only. Argon2id passwords (§9 params) + lockout (5 fails →
15 min); session cookie `HttpOnly; SameSite=Strict` (+ `Secure` in production,
toggled off only in `--dev` over http); per-session **CSRF token** required on every
mutating form; optional `ADMIN_IP_ALLOWLIST`. Screens: **Licences** (search by
school/email/order/licence id), **Licence detail** (customer, verified payment
reference, plan/limits, activation history, server machine [masked], epoch, app
version, transfers) with actions **reissue code** / **revoke** (typed confirmation) /
**approve transfer** / **rotate relay secret** / **add note**; **Orders** (verify &
issue / reject, read-only list); **Audit** (read-only, live chain-status). Every
action requires a **reason** and appends to the hash-chained `admin_audit`.

## Downloads page (Step 6)

Reads a `releases.json` (env `RELEASES_JSON`) — schema + sample in
`releases.example.json` (per-platform `file`, `download_bytes`, `installed_bytes`,
`sha256`, `requirements`, `signed_by` [null = unsigned]). Shows Windows/Android/macOS
cards with sizes, requirements, SHA-256, signing status, the Windows-10 WebView2
note, the §2 "data stored separately / not-counted" disclosure, and Android
"install unknown apps" steps. **Draft releases are never listed.** The Phase-9
release workflow does not yet emit `releases.json`; wiring is a one-step follow-up
(compose it from `SHA256SUMS.txt` + `size-report.json` on publish).

## Operations (Step 7)

`Dockerfile` (multi-stage → distroless, non-root; migrations/CSS/logos embedded via
`include_str!`). Env in `.env.example`; secrets never in git. `/healthz`. Structured
access logs (method/path/status/latency — no bodies, cookies or query strings; codes
only ever travel in POST bodies). **Signing-key rotation:** `gen-prod-key` mints a
new keypair; ship the new public key in the next app release (build-config); old
licences stay valid because the app verifies each signature against the public key
in its own build config, and existing licences were signed with the old key — so
rotate by shipping the new public key going forward and re-signing on next
activation/transfer (documented; no forced re-issue). **Backups:** the store is one
SQLite file under the Fly volume `/data`; Fly takes daily volume snapshots (5-day
retention) + on-demand snapshots (`fly volumes snapshots create`), and an encrypted
off-box copy can be made with `sqlite3 .backup` piped through `age`/`gpg` — see
`docs/DEPLOY-FLY.md §3c`.

## Deployment (owner's plan — confirmed & aligned)

Fly.io, two apps, region `bom` (Mumbai; fallback `sin`): licence →
`https://api.vidya.zuhairhussain.com`, relay → `wss://relay.vidya.zuhairhussain.com`.
Full guide in `docs/DEPLOY-FLY.md` (I filled in the exact env-var names it flagged as
placeholders). The service was aligned to the owner's `fly.toml`/`Dockerfile`:
`LICENCE_BIND`+`LICENCE_PORT` (or a combined `host:port`), `LICENCE_DATA_DIR=/data`
(DB at `/data/vidya-licence.db`), `/healthz`, and the `gen-prod-key` CLI. The
owner's `Dockerfile` was corrected to also `COPY migrations` + `assets` (needed for
`include_str!` to compile). **Nothing was deployed** (no Docker daemon / hosting /
secrets in this environment).

## Dependencies (Step 3 list + one flagged addition)

Used: `axum, tokio, rusqlite(bundled), serde, serde_json, ed25519-dalek, rand,
argon2, hmac, sha2, base64, uuid(v7), time, tracing, tracing-subscriber`. **Not
used** (dropped to stay lean; TLS terminates at the proxy, no outbound HTTP):
`reqwest`, `rustls`. **One addition beyond the Step-3 list — please confirm:**
**`chacha20poly1305`**, needed for the owner-chosen "encrypted, re-viewable" codes
(and to encrypt the transfer verifier at rest). It is the same AEAD the app already
uses (§13), so it is pre-vetted for the project. Dev-only: `tower` (drive the router
in tests). `cargo tree -i aws-lc-rs` is empty (ring only, via nothing here — no TLS
crate linked).

## Tests (Step 8) + verification (real output)

- `cargo test` → **25 pass, 0 fail**: unit **8** (crypto 5 · audit 2 · ratelimit 1),
  `domain_tests` **9**, `http_tests` **6**, `no_key_leak` **2**.
  Covers: forged/duplicate issuance (idempotent, one licence) · buyer claim never
  issues · activation retry same machine → identical · copied code other machine →
  `409` · transfer bad proof → rejected · transfer → old machine `moved`, epoch +1 ·
  admin session + CSRF enforced · generic API errors · rate limits · **no signing
  private key in any app artifact** (scans `src-tauri/` + `src/` for the dev seed).
- `cargo clippy --all-targets -- -D warnings` → **clean**.
- **DONE-MEANS smoke** (live `--dev` server, curl): **23/23 checks pass** — website
  pages render · downloads lists the sample release · buy → order · buyer claim
  (awaiting verification, no code) → admin login → **verify & issue** (code shown) →
  duplicate verify → same code/one licence → Account re-displays the code → activate
  machine A (licence+signature+relay_secret) → retry identical → copied code on
  machine B `409` → admin shows bound machine → **self-service transfer to B** →
  bad proof `403` → machine A `moved`, machine B `active`, **epoch 2** → unauth admin
  action → login redirect → audit chain OK. Script:
  `scratchpad/done_means.sh` (not committed).

## App-workspace / compatibility

- The dev signing key (`.dev-keys/ed25519.key`) is unchanged, so the app's
  `src-tauri/build-config/dev.json` `licence_public_key` keeps verifying offline.
- `--dev`, `gen-code`, `print-key` still work (dev activation for P03/P05 is intact;
  `gen-code` now issues through the real DB path).
- App workspace untouched (`exclude = ["cloud/licence"]`); no app files changed.

## Open owner decisions (build the default; please confirm)

1. **Business text** — `[Company name]`, `[Price]`, `[UPI ID]`, `[Terms URL]`,
   support email/phone, and **GST/invoice** requirements are placeholders (rule 4).
   Set via env (`.env.example`). Add the **UPI QR image** (`UPI_QR_PATH`).
2. **Email** — default is show-the-code-on-page (Success/Account) only; no email
   provider wired. Confirm, or supply a provider to also email codes.
3. **2-factor for admin** — not built (password + lockout + secure session + CSRF +
   optional IP allowlist are). Choose a method (e.g. TOTP) if wanted.
4. **`chacha20poly1305`** §13/Step-3 dependency addition (see Dependencies) — confirm.
5. **Self-service transfer app wiring** — the `recovery_verifier` registration +
   proof generation on the app side (see the [VERIFY] note). Until then, transfers
   go through the admin-approved path.
6. **Relay-secret rotation** — with P05's stateless relay the effective secret is
   `HMAC(RELAY_SHARED_KEY, school_id)`, so the admin "rotate" action records intent +
   audits but cannot change the live per-school secret without a P05 change (rotate
   `RELAY_SHARED_KEY` globally, or teach the relay to verify a stored secret). Same
   open item as P05's "RELAY_SHARED_KEY management".
7. **`releases.json` wiring** into the Phase-9 release workflow (one step on publish).
8. **Prices/plans** — model stays one-time perpetual, unlimited (§10). Set the price
   text when known; `max_students`/`max_devices` remain null (unlimited).
