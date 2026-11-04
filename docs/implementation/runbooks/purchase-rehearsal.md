# Purchase → activation rehearsal (local, test mode, fake data)

A re-runnable, automated rehearsal of the owner's pre-sale check "a customer buys, gets a
licence, and it activates on their computer", across every real component:

| Step | Real code exercised |
|---|---|
| Buyer's PC makes its machine id + machine code | `licence::machine::get_or_create` + `vidya_core::licence::machine_code` (src-tauri) |
| Checkout order for a FAKE school with that code | `createCheckoutOrder` + `attachProviderOrder` + `recordCheckoutReturn` (billing-core, as `vidya_portal`) |
| Razorpay-format `payment.captured` webhook, HMAC-signed with a TEST secret (plus a forged one that must be refused) | `ingestRazorpayWebhook` — what `apps/licence-portal/.../api/webhooks/razorpay` calls |
| Provisioning: payment, entitlement, issuance, `issue_licence` job | billing-worker job loop `runOnce` (as `vidya_worker`) |
| Signing with a throwaway TEST ed25519 key | the **real `licence-signer` binary** (services/licence-signer, as `vidya_signer`) |
| Delivery email with the `.vlic` attached; customer status page shows the key | `deliverLicence` (fake mailer), `findOrderByStatusToken` |
| Activation on the buyer's PC (`.vlic` and pasted key); rejected on another PC (`LICENCE_OTHER_MACHINE`); rejected when the payload or signature is tampered with or the build carries another public key (`LICENCE_INVALID`) | `activate_licence_with_keys` — the body of the `activate_licence` command — on a fresh encrypted DB; app state becomes `activated` and the pending licence is stored |

It also rehearses the **retry path across languages**: the Rust signer fails on its only
allowed attempt (fault injection: the job gets `max_attempts = 1` and `vidya_signer` loses its
`INSERT` grant on `billing.jobs`, so the commit transaction rolls back) → issuance `failed`,
order still paid ("Paid · Licence pending") → the TS owner action `retryFailedIssuance`
re-queues → the Rust signer issues with the **same `licence_id`** and a byte-identical payload
(one issuance, jobs `dead` then `done`) → that licence also activates on the buyer's PC.

Nothing here uses a real account: fake schools and buyers (`example.test`), a TEST webhook
secret, a random TEST signing key created for the run and deleted at the end, no network
calls (the Razorpay and Resend clients are stubs). `src-tauri/build-config` is never touched —
the activation test passes the run's test public key directly.

## Run it

Prerequisites: Rust, Node ≥ 22.12, Postgres 16 binaries (Homebrew `postgresql@16`).

```sh
npm run rehearse:purchase              # or: scripts/rehearse-purchase.sh
```

By default the script starts its **own throwaway Postgres 16 cluster** (initdb in a temp dir,
`127.0.0.1:55433`, trust auth, no unix socket) and stops + deletes it on exit. It refuses to
start if the port is already taken (it never talks to someone else's server by accident).

Options:

| Env | Meaning |
|---|---|
| `DATABASE_URL=postgres://postgres@127.0.0.1:5432/postgres` | Use an existing cluster instead (superuser URL). A fresh `vidya_billing_t_*` database is created from the migrations template and dropped afterwards. |
| `REHEARSAL_PG_PORT=55439` | Port for the throwaway cluster. |
| `PG_BIN=/path/to/postgresql@16/bin` | Where `initdb`/`pg_ctl` live (auto-detected for Homebrew and Debian paths). |
| `VIDYA_REHEARSAL_DIR=/some/dir` | Work dir (machine ids, test public key, `purchase.vlic`, `retry.vlic`, `*.licence-key.txt`, `issued.json`); kept after the run. |
| `KEEP_REHEARSAL_DIR=1` | Keep the default temp work dir. |

The run prints `REHEARSAL PASSED` (exit 0) or `REHEARSAL FAILED (exit N)`.

## What it runs (three phases)

1. `cargo test -p vidya --test purchase_activation -- --ignored --exact rehearsal_1_fake_machines`
   — two fake machines (`buyer`, `other`); writes `buyer.machine-code` / `other.machine-code`.
2. `cd services/billing-worker && npm run rehearse:purchase` (vitest, config
   `vitest.rehearsal.config.ts`, file `rehearsal/purchase.rehearsal.test.ts`; needs
   `VIDYA_REHEARSAL_DIR`, `LICENCE_SIGNER_BIN`, `BILLING_TEST_ADMIN_URL`) — test A (purchase)
   and test B (signer failure → owner retry → same licence_id). Writes the delivered `.vlic`,
   the status-page key and `issued.json`.
3. `cargo test -p vidya --test purchase_activation -- --ignored --exact rehearsal_2_activate`
   — activation / rejection assertions for both licences.

The two Rust tests are `#[ignore]`, and the rehearsal vitest file is outside the worker's
normal `test/` suite, so ordinary `cargo test` / `npx vitest run` stay self-contained.

## When it fails

* **Phase 1/3 compile errors** — the app crate is broken; fix that first (`cargo test -p vidya`).
* **"signer did not reach the expected state"** — the signer's logs are printed in the failure:
  check the fingerprint line, the DB URL (`role=vidya_signer`) and migration grants.
* **`LICENCE_INVALID` in phase 3 on the buyer machine** — the artefact format and the app
  verifier disagree (`vidya_core::licence` vs `services/licence-signer/src/artefact.rs`). Also
  run `services/licence-signer/tests/golden.rs`.
* **Port in use** — set `REHEARSAL_PG_PORT` or `DATABASE_URL`.

Related: [local-dev.md](local-dev.md) (the normal suites), [../licence-signer.md](../licence-signer.md).
