# Licence signer (owner portal, spec Phase 13)

`services/licence-signer/` is the **only** online component that holds the
licence signing key. It is a small queue worker, not a web service: it has **no
HTTP server and no listening socket**, so there is no "sign arbitrary bytes"
endpoint to abuse. It reads `issue_licence` jobs from the billing Postgres
(`infra/licensing/migrations/0001_billing.sql`, schema `billing`), signs the v2
licence payload, verifies the result with the app's own verifier, and writes the
artefact back.

It is its own Cargo workspace (own `Cargo.lock`) and is listed in the root
`Cargo.toml` `exclude`, like `tools/licence-maker`; it is never part of the app
build. It depends on `crates/vidya-core` by path and reuses its `LicenceV2` type,
`encode_licence_key`, `parse_licence_key`, `parse_machine_code`, `verify_v2` and
the module list (`vidya_core::modules::TOGGLEABLE_KEYS`). It does not have its
own copy of the verifier.

## What one job does

The billing worker, after a verified capture (or an audited owner approval),
inserts:

- one `entitlements` row (modules, machine code, school name, `commercial_status='active'`);
- one `licence_issuances` row with `status='pending'` and `entitlement_id`,
  `generation` (1, or n+1 for a transfer), `licence_id` (worker-chosen, e.g.
  `VID-<uuid>`), `machine_code` and `issued_at` (an RFC 3339 string) already set;
- one `jobs` row: `kind='issue_licence'`, `dedupe_key='issue:<issuance_id>'`,
  `payload={"issuance_id":"<uuid>"}`.

The signer then runs three short transactions per job:

1. **Claim.** `SELECT … FOR UPDATE SKIP LOCKED` one `ready` `issue_licence` job
   whose `available_at <= now()`. Set `state='running'`, `attempts=attempts+1`,
   `locked_by=<worker id>`, `lease_until=now()+60s`. Commit. Attempts are
   counted when the job is claimed, so a job that crashes the signer every time
   still runs out of attempts instead of looping forever.
2. **Prepare** (read-only). Load the issuance and its entitlement and check them
   again:
   - the issuance is `pending` (if it is already `issued`, the job just closes);
   - `entitlements.commercial_status = 'active'`, so refunded, disputed or
     superseded entitlements are never signed;
   - the machine code passes `vidya_core::licence::parse_machine_code`
     (Crockford checksum) and is converted to the dashed upper-case form
     `XXXX-XXXX-XXXX-C`;
   - every module is in `core` + `TOGGLEABLE_KEYS` and `core` is present.
     Duplicates are dropped and the list is put in canonical order, so `core`
     always comes first and a retry always gives the same bytes;
   - `licence_id` is 1–128 printable ASCII characters, `school_name` is 1–200
     characters with no control characters, and `issued_at` parses as RFC 3339.

   Next, build the payload, sign it, and **verify** it: parse the key back and
   run `vidya_core::licence::verify_v2` with the signer's own public key and the
   licence's machine code. If verification fails, nothing is written.
3. **Commit.** Lock the job row and check that this worker still owns it
   (`state='running'` and `locked_by` = this worker). If not, it is a zombie
   after its lease expired, and it writes nothing. Then lock the issuance row
   (`FOR UPDATE`), re-check that it is still `pending` and that the entitlement
   is still `active`, and write `payload`, `signature`, `licence_key`, `vlic`,
   `status='issued'`, `issued_ts=now()` and `signer_key_id`. Mark the job `done`
   and append an `owner_audit` row (`actor='signer'`, `action='licence_issued'`,
   `target=<issuance id>`, `after_json` = licence id, machine code, modules, key
   id, key fingerprint and job id). If the issuance is already `issued`, the
   stored bytes must match the new artefact exactly. In that case the job is
   closed with no second audit row. If they differ, it is a permanent error.

The billing worker stores `licence_id` and `issued_at` before the job exists, and
ed25519 signing is deterministic (RFC 8032). So **a retry always produces exactly
the same bytes**. The `status='pending'` guard runs under a row lock, so **only
one artefact is ever written per issuance**, even if several signers or several
jobs for the same issuance race each other.

### Failures, retries, leases

