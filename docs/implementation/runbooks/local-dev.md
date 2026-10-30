# Local development (test mode, fake data)

Prerequisites: Node ≥ 22.12 (developed on 25.3.0; deploy on 24 LTS), Postgres 15+ (tested
on 16), Rust only if you also run the signer.

## 1. Install (three independent packages — never from the repo root)

```sh
cd packages/billing-core     && npm ci
cd ../../services/billing-worker && npm ci
cd ../../apps/licence-portal     && npm ci
```

None of these touch the Vidya app's root `package.json` or lockfile.

## 2. Tests (they create and drop their own databases)

The suites need a Postgres superuser URL (default `postgres://postgres@localhost:55432/postgres`,
override with `BILLING_TEST_ADMIN_URL`). Each test file clones a fresh database from a template
with all migrations applied and connects with `role=vidya_worker|vidya_portal|vidya_signer`, so
grants and RLS are exercised.

```sh
(cd packages/billing-core     && npx vitest run && npx tsc --noEmit && npx eslint .)
(cd services/billing-worker  && npx vitest run && npm run typecheck && npx eslint .)
(cd apps/licence-portal      && npx vitest run && npx tsc --noEmit && npx eslint . && npx next build)
node scripts/check-portal-isolation.mjs
```

Template databases are named `vidya_billing_tpl_<hash>`; they are reused while the
migrations are unchanged and can be dropped at any time.

## 3. A local database

```sh
createdb -h localhost -p 55432 -U postgres vidya_billing
cd services/billing-worker
BILLING_MIGRATION_DATABASE_URL=postgres://postgres@localhost:55432/vidya_billing npm run migrate
psql postgres://postgres@localhost:55432/vidya_billing -f ../../infra/licensing/setup/catalogue.example.sql   # edit placeholders first; use an obviously fake TEST price locally
```

Locally you may connect as the superuser while *assuming* an application role:
`postgres://postgres@localhost:55432/vidya_billing?options=-c%20role%3Dvidya_portal`.
In production use real login roles (`infra/licensing/setup/login-roles.example.sql`).

## 4. Run

```sh
# portal: copy .env.example → .env.local; PORTAL_SURFACE=both is allowed in `next dev` only
cd apps/licence-portal && npm run dev           # http://localhost:3000

# worker: copy .env.example → .env (git-ignored) and fill test values
cd services/billing-worker && npx tsx --env-file=.env src/main.ts
```

`next start` (production mode) refuses `PORTAL_SURFACE=both` and serves the owner surface;
run two instances (`PORTAL_SURFACE=owner` and `PORTAL_SURFACE=checkout`) on two ports to test
a production build.

Without a Supabase project you cannot sign in (owner pages redirect to `/login`). The
automated tests cover the owner flows with a mocked Supabase server client.

## 5. Webhooks locally

Razorpay cannot reach `localhost`. Either use a tunnel to the checkout surface's
`/api/webhooks/razorpay` and register that URL in the Razorpay **test** dashboard, or post a
signed fixture yourself:

```sh
BODY='{"entity":"event","event":"payment.captured","payload":{"payment":{"entity":{"id":"pay_LOCAL1","amount":100,"currency":"INR","status":"captured","order_id":"order_LOCAL"}}}}'
SIG=$(printf %s "$BODY" | openssl dgst -sha256 -hmac "$RAZORPAY_WEBHOOK_SECRET" -hex | sed 's/^.* //')
curl -sS -X POST http://localhost:3000/api/webhooks/razorpay \
  -H 'content-type: application/json' -H "x-razorpay-signature: $SIG" -H 'x-razorpay-event-id: evt_local_1' \
  --data-binary "$BODY"
```
