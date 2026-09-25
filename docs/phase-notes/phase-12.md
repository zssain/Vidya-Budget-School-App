# Phase 12 handoff — zero-cost platform: offline licences, static site, Drive scaffolding

## Start state / environment
- Branch `v2/p12`, cut from `v2/p11` @ `168c43f`. Tools: rustc/cargo/clippy **1.98.1**,
  node **v25.3.0**. Repo conventions unchanged (no `AGENTS.md`, no `docs/PROGRESS.md`;
  progress lives here; prompts in `prompts/`).
- `git status` at start: clean. All work committed on `v2/p12` (no push/merge/tag).

## Scope actually built (owner decision: "offline core + Drive scaffolding")
Phase 12 is front-loaded by **Step 0**, a live Google spike that is a STOP condition
and needs the owner's real accounts + three devices. The real Drive OAuth client was
never built (deliberately gated behind that spike since Phase 6). With the owner's
direction, this phase delivered **everything that is verifiable offline** and stopped
before the live-Drive/OAuth UI wiring:

- **Built + tested offline:** licence v2 (Step 6), `licence-maker` (Step 7), retire
  the licence server (Step 8), static `site/` (Step 9), `SELLING.md` (Step 10),
  `drive_account` migration + model (Step 1), `epoch.json` sign/verify/fence crypto
  (Step 5), and the fake-shared-account + upgrade tests (Step 11).
- **Deferred to the spike-gated follow-up (documented, not built):** the Step 0 live
  spike itself; the real `drive.file` client; Settings → Google Drive UI (Step 2);
  the join "sign this phone into the sync account" step (Step 3); the LAN→Drive→queue
  route + 30 s/20 s import/pull timings on the live loop (Step 4); writing/reading
  `epoch.json` on the live import loop; and the prototype-faithful **activate** screen
  redesign. DONE-MEANS #1–3 and #5 are real-hardware acceptance tests (not runnable
  here); #4 (v1 upgrade, no prompt) and #6 (release needs no LICENCE_API/RELAY_URL)
  are met and tested; #7 (harness scenarios) met for the offline subset.

## Spike result (Step 0)
**Not run** — see `docs/phase-notes/phase-12-spike.md` for the full runbook (exact
Drive REST v3 calls, `drive.file` cross-device pass/fail, Android sign-in A/B, timings,
and STOP fallbacks). The MCP Google Drive tools available in tooling are bound to a
different account via a different OAuth client and cannot stand in for it. The intended
behaviour it must confirm is pinned by the offline harness (see Tests).

## Licence v2 format + maker usage (Steps 6, 7)
- **Machine code** `machine_code(machine_id)` = `SHA-256(machine_id)` → top 60 bits →
  12 Crockford base32 chars + a 1-char checksum (`CROCKFORD[(Σ indices) mod 32]`),
  shown `XXXX-XXXX-XXXX-C`. `parse_machine_code` is lenient (case, dashes/spaces,
  `I/L→1`, `O→0`) and checksum-validated. Derived from the stable keychain
  `machine_id`, so it survives app updates.
- **Licence v2** payload `LicenceV2 {v:2, licence_id, school_name, machine_code,
  issued_at, plan:"perpetual", modules:["core"]}`, ed25519-signed. Delivered as a
  **licence key** = `base64url(payload ‖ 64-byte sig)` (shown grouped by 5) or a
  `.vlic` file (the same key text). `parse_licence_key` strips whitespace, splits the
  trailing 64 bytes, `verify_v2` verifies the signature over the raw payload then
  checks `machine_code` → `LICENCE_INVALID` / `LICENCE_OTHER_MACHINE`.
- **v1 licences** still verify via `vidya_core::licence::verify` and are now perpetual
  — the 30-day online recheck and grace logic are removed.
