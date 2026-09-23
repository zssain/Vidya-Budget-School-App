# cloud/licence — Vidya dev licence service

A small, **separate** Rust binary (its own workspace) that is **never** built into
or shipped with the Vidya apps. In this phase it runs in `--dev` mode only;
Phase 10 turns it into the production licence service (purchase webhook, licence
register, activation API, admin panel).

## Run (from this directory)

```sh
cd cloud/licence

# 1. Start the dev service (127.0.0.1:8787). On first run it generates a dev
#    ed25519 keypair into .dev-keys/ and prints the public key.
cargo run -- --dev

# 2. In another terminal, mint an activation code (VIDYA-XXXX-XXXX-XXXX):
cargo run -- gen-code

# Print the public key again (to paste into src-tauri/build-config/dev.json):
cargo run -- print-key
```

Paste the printed `licence_public_key` into
`src-tauri/build-config/dev.json` so the app can verify signatures offline.

## API (docs/00-SYSTEM-CONTEXT.md §10)

| Endpoint | Behaviour |
|---|---|
| `POST /v1/activate {code, school_name, machine_id, app_version}` | `200 {licence, signature}`. `404 CODE_NOT_FOUND`. Idempotent for the same `machine_id`; `409 CODE_ALREADY_USED` for another machine. |
| `POST /v1/check {licence_id, machine_id}` | `200 {status: active\|revoked\|moved}`. |
| `POST /v1/transfer …` | `501 NOT_IMPLEMENTED` until Phase 8 (licence transfer / restore). |

`.dev-keys/` (keypair + `store.json` code store) is gitignored.
