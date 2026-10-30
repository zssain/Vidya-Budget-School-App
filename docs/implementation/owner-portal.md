# Owner licensing portal, checkout and billing worker (spec Phases 10–12)

Status: **built and verified locally in TEST MODE** against real Postgres 16 with fake
schools, fake secrets and mocked Supabase/Razorpay/Resend clients. Nothing here has been
run against a real Supabase project, Razorpay account, Resend account or public domain;
those gates are **BLOCKED** (see the last section). No price, domain, merchant account
or owner credential has been invented.

The signer is a separate component (`services/licence-signer`, documented in
[licence-signer.md](licence-signer.md)).

## 1. Layout

| Path | What | Deployed as |
|---|---|---|
| `packages/billing-core/` | Shared, **zero-runtime-dependency** TypeScript billing domain: machine-code checksum, money, Razorpay signature checks, webhook ingestion, job queue, provisioning state machine, reconciliation, delivery, audited owner actions, rate limits, CSV, migrations runner, test support. Server-only (uses `node:crypto`). | Not deployed on its own. Imported as TS source by the two below via `file:` dependencies. |
| `apps/licence-portal/` | Next.js App Router app: owner dashboard + public checkout + API routes. Own `package.json` + `package-lock.json`. | **Twice**: `PORTAL_SURFACE=owner` (owner host) and `PORTAL_SURFACE=checkout` (checkout host). |
| `services/billing-worker/` | Continuously running Node worker (`npm start` = `tsx src/main.ts`), migration CLI (`npm run migrate`). | A long-lived process (container/VM/systemd). **Not** a serverless function. |
| `infra/licensing/migrations/` | `0001_billing.sql` (committed earlier, unchanged) + `0002_portal_support.sql` (added). | Applied with `npm run migrate` as the DB owner. |
| `infra/licensing/setup/` | Operator templates: login roles, catalogue. Not migrations. | Run by hand. |
| `scripts/check-portal-isolation.mjs` | CI check: fails if portal/worker deps or secrets reach the Vidya app. | CI step. |

Why `billing-core` has no dependencies: both the portal and the worker own their
`pg` install and pass a pool in (structural `PoolLike` type), so there is no second
copy of `pg`, nothing extra for Turbopack to resolve outside the app, and the domain
code is testable from either package.

## 2. Exact dependency versions (pinned exactly, verified with `npm view` on 2026-10-31)

| Package | Version | Where | Why this version |
|---|---|---|---|
| next | 16.4.3 | portal | Current stable. Node ≥ 20.9; Turbopack default; `proxy.ts` replaces middleware (Node runtime). |
| react / react-dom | 19.3.1 | portal | Current stable; within Next 16 peer range `^18.2 \|\| ^19`. |
| @supabase/supabase-js | 2.118.0 | portal | Current; provides `auth.getUser()`, `auth.getClaims()` (verified JWT, `aal`/`amr`), MFA TOTP API, admin API for bootstrap. |
| @supabase/ssr | 0.12.7 | portal | Current; peer `@supabase/supabase-js ^2.114`. Official Next.js SSR cookie flow (`getAll`/`setAll`). |
| razorpay | 2.9.8 | portal, worker | Official Node SDK (orders.create, orders.fetchPayments). |
| resend | 6.36.0 | worker | Official SDK; `emails.send(payload, { idempotencyKey })`. Returns `{data,error}` (does not throw) — the adapter converts errors so the job retries. |
| pg | 8.23.1 | portal, worker (dev: core) | node-postgres; `FOR UPDATE SKIP LOCKED` jobs, transactions. |
| zod | 4.6.7 | portal, worker | Request/env validation. |
| tailwindcss / @tailwindcss/postcss | 4.3.3 | portal (dev) | Tailwind v4 via PostCSS (`@import "tailwindcss"`). |
| lucide-react | 1.65.0 | portal | Icons; peer React 16–19. |
| server-only | 0.0.1 | portal | Build-time guard that server modules are never imported by client components. |
| tsx | 4.23.15 | worker (runtime), portal (bootstrap script) | Runs the worker's TS + the TS-source `billing-core` without a build step. |
| typescript | 5.9.3 | all | **Not** 7.0.x (latest): `typescript-eslint@8.72.1` requires `<6.1`, and Next's type-check uses the TS JS API. |
| vitest | 4.1.12 | all (dev) | **Not** 5.0.x: vitest 5 declares engines `^22.12 \|\| ^24 \|\| >=26`, excluding the local Node 25.3.0; 4.1 supports ≥24. |
| eslint | 9.39.5 (portal) / 10.14.0 (core, worker) | dev | `eslint-config-next@16.4.3` pulls `eslint-plugin-react@7.37.5` and `eslint-plugin-import@2.32.0`, whose peers stop at ESLint 9. ESLint 9 is in **maintenance** (npm marks it deprecated) — revisit when those plugins support 10. The non-React packages use ESLint 10. |
| eslint-config-next | 16.4.3 | portal (dev) | Matches Next. |
| typescript-eslint / @eslint/js | 8.72.1 / 10.0.1 | core, worker (dev) | Flat config for ESLint 10. |
| @types/node / @types/pg / @types/react(-dom) | 24.19.2 / 8.23.1 / 19.3.0 | dev | Node 24 LTS is the deployment target (`engines: >=22.12`). |