| Failure | Effect |
|---|---|
| Permanent (entitlement not active, bad machine code, unknown module or no `core`, malformed fields, issuance missing, issuance not `pending`, a stored artefact that differs) | Job → `dead` right away. A still-`pending` issuance → `failed` with `last_error`. Audit row `licence_issue_failed`. |
| Transient (database errors) | Job → `ready`, `available_at = now() + min(30s·2^(attempt-1), 1h)`, `last_error` set. Once `attempts >= max_attempts` it goes to `dead`, and the issuance → `failed`. |
| Signer crash or stall | At the start of every poll, any `running` job with `lease_until < now()` goes back to `ready`, or to `dead` (issuance → `failed`) if all its attempts are used. |

`last_error` holds operational text only, at most 500 characters. It never
contains key material, and a database error is reduced to the server message and
SQLSTATE.

The issuance moves to `failed` only when its job is **dead**. A `failed` issuance
cannot be set back to `pending` by any role (the portal can only `INSERT`
issuances). To re-issue after fixing the data (for example, after a dispute is
resolved), the portal or worker creates a new generation.

### Database privileges

The signer runs as a login role that is a member of `vidya_signer`, for example
`CREATE ROLE licence_signer LOGIN PASSWORD '…' IN ROLE vidya_signer;`. Every
statement uses schema-qualified names (`billing.*`) and writes only the columns
that `vidya_signer` is granted: issuance `payload/signature/licence_key/vlic/status/last_error/issued_ts/signer_key_id`,
job `state/attempts/available_at/locked_by/lease_until/last_error/completed_at`,
plus `INSERT` into `owner_audit`. It never locks `entitlements` (it has no
`UPDATE` on that table, so it cannot use `FOR SHARE`). Instead it re-reads
`commercial_status` inside the commit transaction. A refund committed after that
read can still race a commit by a few milliseconds. The licence is perpetual and
offline either way, so any refund handling happens on the commercial side. The
tests run every signer statement as `vidya_signer` (`SET ROLE`), and they check
that this role **cannot** insert entitlements, change an issuance's machine code,
or update entitlements. **No schema change was needed**: `0002_signer.sql` was
not added.

## Byte compatibility with `tools/licence-maker`

| Item | Format |
|---|---|
| payload | `serde_json::to_vec(LicenceV2 { v: 2, licence_id, school_name, machine_code, issued_at, plan: "perpetual", modules })`, compact JSON in this field order |
| machine code in payload | canonical `XXXX-XXXX-XXXX-C`, upper-case, dashed |
| signature | ed25519 over the exact payload bytes (64 bytes) |
| `vlic` column / `.vlic` file | `base64url_nopad(payload ‖ signature)`, no trailing newline (licence-maker writes exactly this) |
| `licence_key` column | the same string in groups of 5, joined with single spaces (licence-maker's `group5`, the form it prints) |

`issue` in licence-maker always uses `modules = ["core"]`. The signer uses the
entitlement's modules in canonical order. When the modules are `["core"]`, the
output is identical to licence-maker's.

Two tests pin this:

- `tests/golden.rs::golden_vector_matches_licence_maker_and_verifies` checks the
  same golden key that licence-maker's `golden_vector_is_stable` and vidya-core's
  `golden_licence_key_from_maker_verifies` pin (test seed `[42;32]`), and checks
  that `verify_v2` accepts it.
- `tests/golden.rs::real_licence_maker_output_is_reproduced_byte_for_byte`
  builds and runs the real `licence-maker` binary (`init` + `issue`) in a temp
  folder. It loads that folder's `licence-signing.key` and `public.key` with the
  signer's loaders, signs the same fields (with a Unicode, quoted, comma-containing
  school name and a machine code typed in lower case with spaces), and compares
  the payload, the `.vlic` contents and the printed grouped key byte for byte.

## Configuration