- **`tools/licence-maker/`** (own `[workspace]`, in root `exclude`, never shipped):
  `init --dir` / `issue --school --machine --utr --email [--notes] [--force]` /
  `transfer --licence --machine` / `list` / `verify <key|.vlic> [--machine]`.
  Validates machine-code checksums, refuses duplicate UTRs (unless `--force`), appends
  to `register.csv`. README covers key backup in two places + lost-laptop rotation
  (the app trusts multiple public keys). A **golden vector** is pinned in BOTH the
  maker (`golden_vector_is_stable`) and vidya-core
  (`golden_licence_key_from_maker_verifies`) so the two crates can never silently
  diverge on the format.
- **Build config:** `licence_public_keys: []` (array — enables rotation) replaces the
  single key; **`licence_api` removed**. `build.rs` requires ≥1 key, none the dev key;
  `write-release-config.sh` emits the array (+ optional `LICENCE_PUBLIC_KEY_PREV`).
- **App:** `activate_licence(licenceKey)` verifies offline + persists a pending v2
  licence; new `machine_code` command for the Set-up screen; the startup online
  recheck task is gone. Frontend wiring updated (`api.ts`, `App.tsx`, i18n
  `licence.other_machine`); the **activate screen's visual redesign is deferred** (it
  is the pixel-exact mock Welcome contract — needs owner fidelity approval + baseline
  regen, like the P11 attendance change).

## Accounts migration (Step 1)
- **Migration `0007_v2_drive_accounts.sql`** (additive): `drive_account(kind PK CHECK
  IN ('sync','backup'), email, token_enc, root/exchange/notes/backups folder ids,
  connected_at, status)` + `school.server_key_pub` / `server_key_priv_enc` (Step 5).
- **`src-tauri/src/drive_account.rs`**: `get / is_connected / connect / disconnect /
  needs_sync_account`. The Principal dashboard now carries `needs_sync_account` →
  Home "Needs attention: Connect the school sync account" until a sync account is set.
- v1 had ONE Principal Drive connection but **never persisted OAuth tokens in a table**
  (the real client was spike-gated), so there is no legacy connection row to copy; the
  migration just adds the table. When the live client lands, migrate any legacy KV
  connection to `kind='backup'`.
- **Per-staff folder sharing:** the only per-staff sharing that exists is P06 FakeDrive
  test scaffolding (`provision_school` grants + `share`/`unshare`) — there is no
  production sharing code. It is left intact so P06 tests keep passing (rule 11); the
  v2 model is added additively as `provision_shared_account` (one identity, full
  access). Removing the old per-staff harness belongs with the live two-account remodel.

## Epoch design (Step 5)
- `vidya_core::epoch`: `EpochFile {school_id, server_epoch, server_machine_code, at}`,
  `SignedEpoch {payload(base64url), sig(base64url)}`, `sign_epoch` (ed25519,
  deterministic) / `verify_epoch` → `EpochFile` or `EPOCH_INVALID`, and
  `fence_outcome(own, file)` → `Current | Fenced | Stale`. `check_epoch` (devices
  refuse a lower epoch) is unchanged.
- The school server key pair is stored by migration 0007 (private seed in the
  SQLCipher-encrypted DB; public key shipped to devices at join — wiring lands with
  the live Drive client). On a transfer, `licence-maker transfer` + `server_epoch + 1`
  → the new PC writes `epoch.json` → the old PC reads it and is fenced.

## What was archived (Step 8)
- `git mv cloud/licence → cloud/_archive/licence` (+ `cloud/_archive/README.md`
  "Retired in v2"). Root workspace members stay `[vidya, vidya-core]`; exclude updated.
- `cloud/relay/README.md` now headed "Optional add-on (Instant sync) — not needed for
  normal use". `release.yml`: `LICENCE_API` dropped from all 3 jobs; a new step writes
  `site/releases.json` to the default branch via `scripts/write-site-releases.mjs`.
  `check-logs.mjs` scans the moved path.

