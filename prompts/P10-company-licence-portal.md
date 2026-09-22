# PHASE 10 of 10 — COMPANY SIDE: PURCHASE, LICENCE SERVICE, ADMIN PANEL, DOWNLOAD PAGE

Can run in parallel with Phases 4–8 by a separate agent once Phase 3 is done (it owns the
contract in context §10). Never shipped inside the school apps.

## ROLE
You are a senior backend + security engineer building the company services for **Vidya
Budget School**.

## STANDING RULES
1. Read `docs/00-SYSTEM-CONTEXT.md` (esp. §1, §2, §10, §17) and `docs/01-MOCK-SPEC.md`, then
   `docs/phase-notes/phase-3.md` and `phase-5.md` (licence + relay contracts).
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. `cloud/licence` dependencies (its own Cargo.toml, not in the app workspace): `axum`,
   `tokio`, `rusqlite` (`bundled`), `serde`, `serde_json`, `ed25519-dalek`, `rand`,
   `argon2`, `hmac`, `sha2`, `base64`, `uuid`, `time`, `reqwest` (rustls, ring),
   `rustls`, `tracing`, `tracing-subscriber`. Anything else → ask.
4. **Never invent prices, plans, taxes (GST), refund terms, legal text, company name or
   payment-provider behaviour.** Use placeholders `[Company name]`, `[Price]`,
   `[Terms URL]` and the provider's official docs + test mode only.
5. No real payments, emails, deployments or production keys unless the owner explicitly
   provides credentials and asks.
6. Licence signing private key lives only on the server (env/secret store), never in git,
   logs or app builds.
7. Branch `rebuild/p10`; small commits; handoff at the end.

## OBJECTIVE
A school can buy Vidya on the website, receive an activation code, download the right
installer, and activate exactly one school server; the company can see and manage every
licence; a school can move to a new PC safely.

## STEPS

### Step 1 — Owner inputs (ask before Step 3; build everything else meanwhile)
Payment provider (e.g. Razorpay or other), test API keys + webhook secret, company name,
support email/phone, prices per plan (if plans exist), invoice/GST requirements, email
provider for sending codes (or "show code on the success page + account page only"),
domain names for website, `LICENCE_API` and `RELAY_URL`.

### Step 2 — Data model (`cloud/licence/migrations/`)
`customer` (email, name, phone, school_name), `order` (provider, provider_order_id UNIQUE,
amount_paise, currency, status created|paid|failed|refunded), `payment_event`
(provider_event_id UNIQUE, raw_json, verified, received_at), `licence` (licence_id,
school_id, plan, max_students NULL, max_devices NULL, status active|revoked|moved,
issued_at, server_machine_id NULL, server_epoch), `activation_code` (code_hash UNIQUE,
licence_id, redeemed_at, redeemed_machine_id, revoked_at), `transfer` (licence_id,
old_machine_id, new_machine_id, at, method), `admin_user` (email, password_hash, role,
disabled), `admin_audit` (append-only, hash-chained like the app), `relay_secret`
(licence_id, secret_hash, rotated_at).

### Step 3 — Purchase flow
Website pages (server-rendered HTML written by hand, styled with the app's tokens and the
Welcome screen look — navy panel, serif headline, teal buttons, brand logo):
Home (what Vidya is, platforms, honest size text: "Each download under 40 MB; the school's
data is stored separately"), Pricing (`[Price]` placeholders until the owner fills them),
Checkout (name, email, phone, school name → provider checkout in test mode), Success page
(shows the activation code once + download links; "we also emailed it" only if email is
configured), Account (email magic link or order id + email lookup **[OWNER]**) to view the
code again, Downloads (see Step 6), Support.
- Order creation calls the provider's API server-side.
- **Only a verified webhook marks an order paid** (verify signature exactly per the
  provider's docs; store the raw event; idempotent by provider_event_id and
  provider_order_id). A browser redirect never grants a licence.
- On paid: create licence + activation code (`VIDYA-XXXX-XXXX-XXXX`, Crockford base32,
  store only SHA-256 of the code + show it once, or store encrypted if the owner wants it
  re-viewable — ask) in one transaction. Repeated webhooks → no second licence.

### Step 4 — Licence API (context §10, production)
`/v1/activate`: code valid + unredeemed → bind to machine_id, sign licence JSON with the
production ed25519 key, return; same machine retry → same licence (idempotent); other
machine → `409 CODE_ALREADY_USED`; revoked → `403`. Also issue the relay secret (if the
owner approved the Phase 5 contract change).
`/v1/check`: `{status}` for licence + machine (`moved` if another machine now holds it).
`/v1/transfer`: verify `recovery_proof` (design agreed with the owner: e.g. HMAC over
`licence_id ‖ new_machine_id ‖ timestamp` with a key derived from the school's recovery
key, whose verifier the app registered at activation) → rebind, `server_epoch + 1`, old
machine → `moved`, record in `transfer`. Admin can also approve a transfer manually after
verifying the purchaser (support case).
`/v1/relay-auth` (if chosen in Phase 5).
Rate limits per IP and per code; generic error messages that don't reveal whether other
schools exist; all actions audited.

### Step 5 — Admin panel (`/admin`, company staff only)
Login: email + password (Argon2id), lockout, session cookie `HttpOnly; Secure;
SameSite=Strict`, CSRF tokens on forms, optional IP allowlist; 2-factor **[OWNER: method]**.
Screens (same visual language): Licences list (search by school/email/order/licence id),
licence detail (customer, verified payment reference, plan/limits, activation history,
server machine, epoch, app version last seen, transfers), actions: reissue activation code
(old one revoked), revoke licence (with reason, confirmation), approve transfer, rotate
relay secret, add note. Orders and payment events (read-only, raw JSON viewer). Admin
audit log (read-only, chain status). Every action requires a reason and is audited.

### Step 6 — Downloads page
Cards for Windows / Android / macOS: version, release date, file size (download) and
installed size, requirements (Windows 10/11 64-bit, macOS 12+, Android 7+), SHA-256,
signing status ("Signed by [Company name]" or "Unsigned — see install steps"), and a short
note that Windows 10 may download Microsoft WebView2 once. Data comes from a
`releases.json` produced by Phase 9's release workflow; draft releases are never listed.
Android page explains "Install unknown apps" steps.

### Step 7 — Operations
Dockerfile, env config (DB path, signing key, provider keys, admin bootstrap user), daily
encrypted DB backup instructions, health endpoint, structured logs without secrets,
signing-key rotation procedure (new public key shipped in the next app release; old
licences stay valid — document).

### Step 8 — Tests
Forged webhook → rejected · duplicate webhook → one licence · activation retry same machine
→ same licence · copied code on another machine → 409 · transfer without valid proof →
rejected · transfer → old machine `moved`, epoch +1 · admin CSRF/session tests · no
private key in any app artifact (search built apps for the key bytes) · rate limits.

## DONE MEANS
In provider TEST mode: buy → webhook → code shown → install app → activate → admin panel
shows the licence bound to the machine → restore on a second machine via transfer → first
machine reports `moved`. All tests green. Nothing deployed unless the owner asked.

## HANDOFF → `docs/phase-notes/phase-10.md`
API reference · data model · provider integration notes (test mode) · admin guide ·
deployment steps · open owner decisions (prices, GST/invoices, email, 2FA, domains).