| Variable | Required | Meaning |
|---|---|---|
| `LICENCE_SIGNING_KEY_FILE` | yes | Path to the key file (format below). The key is **only** read from a file. |
| `LICENCE_SIGNING_KEY_ID` | yes | 1–64 chars `[A-Za-z0-9._-]`, stored in `licence_issuances.signer_key_id`. |
| `EXPECTED_PUBLIC_KEY_FINGERPRINT` | yes | `sha256:<64 hex>`. A comma-separated list is an allow-list. |
| `SIGNER_DATABASE_URL` | yes | Postgres URL for a `vidya_signer` member. Use `sslmode=verify-full`. |
| `SIGNER_POLL_INTERVAL_SECS` | no (5) | Idle poll interval, 1–3600. |
| `SIGNER_WORKER_ID` | no | Value for `locked_by`. Default: `signer-<pid>-<random>`. |

See `services/licence-signer/.env.example` (placeholders only).

Startup refuses to run if any of these hold:
- `LICENCE_SIGNING_KEY`, `LICENCE_SIGNING_KEY_B64` or `LICENCE_SIGNING_KEY_BASE64`
  is set (key material must not be passed in env vars);
- the key file is missing, larger than 4 KiB, not 32 bytes after base64 decoding,
  or (on Unix) group- or other-accessible (`mode & 077 != 0`). Use `chmod 600`
  or `400`. A container secret mount must set its mode explicitly, for example
  Kubernetes `defaultMode: 0400`;
- the public key's fingerprint is not in `EXPECTED_PUBLIC_KEY_FINGERPRINT`.

These checks all run **before** the process connects to the database. Once they
pass, the logs show only the key id and the fingerprint. `Signer` and `Config`
have redacted `Debug` output, and a database connection failure is logged
without the URL. The seed buffers are zeroised after loading, and
ed25519-dalek's `SigningKey` zeroises on drop.

### Key file format

This is exactly what `licence-maker init` writes to `licence-signing.key`: the
**32-byte ed25519 seed encoded as standard base64 with padding** (44 characters).
A trailing newline or surrounding whitespace is allowed. `public.key` is the
standard base64 of the 32 raw public-key bytes. Each entry of `licence_public_keys`
in `src-tauri/build-config/release.json` holds the same value.

### Fingerprint

`sha256:` + lower-case hex of SHA-256 over the **32 raw public-key bytes**:

```sh
licence-signer fingerprint /path/to/public.key       # from the public key file (no secret needed)
LICENCE_SIGNING_KEY_FILE=… licence-signer fingerprint  # derived from the key file
# or: base64 -d < public.key | shasum -a 256
```

The fingerprint check ensures **every artefact the signer writes can be verified
by released apps**, as long as `EXPECTED_PUBLIC_KEY_FINGERPRINT` holds only keys
whose public halves ship in `licence_public_keys`. Set the variable from the
release config. Do not copy it from the key file you are about to use, or the
check proves nothing.

## Key rotation

1. Generate the new key offline (`licence-maker init --dir <new folder>`).
2. Add the new `public.key` value to `licence_public_keys` in the app's release
   config **next to** the old one. Ship that release and wait until the installs
   that matter have it. **Never sign with a key before a release that trusts it
   has shipped**, or customers on older builds cannot activate.
3. Point the signer at the new key file. Set `LICENCE_SIGNING_KEY_ID` to a new
   id and `EXPECTED_PUBLIC_KEY_FINGERPRINT` to the new fingerprint.
4. Keep the old public key in `licence_public_keys` **forever**. Licences already
   issued are perpetual and offline, and they keep verifying against it.
   `signer_key_id` in each issuance row records which key signed it.

## Backup and recovery of the signing key

- The key file is the one secret that cannot be replaced. Keep **two offline,
  encrypted copies** (for example an encrypted USB stick and an encrypted archive
  in private cloud storage, with the passphrase stored separately), as
  `tools/licence-maker/README.md` describes. The portal server holds a third,
  working copy.
- The key **must not** be in the repo, in images, in env vars, in CI logs or in
  database backups. Mount it at runtime as a read-only file readable only by the
  signer user.
- **Lost key:** issued licences keep working, because the apps hold only the
  public key. Rotate (above) before issuing again. Until a release that trusts
  the new key has shipped, new sales can only be fulfilled by hand on installs
  that already trust it.
- **Leaked key:** anyone holding it can mint licences that verify on every build
  that trusts it, and they cannot be revoked remotely. Rotate right away, remove
  the leaked public key from **new** releases, and accept that existing installs
  of older releases still trust it.