Nothing was added to the root `package.json`, root lockfile, `src/`, `src-tauri/`,
`crates/` or the root Cargo workspace.

## 3. Routes

Owner surface (`PORTAL_SURFACE=owner`, e.g. `owner.<domain>`):

| Route | Access |
|---|---|
| `/login` | public form → `POST /api/auth/login` (rate-limited per IP and per email; non-owner accounts are signed out with the same generic error) |
| `/mfa` | **owner UUID at AAL1 only**: enrol TOTP (needs a password sign-in < 10 min ago and no verified factor yet) or verify; also the step-up page (`?step_up=1`) |
| `/dashboard`, `/schools`, `/orders`, `/orders/[id]`, `/payments`, `/licences`, `/licences/[id]`, `/transfers`, `/delivery`, `/audit`, `/settings` | `requireOwnerPage()` → owner UUID + AAL2 |
| `POST /api/owner/actions` | same-origin + owner UUID + AAL2; **fresh TOTP (≤ 5 min)** for `transfer`, `reissue_failed`, `approve_manual_payment`, `create_manual_order`, `set_machine_code` |
| `GET /api/owner/export/[table]` | owner UUID + AAL2; CSV with formula-injection protection |
| `GET /api/owner/licences/[id]/vlic` | owner UUID + AAL2; exact stored artefact |
| `POST /api/auth/mfa/enroll`, `/api/auth/mfa/verify`, `/api/auth/logout` | same-origin; enrol/verify owner UUID at AAL1+; logout revokes all sessions (`scope: global`) |

Checkout surface (`PORTAL_SURFACE=checkout`, e.g. `buy.<domain>`):

| Route | Purpose |
|---|---|
| `/buy` | product list from `billing.products`; form requires school name, contact, machine code, confirmation |
| `POST /api/checkout/orders` | same-origin, rate-limited; validates the machine-code checksum; amount/currency/modules from the catalogue only; creates the internal order then the Razorpay order; returns public checkout values + the status capability URL |
| `POST /api/checkout/verify` | browser return: verifies `razorpay_signature` against the provider order **we stored for that capability** and only enqueues `reconcile_order` |
| `/buy/status/[token]` | the customer's capability page (one order); "Payment received; preparing licence" while jobs run; licence key + `.vlic` once issued; machine-code completion form for `paid_awaiting_machine_code` |
| `GET /api/checkout/licence/[token]` | `.vlic` download for that one order |
| `POST /api/checkout/machine-code` | completion via the capability (once) |
| `POST /api/webhooks/razorpay` | raw body, HMAC, receipt + job in one transaction, duplicates → 200 |

Shared: `/` (redirects per surface), `/api/health` (DB reachability only).
Routes of the other surface return 404 in `proxy.ts` **and** in each handler/page.

## 4. State machines