## Site pages (Step 9)
`site/` (plain HTML/CSS, brand tokens, Welcome look): `index` (what Vidya is, 3
platforms, honest size text, `[PRICE]`), `how-to-buy` (UPI QR placeholder +
prefilled `mailto:`), `downloads` (vanilla JS from `releases.json`; unsigned shown
honestly), `privacy`/`terms`/`refund` (drafts marked "Draft — owner review"). Footer
"Developed by Zuhair Hussain · © 2026". `docs/DEPLOY-SITE.md`: GitHub/Cloudflare Pages
+ DNS for `vidya.zuhairhussain.com`. Not deployed.

## Tests (Step 11) — all run, real output
- `cargo test --workspace --locked` → **0 failed** (src-tauri lib 174, vidya-core lib
  302, integration suites green).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo tree -i aws-lc-rs` → empty (ring only).
- `tools/licence-maker`: `cargo test` 5 passed, clippy clean; CLI smoke-tested
  (issue/dup-UTR/verify/transfer, CSV escaping, lenient machine-code entry).
- `npm run verify` → green (typecheck, check-hex, **check-i18n 815 keys en/hi in
  sync**, check-deps 30 npm + 34 cargo within §13, version, contrast, logs, vitest 38).
  `npm run check:release-clean` → OK.
- New tests of note: licence v2 (machine code/checksum, key round-trip, verify_v2
  good/wrong-machine/tamper/version, cross-crate golden vector); offline app verify
  (this-machine / other-machine / rotation); `drive_account` CRUD + CHECK + needs-sync;
  epoch sign/verify/tamper/fence + transfer flow; FakeDrive shared-account cross-device
  read/write + epoch.json-through-Drive fence; **v1→v2 upgrade** (v1 DB + v1 licence →
  opens to PIN, no prompt, Home asks to connect sync).

### Changed test expectations (with reasons)
- `error.rs::licence_error_maps`: the online `LicenceError` variants
  (`Unreachable/CodeNotFound/CodeAlreadyUsed`) were removed with online activation;
  the test now covers `Invalid` / `OtherMachine`.
- `config.rs` tests: assert `licence_public_keys` (was single `licence_public_key`),
  and there is no `licence_api`.
- Removed with online activation (behaviour change per §2/§10): `licence::mod`
  `parse_activate/activate/check/should_recheck/persist_check` and their tests. Offline
  `verify_offline` tests replace them.

## Owner decisions touched (see `docs/OWNER-DECISIONS.md`)
- **#2 Price** and **#11 UPI QR image** now surface on the static `site/` (`[PRICE]`
  and `site/assets/upi-qr.png` placeholders) and in `SELLING.md`. **#14 code signing**
  and **#15 GST** appear on the Downloads/Refund pages as honest drafts. New build-time
  value: `licence_public_keys` (owner mints it with `licence-maker init`).

## What Phase 13 needs / immediate follow-ups
1. **Run the Step 0 spike** (`phase-12-spike.md`) on real accounts + 3 devices — the
   gate for the live Drive client and Steps 2–4 UI.
2. After PASS: build the `drive.file` client behind `DriveApi`; Settings → Google
   Drive; join sync sign-in; live route/timings; live `epoch.json` write/read; remove
   the P06 per-staff FakeDrive sharing and remodel to two accounts.
3. Redesign the **activate** screen to the prototype (machine code + key + `.vlic`
   load, groups of 5) with owner fidelity approval + baseline regen.
4. Provision the relay secret for the optional `instant_sync` module some way other
   than activation (it is no longer issued at activation).
5. P13 is foundation/Telugu/privacy — unaffected by the deferred Drive wiring.

## Security note
While archiving, a `git mv` briefly staged `cloud/licence/.dev-keys/` (dev signing
keys) because the old gitignore path stopped matching. Caught before push: untracked
with `git rm --cached`, `.gitignore` now has `**/.dev-keys/`, and the commit was
amended. No keys were pushed. Lesson: re-check `.gitignore` coverage after moving a
directory that holds ignored secrets.