## Transfer semantics

A transfer is a new issuance with `generation = n+1` and the new machine code. The
billing worker creates it (and the `licence_transfers` row), and the signer signs
it like any other issuance. The signer **never modifies or revokes** an earlier
generation. The old licence is perpetual, offline and bound to the old PC, so
**the old PC keeps working**. There is no remote revocation (by design, see
`vidya_core::licence`). licence-maker notes that the app may fence an old PC
through the Drive `server_epoch` when it next reads Drive. That is app-side
behaviour, not something this service does.

Difference from licence-maker: `licence-maker transfer` reuses the **same**
`licence_id`, but `billing.licence_issuances.licence_id` is `UNIQUE`, so each
portal generation must carry a **new** `licence_id`. That is the billing worker's
choice. If the app ever keys behaviour on `licence_id`, someone needs to review
this.

## Running it

```sh
cd services/licence-signer
cargo build --release
LICENCE_SIGNING_KEY_FILE=/run/secrets/licence-signing.key \
LICENCE_SIGNING_KEY_ID=owner-2026-a \
EXPECTED_PUBLIC_KEY_FINGERPRINT=sha256:… \
SIGNER_DATABASE_URL='postgres://licence_signer:…@db:5432/billing?sslmode=verify-full' \
./target/release/licence-signer
```

It polls every `SIGNER_POLL_INTERVAL_SECS`. On each pass it recovers expired
leases and then works through the ready jobs. SIGTERM or SIGINT stops it
between jobs: the job in hand finishes first. If it is killed hard instead, lease
recovery covers it. Several instances can run against the same database. Run the process with
no inbound network access and egress only to Postgres.

Tests need a local Postgres. The default is
`postgres://postgres@localhost:55432/postgres`; override it with
`SIGNER_TEST_ADMIN_URL`. Each test creates a throw-away database
`signer_test_<uuid>`, applies the real `0001_billing.sql`, and drops the database
afterwards (a panicking test leaves it behind).

```sh
cd services/licence-signer && cargo test && cargo clippy --all-targets -- -D warnings
```

## What was and was not verified

Verified by automated tests against real Postgres 16 on localhost (`cargo test`:
32 tests: 10 unit, 12 DB, 1 binary end-to-end, 4 golden/compat, 5 binary
startup). The DB and e2e suites passed 5 repeated runs with no flakes:
- the real binary, end to end: it loads a key file, connects with
  `role=vidya_signer`, signs a seeded job (`verify_v2` accepts the stored
  artefact), exits 0 on SIGTERM, and prints no key material in its logs;
- the golden vector, plus byte-for-byte equality with the **real**
  licence-maker binary, and `verify_v2` accepting the output;
- claim → sign → complete, with every write made as `vidya_signer`;
- two concurrent signer processes (separate pools) over 13 issuances, one of
  which has two jobs: exactly one artefact and one `licence_issued` audit row
  per issuance;
- a crash between signing and commit, followed by lease expiry and recovery:
  the retry writes an identical artefact, and the zombie's late commit is
  rejected;
- refunded or disputed entitlements, a bad machine-code checksum, an unknown
  module, missing `core`, and a refund between sign and commit: none are signed;
  the job goes `dead` and the issuance `failed`;
- transient-failure backoff, then `dead` after `max_attempts`;
- lease recovery, both back to `ready` and to `dead`;
- a transfer generation is signed and the old generation is unchanged;
- startup refusal for a wrong fingerprint, a group-readable key file and an
  inline key in the environment, with no key material in the output.

Not verified:
- the end-to-end contract with the real billing worker, which was being built
  in parallel. The tests insert rows the way the contract above describes;
- TLS to a managed Postgres (only plaintext localhost was exercised; the build
  uses sqlx `tls-rustls-ring`, which has no native-tls or OpenSSL dependency);
- real deployment: no container image, secret mounting or supervisor setup is
  provided or tested;
- Windows (the key-file permission check and the startup and e2e tests are
  Unix-only);
- loading a licence signed by this service in the actual desktop app UI.
  Equivalence rests on `verify_v2`, which is the app's verification function.
