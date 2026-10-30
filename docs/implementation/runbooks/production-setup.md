# Production setup (first deployment)

Order: test services → migrations → owner bootstrap + MFA → test checkout/webhooks →
signer compatibility → app integration → live configuration → controlled release.
Every step marked **(owner)** needs an account or a decision only the owner has; nothing in
this repository supplies it.

## 0. Decisions and inputs (owner)

- Domains: e.g. `owner.<domain>` (owner surface) and `buy.<domain>` (checkout). Two hosts, so
  cookies are isolated. Do not reuse the school PWA's host.
- Selling price(s), SKU names and module sets.
- Razorpay merchant account (KYC for live), Resend account + a sending domain you control.
- Hosting for: two portal deployments (any Node host that runs Next.js 16), one long-running
  billing worker, one long-running signer, Postgres (Supabase Postgres or any managed PG 15+).

## 1. Supabase project (owner)

1. Create a project (region close to India). Note the project URL and the **publishable** key.
2. Authentication → Providers: keep **Email** enabled; disable every other provider.
3. Authentication → Settings: **disable "Allow new users to sign up"**. Keep "Confirm email"
   on. Set the minimum password length ≥ 12.
4. Authentication → Multi-Factor: enable **TOTP**.
5. Authentication → URL configuration: Site URL `https://owner.<domain>`; redirect URLs:
   only `https://owner.<domain>/**`. (The portal does not use OAuth or magic links.)
6. API settings → Exposed schemas: do **not** add `billing`. RLS is enabled on every billing
   table and grants nothing to `anon`/`authenticated` anyway.
7. If you use JWT signing keys, prefer an asymmetric key (lets `getClaims()` verify locally).

## 2. Database

1. Use the Supabase database (or a separate managed Postgres). As the database owner:
   ```sh
   cd services/billing-worker
   BILLING_MIGRATION_DATABASE_URL='postgres://postgres:…@db.<ref>.supabase.co:5432/postgres?sslmode=require' npm run migrate
   ```
   Migrations are append-only and checksum-tracked in `public.vidya_billing_migrations`.
2. Create login roles from `infra/licensing/setup/login-roles.example.sql` with generated
   passwords: `vidya_portal_login`, `vidya_worker_login`, `licence_signer`.
3. Insert the catalogue from `infra/licensing/setup/catalogue.example.sql` **(owner: price)**.
4. Enable automated backups / PITR on the database.

## 3. Owner account bootstrap (owner, once, on your own computer)

```sh
cd apps/licence-portal && npm ci
SUPABASE_URL=https://<ref>.supabase.co SUPABASE_SECRET_KEY='<secret key from Supabase → API keys>' \
  npm run bootstrap-owner -- you@yourdomain
```
It refuses if other users exist, prints the **Auth user UUID** and a one-time random
password. Store the password in your password manager, then close the terminal. The
secret key is not needed anywhere else; you may rotate it afterwards.

## 4. Deploy the owner surface

Environment (secret store, no `NEXT_PUBLIC_` anywhere):
`PORTAL_SURFACE=owner`, `OWNER_ORIGIN=https://owner.<domain>`, `SUPABASE_URL`,
`SUPABASE_PUBLISHABLE_KEY`, `OWNER_AUTH_USER_ID=<uuid from step 3>`,
`PORTAL_DATABASE_URL=postgres://vidya_portal_login:…@…?sslmode=require`.
Build: `npm ci && npm run build`; start: `npm start` (Node 24). Disable any CDN caching for
this host (responses are `no-store` regardless). Health: `GET /api/health`.

## 5. Enrol MFA (owner)

Open `https://owner.<domain>/login`, sign in, you are sent to `/mfa`. Within 10 minutes of
the password sign-in click **Start MFA enrolment**, scan the QR code with an authenticator
app, enter the code. You now have an AAL2 session. Do not screenshot the QR/secret. Verify:
sign out, sign in again — the dashboard is unreachable until the TOTP code is entered.

## 6. Razorpay — TEST mode first (owner: account)

1. Dashboard in **Test Mode** → API keys → generate `rzp_test_…` key id + secret.
2. Webhooks → add `https://buy.<domain>/api/webhooks/razorpay`, a long random secret, events:
   `payment.authorized`, `payment.captured`, `payment.failed`, `order.paid`,
   `refund.created`, `refund.processed`, `refund.failed`, `payment.dispute.created`,
   `payment.dispute.won`, `payment.dispute.lost`, `payment.dispute.closed`,
   `payment.dispute.under_review`, `payment.dispute.action_required`.
3. Settings → Payment capture: **automatic capture** (authorized-only payments never issue a
   licence; Razorpay auto-refunds uncaptured authorizations).
4. Deploy the checkout surface: `PORTAL_SURFACE=checkout`,
   `CHECKOUT_ORIGIN=https://buy.<domain>`, `PORTAL_DATABASE_URL`, `RAZORPAY_MODE=test`,
   `RAZORPAY_KEY_ID`, `RAZORPAY_KEY_SECRET`, `RAZORPAY_WEBHOOK_SECRET`, `CHECKOUT_BRAND_NAME`.
   It needs no Supabase configuration.

## 7. Resend (owner: domain)

Add and verify the sending domain (SPF/DKIM DNS records) in Resend, create an API key with
"sending access" only. `BILLING_FROM_EMAIL="Vidya Licences <licences@<domain>>"`.

## 8. Run the billing worker and the signer

- Worker (a long-running process with restart-on-failure; **not** serverless):
  `BILLING_WORKER_DATABASE_URL` (vidya_worker_login), `RAZORPAY_MODE`, `RAZORPAY_KEY_ID`,
  `RAZORPAY_KEY_SECRET` (same mode as checkout), optional `RAZORPAY_ACCOUNT_ID`,
  `RESEND_API_KEY`, `BILLING_FROM_EMAIL`, optional `WORKER_HEALTH_PORT` (loopback).
  `npm ci && npm start`. One or more instances are safe (SKIP LOCKED).
- Signer: see [licence-signer.md § Running it](../licence-signer.md). Use a **test** signing
  key while Razorpay is in test mode.

## 9. End-to-end test-mode acceptance (BLOCKED until steps 1–8 are done)

1. On a school PC, get the machine code (see the app gap noted in owner-portal.md §11).
2. `https://buy.<domain>/buy` → pay with a Razorpay test card/UPI.
3. Status page shows "payment received; preparing licence", then the key and `.vlic`.
4. Owner dashboard shows the order paid, one licence issued, delivery sent.
5. Replay the webhook from the Razorpay dashboard → still one entitlement.
6. Activate the key in Vidya on that PC (Welcome → Set up); confirm a different PC rejects it.
7. Refund from the Razorpay dashboard → order `refunded`, review flag shown, licence not
   claimed as disabled.

## 10. Go live (owner)

Generate **live** keys + a **separate** live webhook secret; switch the checkout and worker
to `RAZORPAY_MODE=live` with the live values together; switch the signer to the production
key whose fingerprint is in the shipped app; set the real catalogue price. Test orders keep
`environment='test'` and can never be fulfilled by live events (and vice versa).