Payment, fulfilment and delivery are separate.

**Order** (`billing.orders.status`), never moves backwards (`nextOrderStatus`, unit-tested):

```
created ──(provider order attached)──▶ pending ──(verified CAPTURE, code present)──▶ paid
   │                                     │      └─(verified CAPTURE, no code)──▶ paid_awaiting_machine_code ─(code set once)─▶ paid
   └─(owner cancel)─▶ cancelled ◀────────┘   (a capture after cancel → paid + review flag)
paid / paid_awaiting ─▶ partially_refunded ─▶ refunded        any captured state ─▶ disputed
```
Authorized or failed payments never change fulfilment.

**Payment** (`billing.payments.state`), monotonic rank
`failed < authorized < captured < partially_refunded < refunded < disputed`; `refunded_paise`
only grows (`GREATEST`); `captured_at` is set once. Out-of-order or duplicate events
therefore converge on the same row.

**Entitlement / issuance**: created only by `ensureEntitlement` inside the transaction that
holds the order row lock (`SELECT … FOR UPDATE`), and only when the payment is
captured (or partially refunded) and the order has a machine code:

1 `entitlements` row (`order_id` UNIQUE) + 1 `licence_issuances` row (generation 1,
`status='pending'`, `licence_id='VID-'+uuid`, `machine_code`, `issued_at` = RFC 3339 UTC
second precision) + 1 `jobs` row (`issue_licence`, `dedupe_key='issue:<issuance_id>'`,
payload `{"issuance_id": …}`). Insert uses `ON CONFLICT (order_id) DO NOTHING`, so a second
attempt creates nothing. A fully refunded or disputed payment never creates one.
`commercial_status`: `active` → `refunded` / `disputed` (set by the worker).

**Delivery**: the worker's sweep enqueues one `deliver_licence` (`deliver:<issuance_id>`) per
`issued` issuance; owner resend enqueues `resend:<issuance_id>:<uuid>`. `deliveries` row per
idempotency key: `pending → sent` or `failed` (retried). The delivery code only reads
`licence_issuances`, and `vidya_worker` has **no UPDATE** on that table, so a delivery can
never re-issue or alter a licence (tested).

