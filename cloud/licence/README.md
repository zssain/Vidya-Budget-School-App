# cloud/licence — Vidya licence service

A **separate** Rust binary (its own workspace) that is **never** built into or
shipped with the Vidya apps. It runs the company side (prompts/P10):

- the **public website** + **manual UPI purchase flow** (Home, Pricing, Checkout,
  Pay-by-UPI-QR, Account, Downloads, Support),
- the **production licence API** (`/v1/activate`, `/v1/check`, `/v1/transfer`),
- the **company admin panel** (`/admin`).

TLS terminates at a reverse proxy / Fly's edge (no in-process TLS crate). Full
deployment guide: [`docs/DEPLOY-FLY.md`](../../docs/DEPLOY-FLY.md). Handoff +
design: [`docs/phase-notes/phase-10.md`](../../docs/phase-notes/phase-10.md).

## Purchase model (owner decision)

Manual **UPI**: the buyer scans the company UPI QR and pays, then a company admin
**verifies the payment in the admin panel and issues the licence**. There is no
payment-provider integration and no webhook. A browser action (submitting a UPI
reference) **never** grants a licence — only an authenticated admin verification
does, and only one licence is ever issued per order.

## Run locally (dev)

```sh
cd cloud/licence

# Start the whole service on http://127.0.0.1:8787 (dev keys under .dev-keys/).
cargo run -- --dev
#   website   http://127.0.0.1:8787/
#   admin     http://127.0.0.1:8787/admin   (admin@vidya.local / vidya-dev-admin)

# Issue a licence + print an activation code (keeps app dev-activation working):
cargo run -- gen-code

# Print the signing public key (paste into src-tauri/build-config/dev.json):
cargo run -- print-key
```

To see the Downloads page populated in dev, point it at the sample:
`RELEASES_JSON=$PWD/releases.example.json cargo run -- --dev`.

## CLI

| Command | What it does |
|---|---|
| `--dev` | Start locally with dev keys + permissive cookies (http). |
| `serve` | Production; all secrets from env (see `.env.example`). |
| `gen-code` | Dev: create a paid order + licence and print the activation code. |
| `gen-prod-key` | Print a NEW production signing keypair (private seed + public key). |
| `gen-enc-key` | Print a NEW base64 code-encryption key (`LICENCE_CODE_ENC_KEY`). |
| `print-key` | Print the signing public key (base64). |
| `migrate` | Create/upgrade the DB at `$LICENCE_DB`, then exit. |
| `admin-add EMAIL` | Create/reset an admin (password from `$LICENCE_ADMIN_PASSWORD`). |

## API (docs/00-SYSTEM-CONTEXT.md §10)

| Endpoint | Behaviour |
|---|---|
| `POST /v1/activate {code, school_name, machine_id, app_version, recovery_verifier?}` | `200 {licence, signature, relay_secret}`. `404 CODE_NOT_FOUND`. Idempotent for the currently-bound machine; `409 CODE_ALREADY_USED` for another; `403 REVOKED`. |
| `POST /v1/check {licence_id, machine_id}` | `200 {status: active\|revoked\|moved}`. `404 CODE_NOT_FOUND`. |
| `POST /v1/transfer {licence_id, new_machine_id, recovery_proof, timestamp}` | `200 {licence, signature, relay_secret}`; `server_epoch + 1`, old machine → `moved`. Idempotent (no proof) when already bound to `new_machine_id`. `403 PROOF_INVALID` / `403 TRANSFER_UNAVAILABLE`. |
| `GET /healthz` | `200 ok` (DB reachable) / `503`. |

Errors are generic (`{ "error": CODE }`) and never reveal whether other schools
exist. Rate-limited per IP and per code/licence.

## Configuration + secrets

See `.env.example` for the complete list. Required in `serve`:
`LICENCE_SIGNING_KEY`, `LICENCE_CODE_ENC_KEY`, `RELAY_SHARED_KEY`,
`LICENCE_ADMIN_EMAIL`, `LICENCE_ADMIN_PASSWORD`. Store lives at
`$LICENCE_DATA_DIR/vidya-licence.db` (the Fly volume). `.dev-keys/` (dev keypair +
dev DB) is gitignored.

## Test

```sh
cargo test          # unit + domain + http + no-key-leak (25 tests)
cargo clippy --all-targets -- -D warnings
```
