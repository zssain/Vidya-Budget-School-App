# licence-maker — Vidya offline licences

**Owner's laptop only. Never shipped, never built by the app.** This tool is its
own Cargo workspace and is listed under the root workspace's `exclude`, so
`cargo build`/`cargo test` of Vidya never touch it.

It mints the ed25519 keypair that signs licences, issues machine-bound **licence
keys** and `.vlic` files, records every issue/transfer in `register.csv`, and
verifies keys. Licences are perpetual: no expiry, no online check, no remote
revoke (docs/00-SYSTEM-CONTEXT.md §10).

## Build

```
cd tools/licence-maker
cargo build --release        # binary at target/release/licence-maker
```

## The key folder

Everything lives in one folder you choose with `--dir` (or `$VIDYA_LICENCE_DIR`,
else the current directory):

| File | What | Secrecy |
|---|---|---|
| `licence-signing.key` | 32-byte ed25519 secret seed (base64) | **SECRET** — anyone with this can mint licences. Never commit, never email. |
| `public.key` | 32-byte public key (base64) | Public. Paste into the app build config. |
| `register.csv` | Every licence issued/transferred (has buyer emails) | Private (contains personal data). |

Keep the key folder **outside** the git repo. The tool sets `licence-signing.key`
to `0600` on macOS/Linux.

## Commands

```
licence-maker init     --dir <path>
licence-maker issue    --school "<name>" --machine <CODE> --utr <ref> --email <addr> [--notes "<t>"] [--force]
licence-maker transfer --licence <id> --machine <CODE> [--force]
licence-maker list
licence-maker verify   <licence-key-or-.vlic-path> [--machine <CODE>]
```

- **`init`** — creates the keypair once. Refuses to overwrite an existing key.
  Prints the public key to paste into `src-tauri/build-config/release.json`
  (`"licence_public_key"`).
- **`issue`** — the machine code comes from the buyer (shown on their Welcome →
  Set up screen, e.g. `7KQ2-M9XD-4TRA-P`). The tool validates its checksum,
  refuses a duplicate `--utr` (warns and asks for `--force`), writes the licence
  key + `.vlic`, and appends an `issue` row.
- **`transfer`** — moving a school to a new PC: pass the existing `--licence` id
  and the new PC's `--machine` code. Keeps the same `licence_id`, appends a
  `transfer` row (with the previous machine), and the app bumps `server_epoch`
  so the old PC fences itself when it next reads Drive.
- **`list`** — prints the register.
- **`verify`** — checks a key/`.vlic` signs against this folder's public key and
  prints its contents; `--machine` also checks the binding.

See `docs/SELLING.md` for the end-to-end selling checklist.

## Back up the key folder AND the register in TWO places

If you lose `licence-signing.key` you can no longer issue or transfer licences for
existing customers with the shipped public key. If you lose `register.csv` you
lose the record of who bought what and which UTRs are used.

1. Keep the whole folder on the laptop.
2. Copy it to an **encrypted** USB stick or a private cloud drive (e.g. the
   Principal-style private Google account, in an encrypted archive).
3. Store the passphrase separately.

Re-check both copies after every `issue`/`transfer`.

## If the laptop (and the key) is lost

The signing key cannot be recovered — but **existing customers are never locked
out**, because the app keeps *both* public keys:

1. `licence-maker init` on a new laptop → a **new** keypair.
2. Add the new `public.key` to the app build config **alongside** the old one
   (the app verifies against every configured public key). Ship it in the next
   release.
3. New licences are signed with the new key; all previously issued licences stay
   valid because the app still trusts the old public key.
4. Restore `register.csv` from backup so duplicate-UTR checks and the customer
   record continue.

Never remove an old public key from the app while any customer still relies on a
licence signed by it.