**Transfer / re-issue** (owner, step-up): new generation `n+1` with a **new**
`licence_id` (UNIQUE) bound to the new machine code (transfer) or the same machine code
(re-issue after the signer marked the issuance `failed`), plus its `issue_licence` job;
transfers also get a `licence_transfers` row. Blocked unless `commercial_status='active'`
and the source is the latest generation. The older generation keeps `status='issued'` in the
database (no role may change another component's rows); the UI derives and shows
**superseded** and states that the old offline PC can still verify its licence.

**Jobs** (`billing.jobs`): `ready → running(lease) → done`; failure → `ready` with backoff
5 s·2^(n−1) capped at 1 h; permanent error or `attempts ≥ max_attempts` → `dead` (visible on
the dashboard). Attempts are counted at claim time. A `running` job whose lease expired is
reclaimable (lease recovery) or dead-lettered if out of attempts. Claim =
`FOR UPDATE SKIP LOCKED`. The worker claims only `process_webhook`, `reconcile_order`,
`deliver_licence`, `purge_webhooks`; the signer claims `issue_licence`.

## 5. Payment verification rules

* Webhook: HMAC-SHA256 over the **exact raw bytes** (`req.arrayBuffer()`), hex,
  `crypto.timingSafeEqual`; JSON is parsed only after verification; 256 KiB cap. The
  receipt (`provider, environment, event_id` UNIQUE; event id from `X-Razorpay-Event-Id`,
  else `sha256:<body digest>`) and its job are committed in one transaction **before** the
  200. DB failure → 500 → Razorpay redelivers. Duplicate → 200 `{duplicate:true}`.
* Worker (`processWebhookReceipt`): locks the receipt; if not `pending` it is a no-op. Then
  checks, rejecting with an owner review flag: merchant `account_id` (if
  `RAZORPAY_ACCOUNT_ID` set), **environment** (the receiving deployment's mode vs the
  order's), provider, **amount** (integer paise, exact), **currency**, **provider order id
  ↔ internal order**, `payment.captured`/`order.paid` must carry a captured entity. Unknown
  orders → `ignored`. Receipt state is written in the same transaction as the effect.
* Authorized-only → payment recorded `authorized`, no entitlement, delayed reconcile job.
* Refund (`refund.processed`) / dispute events → state recorded, entitlement
  `commercial_status` updated, one `review_*` audit flag per change. Nothing is "disabled".
  `refund.created`/`refund.failed`, dispute won/closed → audit only.
* Reconciliation: `reconcile_order` calls `orders.fetchPayments(<our provider order>)` with
  server credentials and applies each payment through the same function. Scheduled for our
  own unpaid orders aged 10 min–72 h (one per order per hour), after a browser return, after
  an authorized event, and by the owner. API errors retry; they never mark paid.
* Browser success is never proof: `/api/checkout/verify` only enqueues reconciliation.
* Manual payment approval exists only for `provider='manual'` orders, is labelled
  "MANUAL APPROVAL (not a verified provider payment)", requires reason + reference + typing
  `MANUAL` + fresh TOTP, creates **no** `payments` row (verified-sales totals exclude it) and
  an entitlement with `source='owner_manual'`.
* Refund **initiation** is not implemented in the portal: refunds are initiated in the
  Razorpay dashboard (its own authentication) and recorded here by webhook.

## 6. Security model

**Owner identity.** One Supabase Auth user, created by
`apps/licence-portal/scripts/bootstrap-owner.ts` (local, one-time, refuses if other users
exist or `OWNER_AUTH_USER_ID` is set). Public signups disabled in Supabase. No signup code
path exists in the app.

**`requireOwner()`** (`src/lib/owner-guard.ts`) — called by every owner page, route handler
and mutation before any query:
1. `auth.getUser()` — Supabase Auth server validates the session;
2. `auth.getClaims()` — verified JWT claims;
3. `claims.sub === user.id === OWNER_AUTH_USER_ID` (email and user/app metadata are never
   read — forged metadata is tested);
4. `claims.aal === 'aal2'`, except `allowAal1` for the MFA flow only;
5. optional freshness: newest `totp` AMR timestamp ≤ 5 min for sensitive actions.
Missing/malformed `OWNER_AUTH_USER_ID` or any SDK error → deny (fail closed).
The proxy's redirect for cookie-less requests is an optimisation only.

**Cookies.** `@supabase/ssr` defaults are JS-readable (so its browser client can work). We
never create a browser client — every auth call is a server route handler — so `setAll`
forces `HttpOnly`, `Secure` (production), `SameSite=Lax`, `Path=/`, no `Domain` (host-only),
and the cookie is named `__Host-vidya-owner` in production. Owner and checkout run on
different hosts, so the checkout host never receives the owner cookie. *Verified only by code
review + unit tests with a mocked client; real-browser cookie flags are BLOCKED on a real
Supabase project.*

**CSRF.** Every browser mutation checks `Origin === OWNER_ORIGIN/CHECKOUT_ORIGIN` (falls
back to `Sec-Fetch-Site: same-origin`). Webhooks are HMAC-authenticated instead.
**Redirects** go through `safeOwnerRedirect` (owner path allowlist; rejects `//`, schemes,
backslashes, encoded tricks). **Rate limits** live in Postgres (`billing.auth_rate_limits`,
hashed keys) so they hold across instances: login 10/IP and 5/email per 15 min, MFA 6 per
5 min, checkout 20/IP/h, status 120/IP/h. Supabase Auth's own limits apply as well.

**Caching.** Root layout `dynamic='force-dynamic'`; `requireOwner` calls `connection()`; the
build output shows every route as `ƒ (Dynamic)`. Owner responses carry
`Cache-Control: no-store, max-age=0, private` (+ `CDN-Cache-Control`). There is no service
worker. CSP: owner `default-src 'self'`, no third-party origins; checkout additionally
allows `checkout.razorpay.com`/`api.razorpay.com`. Scripts allow `'unsafe-inline'` (Next
hydration without nonces) — documented limitation.

**Secrets.** No `NEXT_PUBLIC_` variable exists (ESLint rule forbids them). The owner
deployment holds no Razorpay secret; the checkout deployment holds no Supabase config; the
portal runtime holds no Supabase secret key. A production build with dummy secrets was
grepped: none appear in `.next/static`.

**Database.** Each service connects as a login role inheriting exactly one of
`vidya_portal` / `vidya_worker` / `vidya_signer` (migrations). RLS is enabled on every
table with policies only for those roles, so `anon`/`authenticated` (Supabase Data API) see
nothing. Tests connect with `role=vidya_*` so grants and RLS are exercised. Proven by tests:
`owner_audit` append-only for every role; worker cannot update issuances; signer cannot
create payments/entitlements; portal cannot write payments.

**Customer capability.** 256-bit random token in the URL path; only its SHA-256 is stored
(unique index); expires after 30 days; scoped to one order; artefact shown only once issued;
`Referrer-Policy: no-referrer`. Order ids, payment ids and school names are never accepted.

**Dashboard honesty.** Cards: verified captured sales, refunds, net, manual approvals
(separate), paid-awaiting-issuance, issued licences ("issued ≠ installed/active"), delivery
failures, pending transfers, dead/retrying jobs, review flags, worker heartbeat. Currency
(INR from integer paise) and range (last N days by capture time, UTC) are labelled. Activation
is shown as "unknown / not reported".

## 7. Configuration

| Variable | Used by | Classification |
|---|---|---|
| `PORTAL_SURFACE` | portal | public config (`owner` / `checkout`; `both` is refused in production) |
| `OWNER_ORIGIN`, `CHECKOUT_ORIGIN` | portal | public config — exact origins |
| `SUPABASE_URL`, `SUPABASE_PUBLISHABLE_KEY` | portal (owner) | public by Supabase's design, but only used server-side here (no `NEXT_PUBLIC_`) |
| `OWNER_AUTH_USER_ID` | portal (owner) | server allowlist identifier (not a secret, not a password) |
| `SUPABASE_SECRET_KEY` | bootstrap script only (local shell) | **secret**, never deployed |
| `PORTAL_DATABASE_URL` | portal | **secret** (role ⊂ `vidya_portal`) |
| `BILLING_WORKER_DATABASE_URL` | worker | **secret** (role ⊂ `vidya_worker`) |
| `BILLING_MIGRATION_DATABASE_URL` | `npm run migrate` | **secret** (DB owner), operator only |
| `SIGNER_DATABASE_URL`, `LICENCE_SIGNING_KEY_FILE`, `LICENCE_SIGNING_KEY_ID`, `EXPECTED_PUBLIC_KEY_FINGERPRINT` | signer only | see licence-signer.md |
| `RAZORPAY_MODE` | portal (checkout), worker | public config; must match the key prefix (enforced) |
| `RAZORPAY_KEY_ID` | portal (checkout), worker | public checkout identifier (sent to the browser by the server) |
| `RAZORPAY_KEY_SECRET`, `RAZORPAY_WEBHOOK_SECRET` | checkout / worker | **secret**, distinct test vs live |
| `RAZORPAY_ACCOUNT_ID` | worker (optional) | config; merchant check |
| `RESEND_API_KEY` | worker | **secret** |
| `BILLING_FROM_EMAIL` | worker | config; sender on a Resend-verified domain |
| `CHECKOUT_BRAND_NAME` | checkout | public config |
| `WORKER_*`, `WEBHOOK_RETENTION_DAYS` | worker | config |

Templates: `apps/licence-portal/.env.example`, `services/billing-worker/.env.example`,
`services/licence-signer/.env.example`.

## 8. Contract with the signer (honoured, tested with the real `vidya_signer` grants)

See licence-signer.md. This side: inserts entitlement + gen-1 issuance (`pending`,
`VID-<uuid>`, machine code, RFC 3339 `issued_at`) + `issue_licence` job
(`issue:<issuance_id>`, `{"issuance_id"}`) in one transaction; later generations use a new
`licence_id` and their own machine code (the signer signs the **issuance's** machine code and
takes school name + modules from the entitlement — confirmed by the signer author); delivery
starts only once `status='issued'` and sends the stored `vlic`/`licence_key`. The signer does
not write `worker_heartbeats`, so the dashboard shows "no heartbeat" for it (judge it by its
job queue).

## 9. Isolation

`node scripts/check-portal-isolation.mjs` fails if any portal/worker dependency appears in
the root `package.json`/workspaces/lockfile, if portal code or secret variable names appear in
`src/`, `web-pwa/`, `src-tauri/src` or the app bundles (`dist/`, `dist-mobile/`,
`web-pwa/dist/`), or if a server crate joins the root Cargo workspace. It passes on this
branch and was shown to fail on a scratch tree (`PORTAL_ISOLATION_ROOT=…`). It is **not yet
wired into CI** (the app's CI is owned elsewhere): add `node scripts/check-portal-isolation.mjs`
as a step.

## 10. Evidence (local, 2026-10-31, Node 25.3.0, Postgres 16 on :55432)

| Command | Result |
|---|---|
| `cd packages/billing-core && npx vitest run` | 38 passed (incl. 15 machine-code vectors produced by the real Rust `vidya_core::licence`) |
| `cd services/billing-worker && npx vitest run` | 42 passed (webhook signature valid/forged/reserialised/malformed, wrong amount/currency/unknown order/environment/merchant, authorized-only, duplicates, payment.captured+order.paid, out-of-order refund↔capture, partial refund, dispute, manual approval, concurrency ×8 workers + 10 parallel receipts → 1/1/1, crash before/after commit, lease recovery, dead-letter, missed-webhook reconciliation, Razorpay outage retry, forged/repeated browser callback, signer outage, email outage → retried, no re-issue, resend same artefact, transfer, re-issue, capability isolation, DB-role grants) |
| `cd apps/licence-portal && npx vitest run` | 47 passed (requireOwner matrix; every owner API/page denied for unauthenticated, non-owner, AAL1 owner, forged metadata, forged claims; CSRF; step-up; MFA flow; webhook route; checkout amount/machine-code/CSRF/capability; surface separation) |
| `npm run typecheck` / `npx tsc --noEmit` (all three) | clean |
| `npx eslint .` (all three) | clean |
| `cd apps/licence-portal && npx next build` | success; every route `ƒ (Dynamic)` (none prerendered); no dummy secret in `.next/static` |
| `node scripts/check-portal-isolation.mjs` | passed |
| HTTP smoke (`next start`, owner surface :3999 + checkout surface :3998, worker process) | owner: unauthenticated `/dashboard` → 303 `/login`, owner API → 401, cross-origin action → 403, checkout routes → 404; checkout: owner routes → 404, `/buy` lists the fixture product, signed webhook → 200, duplicate → 200 duplicate, forged/reserialised → 401, bad machine code → 400; worker processed the stored receipt (`ignored: unknown_order`), wrote its heartbeat, health endpoint 200 |

Not verified locally: real browser rendering/screenshots, Playwright, real cookies.

## 11. BLOCKED on external accounts / owner inputs

| Gate | Needs |
|---|---|
| Real owner sign-in, cookie flags in a browser, TOTP enrolment, signup-disabled check | Supabase project + owner email (BLOCKED) |
| Real test checkout → captured webhook → entitlement → signer → email | Razorpay test account + keys + webhook secret, Resend key + verified domain, signer test key (BLOCKED) |
| Live payments | merchant KYC, live keys, **selling price** (BLOCKED — never invented) |
| Public hosts, CSP/HSTS in production, exact redirect URLs | domain + hosting (BLOCKED) |
| Customer can find the machine code | **App gap**: `api.machine_code()` exists ("for Welcome → Set up") but no screen on this branch renders it. The checkout requires the code; the app must display it before real sales. Not changed here (app code out of scope). |
| Activation on the intended PC with a portal-issued licence | built app + signer key aligned with the app release key (BLOCKED) |
